//! Persistent SQLx-backed storage for media-editor projects.
//!
//! Enable the `persistent-media` feature to expose [`MediaEditorRepository`].
//! The repository uses one row per project and stores the ordered asset list as
//! JSON so reads and writes remain deterministic across SQLite deployments.

use crate::media_editor::{EditorError, EditorSnapshot};
use sqlx::{Row, SqlitePool};

#[derive(Debug)]
pub enum MediaEditorRepositoryError {
    Database(sqlx::Error),
    Editor(EditorError),
    CorruptAssets(serde_json::Error),
}

impl From<sqlx::Error> for MediaEditorRepositoryError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl From<EditorError> for MediaEditorRepositoryError {
    fn from(error: EditorError) -> Self {
        Self::Editor(error)
    }
}

/// A persistent media-editor project repository backed by SQLite.
#[derive(Debug, Clone)]
pub struct MediaEditorRepository {
    pool: SqlitePool,
    max_assets: usize,
}

impl MediaEditorRepository {
    /// Creates a repository with a positive per-project asset limit.
    pub fn new(pool: SqlitePool, max_assets: usize) -> Self {
        Self {
            pool,
            max_assets: max_assets.max(1),
        }
    }

    /// Creates the repository table. Calling this method repeatedly is safe.
    pub async fn migrate(&self) -> Result<(), MediaEditorRepositoryError> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS yaiko_media_editor_projects (\
                project_id TEXT PRIMARY KEY NOT NULL,\
                scope TEXT NOT NULL,\
                revision INTEGER NOT NULL,\
                assets_json TEXT NOT NULL,\
                timeline TEXT NOT NULL
            )",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Creates a project at revision zero.
    pub async fn create(
        &self,
        project_id: impl Into<String>,
        scope: impl Into<String>,
    ) -> Result<EditorSnapshot, MediaEditorRepositoryError> {
        let project_id = valid_id(project_id.into())?;
        let scope = valid_id(scope.into())?;
        let result = sqlx::query(
            "INSERT INTO yaiko_media_editor_projects
             (project_id, scope, revision, assets_json, timeline)
             VALUES (?, ?, 0, '[]', '')",
        )
        .bind(&project_id)
        .bind(&scope)
        .execute(&self.pool)
        .await;
        match result {
            Ok(_) => self.snapshot(&project_id, &scope).await,
            Err(sqlx::Error::Database(error)) if error.is_unique_violation() => {
                Err(EditorError::Conflict.into())
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Appends an asset when the caller supplies the current revision.
    pub async fn add_asset(
        &self,
        project_id: &str,
        scope: &str,
        expected_revision: u64,
        asset_id: impl Into<String>,
    ) -> Result<EditorSnapshot, MediaEditorRepositoryError> {
        let asset_id = valid_id(asset_id.into())?;
        let mut transaction = self.pool.begin().await?;
        let row = load_row(&mut transaction, project_id, scope).await?;
        let (revision, mut assets, timeline) = row.ok_or(EditorError::NotFound)?;
        if revision != expected_revision as i64 {
            return Err(EditorError::Conflict.into());
        }
        if assets.len() >= self.max_assets {
            return Err(EditorError::TooManyAssets.into());
        }
        if assets.iter().any(|asset| asset == &asset_id) {
            return Err(EditorError::Conflict.into());
        }
        assets.push(asset_id);
        let next_revision = revision.checked_add(1).ok_or(EditorError::Conflict)?;
        sqlx::query(
            "UPDATE yaiko_media_editor_projects
             SET revision = ?, assets_json = ?
             WHERE project_id = ? AND scope = ? AND revision = ?",
        )
        .bind(next_revision)
        .bind(serde_json::to_string(&assets).map_err(MediaEditorRepositoryError::CorruptAssets)?)
        .bind(project_id)
        .bind(scope)
        .bind(revision)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(EditorSnapshot {
            project_id: project_id.to_owned(),
            scope: scope.to_owned(),
            revision: next_revision as u64,
            assets,
            timeline,
        })
    }

    /// Replaces the timeline when the caller supplies the current revision.
    pub async fn set_timeline(
        &self,
        project_id: &str,
        scope: &str,
        expected_revision: u64,
        timeline: impl Into<String>,
    ) -> Result<EditorSnapshot, MediaEditorRepositoryError> {
        let timeline = timeline.into();
        if timeline.len() > 16_384 {
            return Err(EditorError::TooManyAssets.into());
        }
        let mut transaction = self.pool.begin().await?;
        let row = load_row(&mut transaction, project_id, scope).await?;
        let (revision, assets, _) = row.ok_or(EditorError::NotFound)?;
        if revision != expected_revision as i64 {
            return Err(EditorError::Conflict.into());
        }
        let next_revision = revision.checked_add(1).ok_or(EditorError::Conflict)?;
        sqlx::query(
            "UPDATE yaiko_media_editor_projects
             SET revision = ?, timeline = ?
             WHERE project_id = ? AND scope = ? AND revision = ?",
        )
        .bind(next_revision)
        .bind(&timeline)
        .bind(project_id)
        .bind(scope)
        .bind(revision)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(EditorSnapshot {
            project_id: project_id.to_owned(),
            scope: scope.to_owned(),
            revision: next_revision as u64,
            assets,
            timeline,
        })
    }

    /// Loads a project while enforcing tenant/scope isolation.
    pub async fn snapshot(
        &self,
        project_id: &str,
        scope: &str,
    ) -> Result<EditorSnapshot, MediaEditorRepositoryError> {
        let mut connection = self.pool.acquire().await?;
        let row = load_row(&mut connection, project_id, scope).await?;
        let (revision, assets, timeline) = row.ok_or(EditorError::NotFound)?;
        Ok(EditorSnapshot {
            project_id: project_id.to_owned(),
            scope: scope.to_owned(),
            revision: revision as u64,
            assets,
            timeline,
        })
    }
}

async fn load_row(
    executor: &mut sqlx::SqliteConnection,
    project_id: &str,
    scope: &str,
) -> Result<Option<(i64, Vec<String>, String)>, MediaEditorRepositoryError> {
    let row = sqlx::query(
        "SELECT revision, assets_json, timeline
         FROM yaiko_media_editor_projects
         WHERE project_id = ? AND scope = ?",
    )
    .bind(project_id)
    .bind(scope)
    .fetch_optional(executor)
    .await?;
    row.map(|row| {
        let assets_json: String = row.try_get("assets_json")?;
        let assets = serde_json::from_str(&assets_json)
            .map_err(MediaEditorRepositoryError::CorruptAssets)?;
        Ok((row.try_get("revision")?, assets, row.try_get("timeline")?))
    })
    .transpose()
}

fn valid_id(value: String) -> Result<String, MediaEditorRepositoryError> {
    if value.is_empty()
        || value.len() > 128
        || value
            .chars()
            .any(|character| character.is_control() || matches!(character, '/' | '\\'))
    {
        return Err(EditorError::InvalidId.into());
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn repository() -> MediaEditorRepository {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let repository = MediaEditorRepository::new(pool, 2);
        repository.migrate().await.unwrap();
        repository
    }

    #[tokio::test]
    async fn persists_projects_assets_and_timeline() {
        let repository = repository().await;
        let created = repository.create("project-1", "tenant-1").await.unwrap();
        let asset = repository
            .add_asset("project-1", "tenant-1", created.revision, "video.mp4")
            .await
            .unwrap();
        let timeline = repository
            .set_timeline("project-1", "tenant-1", asset.revision, "timeline-v1")
            .await
            .unwrap();
        assert_eq!(timeline.revision, 2);
        assert_eq!(timeline.assets, vec!["video.mp4"]);
        assert_eq!(
            repository.snapshot("project-1", "tenant-1").await.unwrap(),
            timeline
        );
    }

    #[tokio::test]
    async fn enforces_scope_revision_and_asset_bounds() {
        let repository = repository().await;
        let created = repository.create("project-1", "tenant-1").await.unwrap();
        assert!(matches!(
            repository.snapshot("project-1", "tenant-2").await,
            Err(MediaEditorRepositoryError::Editor(EditorError::NotFound))
        ));
        let asset = repository
            .add_asset("project-1", "tenant-1", created.revision, "video.mp4")
            .await
            .unwrap();
        assert!(matches!(
            repository
                .add_asset("project-1", "tenant-1", created.revision, "music.mp3")
                .await,
            Err(MediaEditorRepositoryError::Editor(EditorError::Conflict))
        ));
        repository
            .add_asset("project-1", "tenant-1", asset.revision, "music.mp3")
            .await
            .unwrap();
        assert!(matches!(
            repository
                .add_asset("project-1", "tenant-1", 2, "voice.wav")
                .await,
            Err(MediaEditorRepositoryError::Editor(
                EditorError::TooManyAssets
            ))
        ));
    }
}
