//! Structured observability log records with deterministic redaction.
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogError {
    InvalidMessage,
    Capacity,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    pub level: LogLevel,
    pub message: String,
    pub fields: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Default)]
pub struct LogFacade {
    records: Vec<LogRecord>,
    capacity: usize,
}
impl LogFacade {
    pub fn new(capacity: usize) -> Self {
        Self {
            records: Vec::new(),
            capacity,
        }
    }
    pub fn record(
        &mut self,
        level: LogLevel,
        message: impl Into<String>,
        fields: impl IntoIterator<Item = (String, String)>,
    ) -> Result<(), LogError> {
        if self.records.len() >= self.capacity {
            return Err(LogError::Capacity);
        }
        let message = message.into();
        if message.is_empty() || message.len() > 512 {
            return Err(LogError::InvalidMessage);
        }
        let fields = fields
            .into_iter()
            .map(|(k, v)| {
                let redacted = if k.to_ascii_lowercase().contains("password")
                    || k.to_ascii_lowercase().contains("secret")
                    || k.to_ascii_lowercase().contains("token")
                {
                    "[REDACTED]".into()
                } else {
                    v
                };
                (k, redacted)
            })
            .collect();
        self.records.push(LogRecord {
            level,
            message,
            fields,
        });
        Ok(())
    }
    pub fn render(&self) -> Vec<String> {
        self.records
            .iter()
            .map(|r| {
                let fields = r
                    .fields
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!(
                    "{:?} {}{}",
                    r.level,
                    r.message,
                    if fields.is_empty() {
                        String::new()
                    } else {
                        format!(" {fields}")
                    }
                )
            })
            .collect()
    }
    pub fn snapshot(&self) -> &[LogRecord] {
        &self.records
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redacts_and_renders_deterministically() {
        let mut l = LogFacade::new(2);
        l.record(
            LogLevel::Info,
            "uploaded",
            [
                ("token".into(), "secret".into()),
                ("asset".into(), "a1".into()),
            ],
        )
        .unwrap();
        assert_eq!(l.render()[0], "Info uploaded asset=a1 token=[REDACTED]")
    }
    #[test]
    fn validates_message_and_capacity() {
        let mut l = LogFacade::new(1);
        assert!(l.record(LogLevel::Info, "", Vec::new()).is_err());
        l.record(LogLevel::Warn, "ok", Vec::new()).unwrap();
        assert!(l.record(LogLevel::Error, "third", Vec::new()).is_err())
    }
}
