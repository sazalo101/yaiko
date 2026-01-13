# Background Jobs

Yaiko includes a simple in-memory background job queue for processing tasks asynchronously, such as sending emails, processing images, or generating reports.

## Usage

### 1. Import Job Types

```rust
use yaiko_core::{JobQueue, Job};
use std::sync::Arc;
```

### 2. Initialize Queue

Create a `JobQueue` and start the worker:

```rust
#[tokio::main]
async fn main() {
    let queue = Arc::new(JobQueue::new());
    
    // Start the queue worker in the background
    let queue_clone = queue.clone();
    tokio::spawn(async move {
        queue_clone.start().await;
    });
    
    // ... start server
}
```

### 3. Add Jobs

Add jobs to the queue from your handlers:

```rust
async fn send_email_handler(req: Request, state: AppState) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    // Add a job
    state.queue.add("send_welcome_email", || async {
        // Async task logic here
        send_email("user@example.com", "Welcome!").await;
    }).await;
    
    Ok(Response::new().text("Email queued"))
}
```

## Job Structure

A job consists of:
- **Name**: A string identifier for the job type.
- **Task**: An async closure or function that performs the work.

## Error Handling

Currently, if a job panics or fails, it is logged, but there is no built-in retry mechanism in the basic `JobQueue`. For critical tasks, consider implementing retries within the job closure or using a more robust queue system like Redis-backed queues (e.g., `sidekiq-rs`) if persistence is required.
