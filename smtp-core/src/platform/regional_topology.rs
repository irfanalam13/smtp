use std::collections::HashSet;

use thiserror::Error;

const MAX_REGIONS: usize = 32;
const MAX_AVAILABILITY_ZONES_PER_REGION: usize = 8;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RegionalService {
    Api,
    Smtp,
    Imap,
    Pop3,
    Web,
    Authentication,
    Storage,
    Database,
    Queue,
    Search,
    Ai,
    Monitoring,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionRole {
    Primary,
    Standby,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionDefinition {
    id: String,
    role: RegionRole,
    availability_zones: Vec<String>,
    services: HashSet<RegionalService>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionalTopology {
    regions: Vec<RegionDefinition>,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum RegionalTopologyError {
    #[error("regional topology must contain at least two regions")]
    TooFewRegions,
    #[error("regional topology exceeds the configured region limit")]
    TooManyRegions,
    #[error("regional topology must have exactly one primary region")]
    InvalidPrimaryCount,
    #[error("regional topology must have at least one standby region")]
    MissingStandby,
    #[error("region identifier is empty, too long, or malformed")]
    InvalidRegionId,
    #[error("region identifier is duplicated")]
    DuplicateRegionId,
    #[error("region availability-zone count is outside supported bounds")]
    InvalidAvailabilityZoneCount,
    #[error("availability-zone identifier is empty, too long, or malformed")]
    InvalidAvailabilityZoneId,
    #[error("availability zone is duplicated in a region")]
    DuplicateAvailabilityZone,
    #[error("region is missing a required service")]
    MissingRequiredService,
}

impl RegionDefinition {
    pub fn new(
        id: String,
        role: RegionRole,
        availability_zones: Vec<String>,
        services: impl IntoIterator<Item = RegionalService>,
    ) -> Self {
        Self {
            id,
            role,
            availability_zones,
            services: services.into_iter().collect(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn role(&self) -> RegionRole {
        self.role
    }

    pub fn availability_zones(&self) -> &[String] {
        &self.availability_zones
    }

    pub fn provides(&self, service: RegionalService) -> bool {
        self.services.contains(&service)
    }
}

impl RegionalTopology {
    pub fn new(regions: Vec<RegionDefinition>) -> Self {
        Self { regions }
    }

    pub fn regions(&self) -> &[RegionDefinition] {
        &self.regions
    }

    pub fn validate(
        &self,
        required_services: &[RegionalService],
    ) -> Result<(), RegionalTopologyError> {
        if self.regions.len() < 2 {
            return Err(RegionalTopologyError::TooFewRegions);
        }
        if self.regions.len() > MAX_REGIONS {
            return Err(RegionalTopologyError::TooManyRegions);
        }

        let mut region_ids = HashSet::with_capacity(self.regions.len());
        let mut primary_count = 0;
        let mut standby_count = 0;
        for region in &self.regions {
            if !valid_identifier(&region.id, 32) {
                return Err(RegionalTopologyError::InvalidRegionId);
            }
            if !region_ids.insert(region.id.as_str()) {
                return Err(RegionalTopologyError::DuplicateRegionId);
            }

            match region.role {
                RegionRole::Primary => primary_count += 1,
                RegionRole::Standby => standby_count += 1,
            }

            if region.availability_zones.is_empty()
                || region.availability_zones.len() > MAX_AVAILABILITY_ZONES_PER_REGION
            {
                return Err(RegionalTopologyError::InvalidAvailabilityZoneCount);
            }
            let mut zones = HashSet::with_capacity(region.availability_zones.len());
            for zone in &region.availability_zones {
                if !valid_identifier(zone, 64) {
                    return Err(RegionalTopologyError::InvalidAvailabilityZoneId);
                }
                if !zones.insert(zone.as_str()) {
                    return Err(RegionalTopologyError::DuplicateAvailabilityZone);
                }
            }
        }

        if primary_count != 1 {
            return Err(RegionalTopologyError::InvalidPrimaryCount);
        }
        if standby_count == 0 {
            return Err(RegionalTopologyError::MissingStandby);
        }
        if required_services.iter().any(|service| {
            self.regions
                .iter()
                .any(|region| !region.services.contains(service))
        }) {
            return Err(RegionalTopologyError::MissingRequiredService);
        }
        Ok(())
    }
}

fn valid_identifier(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_bytes
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(id: &str, role: RegionRole) -> RegionDefinition {
        RegionDefinition::new(
            id.to_owned(),
            role,
            vec![format!("{id}-az1"), format!("{id}-az2")],
            [
                RegionalService::Api,
                RegionalService::Smtp,
                RegionalService::Database,
            ],
        )
    }

    fn valid_topology() -> RegionalTopology {
        RegionalTopology::new(vec![
            region("region-primary", RegionRole::Primary),
            region("region-standby", RegionRole::Standby),
        ])
    }

    #[test]
    fn accepts_primary_and_standby_with_required_service_coverage() {
        let topology = valid_topology();
        assert_eq!(
            topology.validate(&[RegionalService::Api, RegionalService::Database]),
            Ok(())
        );
    }

    #[test]
    fn rejects_single_region_and_too_many_regions() {
        assert_eq!(
            RegionalTopology::new(vec![region("region-one", RegionRole::Primary)]).validate(&[]),
            Err(RegionalTopologyError::TooFewRegions)
        );
        let regions = (0..=MAX_REGIONS)
            .map(|index| {
                region(
                    &format!("region-{index}"),
                    if index == 0 {
                        RegionRole::Primary
                    } else {
                        RegionRole::Standby
                    },
                )
            })
            .collect();
        assert_eq!(
            RegionalTopology::new(regions).validate(&[]),
            Err(RegionalTopologyError::TooManyRegions)
        );
    }

    #[test]
    fn rejects_duplicate_region_ids_and_multiple_primaries() {
        assert_eq!(
            RegionalTopology::new(vec![
                region("region-same", RegionRole::Primary),
                region("region-same", RegionRole::Standby),
            ])
            .validate(&[]),
            Err(RegionalTopologyError::DuplicateRegionId)
        );
        assert_eq!(
            RegionalTopology::new(vec![
                region("region-one", RegionRole::Primary),
                region("region-two", RegionRole::Primary),
            ])
            .validate(&[]),
            Err(RegionalTopologyError::InvalidPrimaryCount)
        );
    }

    #[test]
    fn rejects_standby_only_topology_and_missing_service_coverage() {
        assert_eq!(
            RegionalTopology::new(vec![
                region("region-one", RegionRole::Standby),
                region("region-two", RegionRole::Standby),
            ])
            .validate(&[]),
            Err(RegionalTopologyError::InvalidPrimaryCount)
        );
        let mut standby = region("region-standby", RegionRole::Standby);
        standby.services.remove(&RegionalService::Smtp);
        assert_eq!(
            RegionalTopology::new(vec![region("region-primary", RegionRole::Primary), standby,])
                .validate(&[RegionalService::Smtp]),
            Err(RegionalTopologyError::MissingRequiredService)
        );
    }

    #[test]
    fn rejects_invalid_and_duplicate_availability_zones() {
        let invalid = RegionDefinition::new(
            "region-primary".to_owned(),
            RegionRole::Primary,
            vec!["region-primary-az1".to_owned(), "Bad AZ".to_owned()],
            [],
        );
        assert_eq!(
            RegionalTopology::new(vec![invalid, region("region-standby", RegionRole::Standby),])
                .validate(&[]),
            Err(RegionalTopologyError::InvalidAvailabilityZoneId)
        );
        let duplicate = RegionDefinition::new(
            "region-primary".to_owned(),
            RegionRole::Primary,
            vec![
                "region-primary-az1".to_owned(),
                "region-primary-az1".to_owned(),
            ],
            [],
        );
        assert_eq!(
            RegionalTopology::new(vec![
                duplicate,
                region("region-standby", RegionRole::Standby),
            ])
            .validate(&[]),
            Err(RegionalTopologyError::DuplicateAvailabilityZone)
        );
    }
}
