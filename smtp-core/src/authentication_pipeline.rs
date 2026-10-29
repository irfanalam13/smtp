use crate::{ArcResult, BimiResult, DmarcResult, SpfResult};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticationInput {
    pub spf: SpfResult,
    pub dkim_pass: bool,
    pub dmarc: DmarcResult,
    pub arc: ArcResult,
    pub bimi: BimiResult,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthenticationDisposition {
    Pass,
    Fail,
    Quarantine,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticationReport {
    pub spf: SpfResult,
    pub dkim_pass: bool,
    pub dmarc: DmarcResult,
    pub arc: ArcResult,
    pub bimi: BimiResult,
    pub disposition: AuthenticationDisposition,
}

pub fn authenticate(input: AuthenticationInput) -> AuthenticationReport {
    let disposition = match input.dmarc {
        DmarcResult::Pass => AuthenticationDisposition::Pass,
        DmarcResult::Fail(crate::DmarcDisposition::Reject) => AuthenticationDisposition::Fail,
        DmarcResult::Fail(_) => AuthenticationDisposition::Quarantine,
    };
    AuthenticationReport {
        spf: input.spf,
        dkim_pass: input.dkim_pass,
        dmarc: input.dmarc,
        arc: input.arc,
        bimi: input.bimi,
        disposition,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DmarcDisposition, DmarcResult};

    #[test]
    fn combines_dmarc_and_arc_without_treating_bimi_as_authentication() {
        let report = authenticate(AuthenticationInput {
            spf: SpfResult::Fail,
            dkim_pass: false,
            dmarc: DmarcResult::Fail(DmarcDisposition::Quarantine),
            arc: ArcResult::None,
            bimi: BimiResult::Pass,
        });
        assert_eq!(report.disposition, AuthenticationDisposition::Quarantine);
    }

    #[test]
    fn arc_does_not_override_local_dmarc_policy_automatically() {
        let report = authenticate(AuthenticationInput {
            spf: SpfResult::Fail,
            dkim_pass: false,
            dmarc: DmarcResult::Fail(DmarcDisposition::Reject),
            arc: ArcResult::Pass,
            bimi: BimiResult::None,
        });
        assert_eq!(report.disposition, AuthenticationDisposition::Fail);
    }
}
