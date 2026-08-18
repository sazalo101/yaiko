//! Provider-neutral migration planning and checksum tracking.

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migration {
    pub version: u64,
    pub name: String,
    pub up_sql: String,
    pub down_sql: Option<String>,
    pub checksum: String,
}

impl Migration {
    pub fn new(
        version: u64,
        name: impl Into<String>,
        up_sql: impl Into<String>,
        down_sql: Option<String>,
    ) -> Self {
        let name = name.into();
        let up_sql = up_sql.into();
        let checksum = checksum_for(&name, &up_sql, down_sql.as_deref());
        Self {
            version,
            name,
            up_sql,
            down_sql,
            checksum,
        }
    }

    pub fn verify_checksum(&self) -> bool {
        self.checksum == checksum_for(&self.name, &self.up_sql, self.down_sql.as_deref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedMigration {
    pub version: u64,
    pub checksum: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationPlan {
    pub pending: Vec<Migration>,
    pub already_applied: Vec<AppliedMigration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationError {
    DuplicateVersion,
    InvalidChecksum,
    ChecksumDrift { version: u64 },
    MissingRollback { version: u64 },
    UnknownMigration { version: u64 },
}

#[derive(Clone, Default)]
pub struct MigrationRunner {
    applied: Arc<Mutex<BTreeMap<u64, AppliedMigration>>>,
}

impl MigrationRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn validate(migrations: &[Migration]) -> Result<Vec<Migration>, MigrationError> {
        let mut versions = BTreeSet::new();
        let mut ordered = migrations.to_vec();
        for migration in &ordered {
            if !versions.insert(migration.version) {
                return Err(MigrationError::DuplicateVersion);
            }
            if !migration.verify_checksum() {
                return Err(MigrationError::InvalidChecksum);
            }
        }
        ordered.sort_by_key(|migration| migration.version);
        Ok(ordered)
    }

    pub async fn plan(&self, migrations: &[Migration]) -> Result<MigrationPlan, MigrationError> {
        let migrations = Self::validate(migrations)?;
        let applied = self.applied.lock().await;
        let mut pending = Vec::new();
        let mut already_applied = Vec::new();
        for migration in migrations {
            if let Some(record) = applied.get(&migration.version) {
                if record.checksum != migration.checksum {
                    return Err(MigrationError::ChecksumDrift {
                        version: migration.version,
                    });
                }
                already_applied.push(record.clone());
            } else {
                pending.push(migration);
            }
        }
        Ok(MigrationPlan {
            pending,
            already_applied,
        })
    }

    pub async fn apply(&self, migrations: &[Migration]) -> Result<MigrationPlan, MigrationError> {
        let plan = self.plan(migrations).await?;
        let mut applied = self.applied.lock().await;
        for migration in &plan.pending {
            applied.insert(
                migration.version,
                AppliedMigration {
                    version: migration.version,
                    checksum: migration.checksum.clone(),
                },
            );
        }
        Ok(plan)
    }

    pub async fn rollback(
        &self,
        migrations: &[Migration],
        version: u64,
    ) -> Result<Migration, MigrationError> {
        let migrations = Self::validate(migrations)?;
        let migration = migrations
            .iter()
            .find(|migration| migration.version == version)
            .ok_or(MigrationError::UnknownMigration { version })?;
        if migration.down_sql.is_none() {
            return Err(MigrationError::MissingRollback { version });
        }
        let mut applied = self.applied.lock().await;
        if applied.remove(&version).is_none() {
            return Err(MigrationError::UnknownMigration { version });
        }
        Ok(migration.clone())
    }

    pub async fn applied(&self) -> Vec<AppliedMigration> {
        self.applied.lock().await.values().cloned().collect()
    }
}

fn checksum_for(name: &str, up_sql: &str, down_sql: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    hasher.update(b"\n");
    hasher.update(up_sql.as_bytes());
    hasher.update(b"\n");
    hasher.update(down_sql.unwrap_or_default().as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrations() -> Vec<Migration> {
        vec![
            Migration::new(
                2,
                "add captions",
                "CREATE TABLE captions;",
                Some("DROP TABLE captions;".into()),
            ),
            Migration::new(
                1,
                "create projects",
                "CREATE TABLE projects;",
                Some("DROP TABLE projects;".into()),
            ),
        ]
    }

    #[tokio::test]
    async fn validates_orders_plans_and_applies_migrations() {
        let runner = MigrationRunner::new();
        let plan = runner.plan(&migrations()).await.unwrap();
        assert_eq!(
            plan.pending
                .iter()
                .map(|migration| migration.version)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        runner.apply(&migrations()).await.unwrap();
        assert_eq!(runner.plan(&migrations()).await.unwrap().pending.len(), 0);
        assert_eq!(runner.applied().await.len(), 2);
    }

    #[tokio::test]
    async fn detects_checksum_drift_and_supports_rollback() {
        let runner = MigrationRunner::new();
        runner.apply(&migrations()).await.unwrap();
        let mut changed = migrations();
        changed[0] = Migration::new(
            2,
            "add captions",
            "ALTERED",
            Some("DROP TABLE captions;".into()),
        );
        assert_eq!(
            runner.plan(&changed).await.unwrap_err(),
            MigrationError::ChecksumDrift { version: 2 }
        );
        let rolled_back = runner.rollback(&migrations(), 2).await.unwrap();
        assert_eq!(rolled_back.version, 2);
        assert_eq!(runner.applied().await.len(), 1);
    }

    #[test]
    fn rejects_duplicate_versions_and_invalid_checksums() {
        let mut duplicate = migrations();
        duplicate.push(Migration::new(1, "duplicate", "SELECT 1", None));
        assert_eq!(
            MigrationRunner::validate(&duplicate).unwrap_err(),
            MigrationError::DuplicateVersion
        );
        let mut invalid = migrations();
        invalid[0].checksum = "bad".into();
        assert_eq!(
            MigrationRunner::validate(&invalid).unwrap_err(),
            MigrationError::InvalidChecksum
        );
    }
}
