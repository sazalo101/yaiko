//! WebSocket support for Yaiko applications
//!
//! Provides WebSocket upgrade handling and connection management
//! with rooms, keepalive, rate limiting, and typed message serialization.

use crate::{Request, Response};
use hyper::{Body, StatusCode};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// WebSocket connection manager
pub struct WebSocketManager {
    connections: Arc<RwLock<HashMap<String, WebSocketConnection>>>,
    rooms: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    rate_limits: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
}

/// Represents an active WebSocket connection
pub struct WebSocketConnection {
    pub id: String,
    pub user_id: Option<String>,
    pub metadata: HashMap<String, String>,
    pub sender: Option<tokio::sync::mpsc::Sender<String>>,
}

impl WebSocketManager {
    /// Create a new WebSocket manager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            rooms: Arc::new(RwLock::new(HashMap::new())),
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the number of active connections
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// Register a new connection
    pub async fn register(&self, id: String, user_id: Option<String>, sender: Option<tokio::sync::mpsc::Sender<String>>) {
        let conn = WebSocketConnection {
            id: id.clone(),
            user_id,
            metadata: HashMap::new(),
            sender,
        };
        self.connections.write().await.insert(id, conn);
    }

    /// Remove a connection and clean up its room memberships
    pub async fn unregister(&self, id: &str) {
        self.connections.write().await.remove(id);
        // Remove from all rooms
        let mut rooms = self.rooms.write().await;
        for members in rooms.values_mut() {
            members.remove(id);
        }
        // Remove empty rooms
        rooms.retain(|_, members| !members.is_empty());
        // Remove rate limit tracking
        self.rate_limits.write().await.remove(id);
    }

    /// Get all connection IDs
    pub async fn get_connection_ids(&self) -> Vec<String> {
        self.connections.read().await.keys().cloned().collect()
    }

    // ── Sending ─────────────────────────────────────────────

    /// Send a message to a specific connection by ID
    pub async fn send(&self, id: &str, msg: String) -> Result<(), &'static str> {
        let connections = self.connections.read().await;
        if let Some(conn) = connections.get(id) {
            if let Some(sender) = &conn.sender {
                if sender.send(msg).await.is_ok() {
                    return Ok(());
                }
            }
        }
        Err("Connection not found or sender unavailable")
    }

    /// Send a message to all connections belonging to a specific user
    pub async fn send_to_user(&self, user_id: &str, msg: String) {
        let connections = self.connections.read().await;
        for conn in connections.values() {
            if conn.user_id.as_deref() == Some(user_id) {
                if let Some(sender) = &conn.sender {
                    let _ = sender.send(msg.clone()).await;
                }
            }
        }
    }

    /// Broadcast a message to all active connections
    pub async fn broadcast(&self, msg: String) {
        let connections = self.connections.read().await;
        for conn in connections.values() {
            if let Some(sender) = &conn.sender {
                let _ = sender.send(msg.clone()).await;
            }
        }
    }

    /// Send a JSON-serializable value to a specific connection
    pub async fn send_json<T: serde::Serialize>(&self, id: &str, data: &T) -> Result<(), String> {
        let json_str = serde_json::to_string(data).map_err(|e| e.to_string())?;
        self.send(id, json_str).await.map_err(|e| e.to_string())
    }

    /// Broadcast a JSON-serializable value to all connections
    pub async fn broadcast_json<T: serde::Serialize>(&self, data: &T) -> Result<(), String> {
        let json_str = serde_json::to_string(data).map_err(|e| e.to_string())?;
        self.broadcast(json_str).await;
        Ok(())
    }

    // ── Rooms ───────────────────────────────────────────────

    /// Add a connection to a room
    pub async fn join_room(&self, conn_id: &str, room: &str) {
        self.rooms
            .write()
            .await
            .entry(room.to_string())
            .or_insert_with(HashSet::new)
            .insert(conn_id.to_string());
    }

    /// Remove a connection from a room
    pub async fn leave_room(&self, conn_id: &str, room: &str) {
        let mut rooms = self.rooms.write().await;
        if let Some(members) = rooms.get_mut(room) {
            members.remove(conn_id);
            if members.is_empty() {
                rooms.remove(room);
            }
        }
    }

    /// Send a message to all connections in a room
    pub async fn send_to_room(&self, room: &str, msg: String) {
        let rooms = self.rooms.read().await;
        if let Some(members) = rooms.get(room) {
            let connections = self.connections.read().await;
            for member_id in members {
                if let Some(conn) = connections.get(member_id) {
                    if let Some(sender) = &conn.sender {
                        let _ = sender.send(msg.clone()).await;
                    }
                }
            }
        }
    }

    /// Get all connection IDs in a room
    pub async fn get_room_members(&self, room: &str) -> Vec<String> {
        self.rooms
            .read()
            .await
            .get(room)
            .map(|m| m.iter().cloned().collect())
            .unwrap_or_default()
    }

    // ── Keepalive & Cleanup ─────────────────────────────────

    /// Start a background keepalive loop that pings connections and removes dead ones
    pub fn start_keepalive(self: Arc<Self>, interval_secs: u64) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));
            loop {
                interval.tick().await;

                let mut dead_ids = Vec::new();
                {
                    let connections = self.connections.read().await;
                    for (id, conn) in connections.iter() {
                        if let Some(sender) = &conn.sender {
                            if sender.send("__ping".to_string()).await.is_err() {
                                dead_ids.push(id.clone());
                            }
                        } else {
                            dead_ids.push(id.clone());
                        }
                    }
                }

                // Remove dead connections
                for id in &dead_ids {
                    tracing::info!(conn_id = %id, "Removing dead WebSocket connection");
                    self.unregister(id).await;
                }
            }
        })
    }

    // ── Rate Limiting ───────────────────────────────────────

    /// Check if a connection is within its rate limit.
    /// Returns `true` if allowed, `false` if rate-limited.
    /// Records the timestamp if allowed.
    pub async fn check_rate_limit(&self, id: &str, max_per_sec: u32) -> bool {
        let now = Instant::now();
        let mut limits = self.rate_limits.write().await;
        let timestamps = limits.entry(id.to_string()).or_insert_with(Vec::new);

        // Prune entries older than 1 second
        timestamps.retain(|t| now.duration_since(*t).as_secs_f64() < 1.0);

        if (timestamps.len() as u32) < max_per_sec {
            timestamps.push(now);
            true
        } else {
            false
        }
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
