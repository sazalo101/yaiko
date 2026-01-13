# WebSockets

Yaiko provides built-in support for WebSocket connections, making it easy to build real-time applications like chat apps, notifications, and live dashboards.

## Usage

### 1. Import WebSocket Types

```rust
use yaiko_core::{WebSocketManager, WebSocketConnection, is_websocket_upgrade};
```

### 2. Check for Upgrade Request

In your handler, check if the request is a WebSocket upgrade:

```rust
async fn websocket_handler(req: Request, state: AppState) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    if is_websocket_upgrade(&req) {
        // Handle upgrade...
    }
    // ...
}
```

### 3. Managing Connections

Use `WebSocketManager` to track active connections:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    ws_manager: Arc<WebSocketManager>,
}

// In main:
let ws_manager = Arc::new(WebSocketManager::new());
```

### 4. Example Implementation

Here's a simple example of handling a WebSocket upgrade:

```rust
use yaiko_core::websocket::websocket_upgrade_response;

async fn ws_handler(req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    if !is_websocket_upgrade(&req) {
        return Ok(Response::new().status(StatusCode::BAD_REQUEST).text("Not a websocket request"));
    }

    let key = req.headers.get("sec-websocket-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Create upgrade response
    let response = websocket_upgrade_response(key);
    
    // Note: You need to handle the actual WebSocket stream using a library 
    // like tokio-tungstenite after the upgrade.
    
    Ok(response)
}
```

## Client-Side

Connect from the frontend using standard JavaScript WebSocket API:

```javascript
const ws = new WebSocket(`ws://${window.location.host}/ws`);

ws.onopen = () => {
    console.log('Connected');
};

ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    console.log('Received:', data);
};
```
