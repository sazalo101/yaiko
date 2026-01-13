//! WebSocket support for Yaiko applications
//!
//! Provides WebSocket upgrade handling and connection management.

use crate::{Request, Response};
use hyper::{Body, StatusCode};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// WebSocket connection manager
pub struct WebSocketManager {
    connections: Arc<RwLock<HashMap<String, WebSocketConnection>>>,
}

/// Represents an active WebSocket connection
pub struct WebSocketConnection {
    pub id: String,
    pub user_id: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl WebSocketManager {
    /// Create a new WebSocket manager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the number of active connections
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// Register a new connection
    pub async fn register(&self, id: String, user_id: Option<String>) {
        let conn = WebSocketConnection {
            id: id.clone(),
            user_id,
            metadata: HashMap::new(),
        };
        self.connections.write().await.insert(id, conn);
    }

    /// Remove a connection
    pub async fn unregister(&self, id: &str) {
        self.connections.write().await.remove(id);
    }

    /// Get all connection IDs
    pub async fn get_connection_ids(&self) -> Vec<String> {
        self.connections.read().await.keys().cloned().collect()
    }
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a request is a WebSocket upgrade request
pub fn is_websocket_upgrade(req: &Request) -> bool {
    let upgrade = req.headers
        .get("upgrade")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    
    let connection = req.headers
        .get("connection")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    
    upgrade == "websocket" && connection.contains("upgrade")
}

/// Create a WebSocket upgrade response
/// 
/// Note: This creates the HTTP upgrade response. You'll need to use
/// tokio-tungstenite or similar for actual WebSocket handling.
pub fn websocket_upgrade_response(key: &str) -> Response {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    
    // Calculate Sec-WebSocket-Accept
    let mut hasher = sha1_smol::Sha1::new();
    hasher.update(format!("{}258EAFA5-E914-47DA-95CA-C5AB0DC85B11", key).as_bytes());
    let accept = STANDARD.encode(hasher.digest().bytes());
    
    Response {
        status: StatusCode::SWITCHING_PROTOCOLS,
        headers: {
            let mut h = HashMap::new();
            h.insert("Upgrade".to_string(), "websocket".to_string());
            h.insert("Connection".to_string(), "Upgrade".to_string());
            h.insert("Sec-WebSocket-Accept".to_string(), accept);
            h
        },
        body: Body::empty(),
    }
}
