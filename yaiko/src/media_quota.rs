//! Scoped byte/file quota accounting for media uploads and generated outputs.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuotaError {
    InvalidScope,
    ByteLimit,
    FileLimit,
    Overflow,
    ReservationClosed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuotaLimit {
    pub max_bytes: u64,
    pub max_files: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QuotaUsage {
    pub used_bytes: u64,
    pub used_files: u64,
    pub reserved_bytes: u64,
    pub reserved_files: u64,
}

impl QuotaUsage {
    pub fn total_bytes(self) -> u64 {
        self.used_bytes.saturating_add(self.reserved_bytes)
    }
    pub fn total_files(self) -> u64 {
        self.used_files.saturating_add(self.reserved_files)
    }
}

#[derive(Debug, Clone)]
pub struct MediaQuota {
    inner: Arc<Mutex<HashMap<String, (QuotaLimit, QuotaUsage)>>>,
}

impl Default for MediaQuota {
    fn default() -> Self {
        Self::new()
    }
}
impl MediaQuota {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub fn configure(&self, scope: impl Into<String>, limit: QuotaLimit) -> Result<(), QuotaError> {
        let scope = validate_scope(scope.into())?;
        self.inner
            .lock()
            .unwrap()
            .insert(scope, (limit, QuotaUsage::default()));
        Ok(())
    }
    pub fn usage(&self, scope: &str) -> Result<QuotaUsage, QuotaError> {
        let scope = validate_scope(scope.to_string())?;
        self.inner
            .lock()
            .unwrap()
            .get(&scope)
            .map(|(_, usage)| *usage)
            .ok_or(QuotaError::InvalidScope)
    }
    pub fn reserve(
        &self,
        scope: impl Into<String>,
        bytes: u64,
        files: u64,
    ) -> Result<QuotaReservation, QuotaError> {
        let scope = validate_scope(scope.into())?;
        if bytes == 0 || files == 0 {
            return Err(QuotaError::Overflow);
        }
        let mut guard = self.inner.lock().unwrap();
        let (limit, usage) = guard.get_mut(&scope).ok_or(QuotaError::InvalidScope)?;
        if usage.total_bytes().checked_add(bytes).is_none() {
            return Err(QuotaError::Overflow);
        }
        if usage.total_files().checked_add(files).is_none() {
            return Err(QuotaError::Overflow);
        }
        if usage.total_bytes() + bytes > limit.max_bytes {
            return Err(QuotaError::ByteLimit);
        }
        if usage.total_files() + files > limit.max_files {
            return Err(QuotaError::FileLimit);
        }
        usage.reserved_bytes += bytes;
        usage.reserved_files += files;
        Ok(QuotaReservation {
            quota: self.clone(),
            scope,
            bytes,
            files,
            state: ReservationState::Open,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReservationState {
    Open,
    Committed,
    Released,
}

#[derive(Debug)]
pub struct QuotaReservation {
    quota: MediaQuota,
    scope: String,
    bytes: u64,
    files: u64,
    state: ReservationState,
}

impl QuotaReservation {
    pub fn commit(&mut self) -> Result<(), QuotaError> {
        if self.state != ReservationState::Open {
            return Err(QuotaError::ReservationClosed);
        }
        let mut guard = self.quota.inner.lock().unwrap();
        let (_, usage) = guard.get_mut(&self.scope).ok_or(QuotaError::InvalidScope)?;
        usage.reserved_bytes = usage.reserved_bytes.saturating_sub(self.bytes);
        usage.reserved_files = usage.reserved_files.saturating_sub(self.files);
        usage.used_bytes = usage
            .used_bytes
            .checked_add(self.bytes)
            .ok_or(QuotaError::Overflow)?;
        usage.used_files = usage
            .used_files
            .checked_add(self.files)
            .ok_or(QuotaError::Overflow)?;
        self.state = ReservationState::Committed;
        Ok(())
    }
    pub fn release(&mut self) -> Result<(), QuotaError> {
        if self.state != ReservationState::Open {
            return Err(QuotaError::ReservationClosed);
        }
        let mut guard = self.quota.inner.lock().unwrap();
        let (_, usage) = guard.get_mut(&self.scope).ok_or(QuotaError::InvalidScope)?;
        usage.reserved_bytes = usage.reserved_bytes.saturating_sub(self.bytes);
        usage.reserved_files = usage.reserved_files.saturating_sub(self.files);
        self.state = ReservationState::Released;
        Ok(())
    }
    pub fn scope(&self) -> &str {
        &self.scope
    }
}

fn validate_scope(scope: String) -> Result<String, QuotaError> {
    if scope.is_empty()
        || scope.len() > 128
        || scope
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err(QuotaError::InvalidScope);
    }
    Ok(scope)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn quota() -> MediaQuota {
        let quota = MediaQuota::new();
        quota
            .configure(
                "tenant-a",
                QuotaLimit {
                    max_bytes: 1000,
                    max_files: 2,
                },
            )
            .unwrap();
        quota
            .configure(
                "tenant-b",
                QuotaLimit {
                    max_bytes: 1000,
                    max_files: 2,
                },
            )
            .unwrap();
        quota
    }
    #[test]
    fn reserves_commits_and_releases_usage() {
        let quota = quota();
        let mut reservation = quota.reserve("tenant-a", 400, 1).unwrap();
        assert_eq!(quota.usage("tenant-a").unwrap().reserved_bytes, 400);
        reservation.commit().unwrap();
        assert_eq!(
            quota.usage("tenant-a").unwrap(),
            QuotaUsage {
                used_bytes: 400,
                used_files: 1,
                reserved_bytes: 0,
                reserved_files: 0
            }
        );
        assert_eq!(reservation.commit(), Err(QuotaError::ReservationClosed));
        let mut released = quota.reserve("tenant-a", 100, 1).unwrap();
        released.release().unwrap();
        assert_eq!(quota.usage("tenant-a").unwrap().used_bytes, 400);
    }
    #[test]
    fn enforces_bytes_files_and_scope_isolation() {
        let quota = quota();
        assert_eq!(
            quota.reserve("tenant-a", 1001, 1).unwrap_err(),
            QuotaError::ByteLimit
        );
        assert_eq!(
            quota.reserve("tenant-a", 1, 3).unwrap_err(),
            QuotaError::FileLimit
        );
        let mut reservation = quota.reserve("tenant-a", 900, 1).unwrap();
        reservation.commit().unwrap();
        assert_eq!(quota.usage("tenant-b").unwrap(), QuotaUsage::default());
    }
    #[test]
    fn rejects_invalid_scopes_zero_values_and_overflow() {
        let quota = quota();
        assert_eq!(
            quota.reserve("../tenant", 1, 1).unwrap_err(),
            QuotaError::InvalidScope
        );
        assert_eq!(
            quota.reserve("tenant-a", 0, 1).unwrap_err(),
            QuotaError::Overflow
        );
        assert_eq!(
            quota.usage("missing").unwrap_err(),
            QuotaError::InvalidScope
        );
    }
    #[test]
    fn concurrent_reservations_are_atomic() {
        let quota = quota();
        let first = quota.reserve("tenant-a", 600, 1).unwrap();
        assert_eq!(
            quota.reserve("tenant-a", 500, 1).unwrap_err(),
            QuotaError::ByteLimit
        );
        drop(first);
        assert_eq!(quota.usage("tenant-a").unwrap().reserved_bytes, 600);
    }
}
