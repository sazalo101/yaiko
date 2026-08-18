//! Deterministic transaction state helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionState {
    Active,
    Committed,
    RolledBack,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionError {
    InvalidState,
    Capacity,
    InvalidRetries,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub id: String,
    pub operations: Vec<String>,
    pub retries: u32,
    pub max_retries: u32,
    pub state: TransactionState,
}
impl Transaction {
    pub fn begin(id: impl Into<String>, max_retries: u32) -> Result<Self, TransactionError> {
        if max_retries > 32 {
            return Err(TransactionError::InvalidRetries);
        }
        Ok(Self {
            id: id.into(),
            operations: Vec::new(),
            retries: 0,
            max_retries,
            state: TransactionState::Active,
        })
    }
    pub fn record(&mut self, operation: impl Into<String>) -> Result<(), TransactionError> {
        if self.state != TransactionState::Active {
            return Err(TransactionError::InvalidState);
        }
        let operation = operation.into();
        if operation.is_empty() || operation.len() > 256 {
            return Err(TransactionError::Capacity);
        }
        self.operations.push(operation);
        Ok(())
    }
    pub fn commit(&mut self) -> Result<(), TransactionError> {
        if self.state != TransactionState::Active {
            return Err(TransactionError::InvalidState);
        }
        self.state = TransactionState::Committed;
        Ok(())
    }
    pub fn rollback(&mut self) -> Result<(), TransactionError> {
        if self.state != TransactionState::Active {
            return Err(TransactionError::InvalidState);
        }
        self.state = TransactionState::RolledBack;
        Ok(())
    }
    pub fn retry(&mut self) -> Result<(), TransactionError> {
        if self.state != TransactionState::Active {
            return Err(TransactionError::InvalidState);
        }
        if self.retries >= self.max_retries {
            return Err(TransactionError::InvalidRetries);
        }
        self.retries += 1;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn records_and_commits_once() {
        let mut t = Transaction::begin("tx-1", 1).unwrap();
        t.record("insert media").unwrap();
        t.commit().unwrap();
        assert_eq!(t.state, TransactionState::Committed);
        assert!(t.record("late").is_err())
    }
    #[test]
    fn rolls_back_and_bounds_retries() {
        let mut t = Transaction::begin("tx-2", 1).unwrap();
        t.retry().unwrap();
        assert!(t.retry().is_err());
        t.rollback().unwrap();
        assert!(t.commit().is_err())
    }
}
