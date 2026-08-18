//! Deterministic retention planning for generated media artifacts.

use crate::media_processing::MediaPath;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetentionError {
    UnsafePath,
    InvalidScope,
    InvalidArtifact,
    TooManyArtifacts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionPolicy {
    pub max_age: Duration,
    pub max_bytes: u64,
    pub max_artifacts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaArtifact {
    pub scope: String,
    pub path: MediaPath,
    pub bytes: u64,
    pub created_at: SystemTime,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupReport {
    pub removed: Vec<MediaPath>,
    pub retained: Vec<MediaPath>,
    pub removed_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct MediaRetention {
    artifacts: HashMap<String, Vec<MediaArtifact>>,
    max_registry_entries: usize,
}

impl MediaRetention {
    pub fn new(max_registry_entries: usize) -> Self {
        Self {
            artifacts: HashMap::new(),
            max_registry_entries: max_registry_entries.max(1),
        }
    }
    pub fn register(
        &mut self,
        scope: impl Into<String>,
        path: impl Into<PathBuf>,
        bytes: u64,
        created_at: SystemTime,
        task_id: Option<String>,
    ) -> Result<(), RetentionError> {
        let scope = validate_scope(scope.into())?;
        let path = MediaPath::new(path).map_err(|_| RetentionError::UnsafePath)?;
        if bytes == 0
            || task_id
                .as_ref()
                .is_some_and(|id| id.is_empty() || id.len() > 128)
        {
            return Err(RetentionError::InvalidArtifact);
        }
        if self.artifacts.values().map(Vec::len).sum::<usize>() >= self.max_registry_entries {
            return Err(RetentionError::TooManyArtifacts);
        }
        self.artifacts
            .entry(scope.clone())
            .or_default()
            .push(MediaArtifact {
                scope,
                path,
                bytes,
                created_at,
                task_id,
            });
        Ok(())
    }
    pub fn plan_cleanup(
        &self,
        scope: &str,
        policy: RetentionPolicy,
        now: SystemTime,
    ) -> Result<CleanupReport, RetentionError> {
        let scope = validate_scope(scope.to_string())?;
        let mut artifacts = self.artifacts.get(&scope).cloned().unwrap_or_default();
        artifacts.sort_by_key(|artifact| artifact.created_at);
        let mut total_bytes = artifacts.iter().map(|artifact| artifact.bytes).sum::<u64>();
        let mut report = CleanupReport {
            removed: Vec::new(),
            retained: Vec::new(),
            removed_bytes: 0,
        };
        for artifact in artifacts {
            let age = now
                .duration_since(artifact.created_at)
                .unwrap_or(Duration::ZERO);
            let expired = age > policy.max_age;
            let over_bytes = total_bytes > policy.max_bytes;
            let over_count = report.retained.len() >= policy.max_artifacts;
            if (expired || over_bytes || over_count) && artifact.task_id.is_none() {
                total_bytes = total_bytes.saturating_sub(artifact.bytes);
                report.removed_bytes = report.removed_bytes.saturating_add(artifact.bytes);
                report.removed.push(artifact.path);
            } else {
                report.retained.push(artifact.path);
            }
        }
        Ok(report)
    }
    pub fn scopes(&self) -> usize {
        self.artifacts.len()
    }
}

fn validate_scope(scope: String) -> Result<String, RetentionError> {
    if scope.is_empty()
        || scope.len() > 128
        || scope
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err(RetentionError::InvalidScope);
    }
    Ok(scope)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn now() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(10_000)
    }
    #[test]
    fn removes_expired_and_preserves_active_task_outputs() {
        let mut retention = MediaRetention::new(10);
        retention
            .register(
                "tenant-a",
                "old.mp4",
                100,
                now() - Duration::from_secs(100),
                None,
            )
            .unwrap();
        retention
            .register(
                "tenant-a",
                "active.mp4",
                200,
                now() - Duration::from_secs(100),
                Some("task-1".into()),
            )
            .unwrap();
        let report = retention
            .plan_cleanup(
                "tenant-a",
                RetentionPolicy {
                    max_age: Duration::from_secs(10),
                    max_bytes: 1000,
                    max_artifacts: 10,
                },
                now(),
            )
            .unwrap();
        assert_eq!(report.removed, vec![MediaPath::new("old.mp4").unwrap()]);
        assert_eq!(report.retained, vec![MediaPath::new("active.mp4").unwrap()]);
        assert_eq!(report.removed_bytes, 100);
    }
    #[test]
    fn enforces_bytes_and_count_deterministically() {
        let mut retention = MediaRetention::new(10);
        retention
            .register(
                "tenant-a",
                "a.mp4",
                100,
                now() - Duration::from_secs(3),
                None,
            )
            .unwrap();
        retention
            .register(
                "tenant-a",
                "b.mp4",
                200,
                now() - Duration::from_secs(2),
                None,
            )
            .unwrap();
        retention
            .register(
                "tenant-a",
                "c.mp4",
                300,
                now() - Duration::from_secs(1),
                None,
            )
            .unwrap();
        let report = retention
            .plan_cleanup(
                "tenant-a",
                RetentionPolicy {
                    max_age: Duration::from_secs(100),
                    max_bytes: 300,
                    max_artifacts: 2,
                },
                now(),
            )
            .unwrap();
        assert_eq!(
            report.removed,
            vec![
                MediaPath::new("a.mp4").unwrap(),
                MediaPath::new("b.mp4").unwrap()
            ]
        );
        assert_eq!(report.retained, vec![MediaPath::new("c.mp4").unwrap()]);
    }
    #[test]
    fn isolates_scopes_and_validates_registry_inputs() {
        let mut retention = MediaRetention::new(1);
        retention
            .register("tenant-a", "a.mp4", 1, now(), None)
            .unwrap();
        assert_eq!(
            retention
                .plan_cleanup(
                    "tenant-b",
                    RetentionPolicy {
                        max_age: Duration::from_secs(1),
                        max_bytes: 1,
                        max_artifacts: 1
                    },
                    now()
                )
                .unwrap()
                .removed
                .len(),
            0
        );
        assert_eq!(
            retention.register("../bad", "b.mp4", 1, now(), None),
            Err(RetentionError::InvalidScope)
        );
        assert_eq!(
            retention.register("tenant-a", "b.mp4", 1, now(), None),
            Err(RetentionError::TooManyArtifacts)
        );
    }
}
