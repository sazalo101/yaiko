use yaiko_core::{Request, Response, BoxError, StatusCode, json, is_websocket_upgrade};
use yaiko_core::websocket::{handle_websocket_upgrade, WsMessage};
use yaiko_core::WebSocketManager;
use sqlx::{SqlitePool, Row};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;
use uuid::Uuid;

pub struct AppState {
    pub ws_manager: Arc<WebSocketManager>,
    pub db_pool: SqlitePool,
}

use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::{WebSocketStream, tungstenite::protocol::Message as TMessage};

pub async fn chat_ws_handler(mut req: Request, state: Arc<AppState>) -> Result<Response, BoxError> {
    let user_id = match req.session.as_ref().and_then(|s| s.get::<String>("user_id")) {
        Some(id) => id,
        None => return Ok(Response::new().status(StatusCode::UNAUTHORIZED).json(&json!({"error": "Unauthorized"}))?)
    };
    let username = req.session.as_ref().and_then(|s| s.get::<String>("username")).unwrap_or_default();

    if !is_websocket_upgrade(&req) {
        return Ok(Response::new().status(StatusCode::BAD_REQUEST).text("Expected WebSocket upgrade"));
    }

    let manager = state.ws_manager.clone();
    let (response, conn_id, mut rx) = handle_websocket_upgrade(&req, manager.clone(), Some(user_id.clone())).await?;
    
    // Perform actual upgrade in a spawned task
    let (mut parts, _) = yaiko_core::hyper::Request::builder()
        .method(req.method.clone())
        .uri(req.uri.clone())
        .body(())
        .unwrap()
        .into_parts();
    parts.headers = req.headers.clone();
    parts.extensions = std::mem::take(&mut req.extensions);
    
    let hyper_req = yaiko_core::hyper::Request::from_parts(parts, yaiko_core::hyper::Body::empty());
    let db = state.db_pool.clone();
    let manager_clone = manager.clone();
    let conn_id_clone = conn_id.clone();
    let uname = username.clone();
    let uid = user_id.clone();

    tokio::spawn(async move {
        match yaiko_core::hyper::upgrade::on(hyper_req).await {
            Ok(upgraded) => {
                let mut ws_stream: WebSocketStream<yaiko_core::hyper::upgrade::Upgraded> = WebSocketStream::from_raw_socket(
                    upgraded,
                    tokio_tungstenite::tungstenite::protocol::Role::Server,
                    None,
                ).await;
                tracing::info!(conn_id = %conn_id_clone, "WebSocket upgraded successfully");

                loop {
                    tokio::select! {
                        // Outbound: From Manager to WebSocket
                        Some(msg) = rx.recv() => {
                            let tmsg = TMessage::Text(msg);
                            if ws_stream.send(tmsg).await.is_err() {
                                break;
                            }
                        }
                        // Inbound: From WebSocket to Manager/DB
                        Some(result) = ws_stream.next() => {
                            match result {
                                Ok(TMessage::Text(msg)) => {
                                    handle_incoming_text(&conn_id_clone, &uid, &uname, msg, &manager_clone, &db).await;
                                }
                                Ok(TMessage::Close(_)) => break,
                                Err(_) => break,
                                _ => {}
                            }
                        }
                        else => break,
                    }
                }
                manager_clone.unregister(&conn_id_clone).await;
            }
            Err(e) => {
                tracing::error!("WebSocket upgrade failed: {}", e);
            }
        }
    });

    // Automatically put user in global 'presence' room
    manager.join_room(&conn_id, "presence").await;
    
    Ok(response)
}

async fn handle_incoming_text(conn_id: &str, uid: &str, uname: &str, msg: String, manager: &Arc<WebSocketManager>, db: &SqlitePool) {
    // Rate limit to 5 messages per second
    if !manager.check_rate_limit(conn_id, 5).await {
        let _ = manager.send(conn_id, json!({"error": "Rate limit exceeded"}).to_string()).await;
        return;
    }

    if let Ok(data) = serde_json::from_str::<Value>(&msg) {
        let msg_type = data["type"].as_str().unwrap_or("");
        let room = data["room"].as_str().unwrap_or("general");

        match msg_type {
            "join" => {
                manager.join_room(conn_id, room).await;
                manager.send_to_room(
                    room,
                    json!({
                        "type": "system",
                        "room": room,
                        "content": format!("{} joined", uname)
                    }).to_string()
                ).await;
            }
            "typing" => {
                manager.send_to_room(
                    room,
                    json!({
                        "type": "typing",
                        "room": room,
                        "username": uname
                    }).to_string()
                ).await;
            }
            "message" => {
                let content = data["content"].as_str().unwrap_or("");
                if content.trim().is_empty() { return; }

                let attachment_url = data["attachment_url"].as_str();
                let msg_id = Uuid::new_v4().to_string();

                // Save to DB
                let _ = sqlx::query("INSERT INTO messages (id, room_id, user_id, content, attachment_url) VALUES (?, ?, ?, ?, ?)")
                    .bind(&msg_id)
                    .bind(room)
                    .bind(uid)
                    .bind(content)
                    .bind(attachment_url)
                    .execute(db)
                    .await;

                // Broadcast
                manager.send_to_room(
                    room,
                    json!({
                        "type": "message",
                        "room": room,
                        "id": msg_id,
                        "user_id": uid,
                        "username": uname,
                        "content": content,
                        "attachment_url": attachment_url
                    }).to_string()
                ).await;
            }
            _ => {}
        }
    }
}

// Endpoint to fetch history
pub async fn get_history(req: Request, state: Arc<AppState>) -> Result<Response, BoxError> {
    let room = req.param("room").map(|s| s.to_string()).unwrap_or("general".into());
    
    let rows = sqlx::query("SELECT m.id, m.content, m.attachment_url, m.created_at, u.username, u.id as user_id FROM messages m JOIN users u ON m.user_id = u.id WHERE m.room_id = ? ORDER BY m.created_at ASC LIMIT 100")
        .bind(&room)
        .fetch_all(&state.db_pool)
        .await?;

    let mut history = vec![];
    for r in rows {
        let content: String = r.get("content");
        let username: String = r.get("username");
        let user_id: String = r.get("user_id");
        let attachment_url: Option<String> = r.try_get("attachment_url").unwrap_or(None);
        
        history.push(json!({
            "type": "message",
            "room": room,
            "username": username,
            "user_id": user_id,
            "content": content,
            "attachment_url": attachment_url
        }));
    }

    Ok(Response::new().json(&history)?)
}
