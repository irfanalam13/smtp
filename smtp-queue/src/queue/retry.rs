use std::time::Duration;

use rand::Rng;

use crate::QueueConfig;

pub fn delay(config: &QueueConfig, attempt: u32, rng: &mut impl Rng) -> Duration {
    let exponent = attempt.saturating_sub(1).min(31);
    let multiplier = 1u128 << exponent;
    let base = config
        .initial_retry_delay
        .as_millis()
        .saturating_mul(multiplier);
    let capped = base.min(config.max_retry_delay.as_millis());
    let jitter = if config.jitter.is_zero() {
        0
    } else {
        rng.random_range(0..=config.jitter.as_millis())
    };
    Duration::from_millis(capped.saturating_add(jitter).min(u64::MAX as u128) as u64)
}

pub fn should_retry(category: &crate::FailureCategory, attempt: u32, max_attempts: u32) -> bool {
    matches!(
        category,
        crate::FailureCategory::Temporary | crate::FailureCategory::Internal
    ) && attempt < max_attempts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FailureCategory;

    #[test]
    fn backoff_is_bounded_and_jittered() {
        let config = QueueConfig {
            initial_retry_delay: Duration::from_millis(10),
            max_retry_delay: Duration::from_millis(30),
            jitter: Duration::from_millis(5),
            ..QueueConfig::default()
        };
        let mut rng = rand::rng();
        let first = delay(&config, 1, &mut rng);
        let later = delay(&config, 8, &mut rng);
        assert!(first >= Duration::from_millis(10) && first <= Duration::from_millis(15));
        assert!(later >= Duration::from_millis(30) && later <= Duration::from_millis(35));
        assert!(!should_retry(&FailureCategory::Permanent, 1, 5));
        assert!(!should_retry(&FailureCategory::Temporary, 5, 5));
    }
}
