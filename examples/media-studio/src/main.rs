use yaiko_core::MediaEditorRepository;
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite://media-studio.db?mode=rwc")
        .await?;
    let repository = MediaEditorRepository::new(pool, 8);
    repository.migrate().await?;

    let project = match repository.create("demo-project", "demo-tenant").await {
        Ok(snapshot) => snapshot,
        Err(_) => repository.snapshot("demo-project", "demo-tenant").await?,
    };
    let with_video = if project.assets.is_empty() {
        repository
            .add_asset(
                "demo-project",
                "demo-tenant",
                project.revision,
                "video.mp4",
            )
            .await?
    } else {
        project
    };
    let final_snapshot = if with_video.timeline.is_empty() {
        repository
            .set_timeline(
                "demo-project",
                "demo-tenant",
                with_video.revision,
                "video.mp4 -> captions -> background-music",
            )
            .await?
    } else {
        with_video
    };

    println!("Yaiko media studio project persisted successfully:");
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "project_id": final_snapshot.project_id,
        "scope": final_snapshot.scope,
        "revision": final_snapshot.revision,
        "assets": final_snapshot.assets,
        "timeline": final_snapshot.timeline,
        "database": "media-studio.db"
    }))?);
    Ok(())
}
