use std::time::Duration;

use serde_json::json;
use yaiko_core::{WebhookEvent, WebhookVerifier};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let verifier = WebhookVerifier::new("local-development-secret", Duration::from_secs(300));
    let event = WebhookEvent::new(
        "evt-caption-001",
        "caption.created",
        json!({"project_id": "demo-project", "caption": "Hello Yaiko"}),
    );
    let body = event.body()?;
    let timestamp = 1_700_000_000;
    let signature = verifier.signature(timestamp, &body)?;
    let verified_at = verifier.verify(&signature, &body, timestamp).await?;

    println!("Accepted webhook {} at timestamp {}", event.id, verified_at);
    println!("Event type: {}", event.event_type);
    println!("Payload: {}", serde_json::to_string_pretty(&event.payload)?);
    println!("Replay protection: enabled");
    Ok(())
}
