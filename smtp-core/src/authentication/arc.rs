use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArcInstance {
    pub instance: u32,
    pub authentication_results: String,
    pub seal: String,
    pub signature: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArcChain {
    pub instances: Vec<ArcInstance>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArcResult {
    Pass,
    Fail,
    None,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ArcError {
    #[error("ARC instance sequence is invalid")]
    InvalidSequence,
    #[error("ARC instance contains an empty required field")]
    EmptyField,
}

pub trait ArcVerifier {
    fn verify_chain(&self, chain: &ArcChain) -> Result<bool, ArcError>;
}

pub fn validate_arc_chain(chain: &ArcChain) -> Result<(), ArcError> {
    if chain.instances.is_empty() {
        return Err(ArcError::InvalidSequence);
    }
    for (index, instance) in chain.instances.iter().enumerate() {
        if instance.instance != (index as u32 + 1)
            || instance.authentication_results.is_empty()
            || instance.seal.is_empty()
            || instance.signature.is_empty()
        {
            return Err(if instance.instance != (index as u32 + 1) {
                ArcError::InvalidSequence
            } else {
                ArcError::EmptyField
            });
        }
    }
    Ok(())
}

pub fn verify_arc_chain(
    chain: &ArcChain,
    verifier: &dyn ArcVerifier,
) -> Result<ArcResult, ArcError> {
    if chain.instances.is_empty() {
        return Ok(ArcResult::None);
    }
    validate_arc_chain(chain)?;
    Ok(if verifier.verify_chain(chain)? {
        ArcResult::Pass
    } else {
        ArcResult::Fail
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_ordered_arc_instances() {
        let chain = ArcChain {
            instances: vec![ArcInstance {
                instance: 1,
                authentication_results: "spf=pass".into(),
                seal: "seal".into(),
                signature: "signature".into(),
            }],
        };
        assert!(validate_arc_chain(&chain).is_ok());
    }
}
