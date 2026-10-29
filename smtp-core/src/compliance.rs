#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComplianceFixture {
    pub name: String,
    pub input: String,
    pub expected: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComplianceResult {
    pub name: String,
    pub passed: bool,
}

pub fn run_compliance_fixture(
    fixture: &ComplianceFixture,
    evaluator: impl Fn(&str) -> String,
) -> ComplianceResult {
    ComplianceResult {
        name: fixture.name.clone(),
        passed: evaluator(&fixture.input) == fixture.expected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_fixture_mismatches_without_silent_success() {
        let fixture = ComplianceFixture {
            name: "smtp-positive".into(),
            input: "EHLO".into(),
            expected: "250".into(),
        };
        assert!(run_compliance_fixture(&fixture, |_| "250".into()).passed);
        assert!(!run_compliance_fixture(&fixture, |_| "500".into()).passed);
    }
}
