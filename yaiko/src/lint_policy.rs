//! Deterministic lint policy and diagnostic reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LintSeverity {
    Info,
    Warning,
    Error,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintError {
    InvalidRule,
    InvalidMessage,
    Capacity,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintDiagnostic {
    pub rule: String,
    pub severity: LintSeverity,
    pub message: String,
    pub suppressed: bool,
}
#[derive(Debug, Clone, Default)]
pub struct LintPolicy {
    diagnostics: Vec<LintDiagnostic>,
    capacity: usize,
}
impl LintPolicy {
    pub fn new(capacity: usize) -> Self {
        Self {
            diagnostics: Vec::new(),
            capacity,
        }
    }
    pub fn add(
        mut self,
        rule: impl Into<String>,
        severity: LintSeverity,
        message: impl Into<String>,
        suppressed: bool,
    ) -> Result<Self, LintError> {
        let rule = rule.into();
        let message = message.into();
        if rule.is_empty()
            || rule.len() > 128
            || rule.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(LintError::InvalidRule);
        }
        if message.is_empty() || message.len() > 512 {
            return Err(LintError::InvalidMessage);
        }
        if self.diagnostics.len() >= self.capacity {
            return Err(LintError::Capacity);
        }
        self.diagnostics.push(LintDiagnostic {
            rule,
            severity,
            message,
            suppressed,
        });
        self.diagnostics
            .sort_by(|a, b| a.rule.cmp(&b.rule).then(a.severity.cmp(&b.severity)));
        Ok(self)
    }
    pub fn errors(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == LintSeverity::Error && !d.suppressed)
            .count()
    }
    pub fn active(&self) -> Vec<&LintDiagnostic> {
        self.diagnostics.iter().filter(|d| !d.suppressed).collect()
    }
    pub fn snapshot(&self) -> &[LintDiagnostic] {
        &self.diagnostics
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sorts_and_counts_active_errors() {
        let p = LintPolicy::new(2)
            .add("unused", LintSeverity::Warning, "warn", false)
            .unwrap()
            .add("security", LintSeverity::Error, "bad", true)
            .unwrap();
        assert_eq!(p.snapshot()[0].rule, "security");
        assert_eq!(p.errors(), 0);
        assert_eq!(p.active().len(), 1)
    }
    #[test]
    fn validates_rules_messages_and_capacity() {
        assert!(LintPolicy::new(1)
            .add("bad rule", LintSeverity::Info, "ok", false)
            .is_err());
        let p = LintPolicy::new(1)
            .add("a", LintSeverity::Info, "ok", false)
            .unwrap();
        assert!(p.add("b", LintSeverity::Info, "ok", false).is_err());
        assert!(LintPolicy::new(1)
            .add("a", LintSeverity::Info, "", false)
            .is_err())
    }
}
