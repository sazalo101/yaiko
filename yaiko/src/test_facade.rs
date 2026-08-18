//! Deterministic test harness metadata and reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestOutcome {
    Passed,
    Failed,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestError {
    InvalidName,
    InvalidMessage,
    Capacity,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestReport {
    pub name: String,
    pub outcome: TestOutcome,
    pub message: String,
}
#[derive(Debug, Clone, Default)]
pub struct TestHarness {
    cases: Vec<TestReport>,
    capacity: usize,
}
impl TestHarness {
    pub fn new(capacity: usize) -> Self {
        Self {
            cases: Vec::new(),
            capacity,
        }
    }
    pub fn add(
        mut self,
        name: impl Into<String>,
        outcome: TestOutcome,
        message: impl Into<String>,
    ) -> Result<Self, TestError> {
        let name = name.into();
        let message = message.into();
        if name.is_empty()
            || name.len() > 128
            || name.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(TestError::InvalidName);
        }
        if message.len() > 512 {
            return Err(TestError::InvalidMessage);
        }
        if self.cases.len() >= self.capacity {
            return Err(TestError::Capacity);
        }
        self.cases.push(TestReport {
            name,
            outcome,
            message,
        });
        self.cases.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(self)
    }
    pub fn passed(&self) -> usize {
        self.cases
            .iter()
            .filter(|x| x.outcome == TestOutcome::Passed)
            .count()
    }
    pub fn failed(&self) -> usize {
        self.cases
            .iter()
            .filter(|x| x.outcome == TestOutcome::Failed)
            .count()
    }
    pub fn snapshot(&self) -> &[TestReport] {
        &self.cases
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sorts_and_counts_reports() {
        let h = TestHarness::new(2)
            .add("z", TestOutcome::Passed, "ok")
            .unwrap()
            .add("a", TestOutcome::Failed, "bad")
            .unwrap();
        assert_eq!(h.snapshot()[0].name, "a");
        assert_eq!(h.passed(), 1);
        assert_eq!(h.failed(), 1)
    }
    #[test]
    fn validates_and_bounds_cases() {
        assert!(TestHarness::new(1)
            .add("bad name", TestOutcome::Passed, "ok")
            .is_err());
        let h = TestHarness::new(1)
            .add("a", TestOutcome::Passed, "ok")
            .unwrap();
        assert!(h.add("b", TestOutcome::Passed, "ok").is_err());
        assert!(TestHarness::new(1)
            .add("a", TestOutcome::Passed, "x".repeat(513))
            .is_err())
    }
}
