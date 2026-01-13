use yaiko_core::{
    App, Router, Server, Request, Response, StatusCode, json,
    LoggingMiddleware, SecurityHeaders, HealthCheck, init_tracing, tracing,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::net::SocketAddr;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Note model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create note request
#[derive(Debug, Deserialize)]
pub struct CreateNote {
    pub title: String,
    pub content: String,
}

/// Update note request
#[derive(Debug, Deserialize)]
pub struct UpdateNote {
    pub title: Option<String>,
    pub content: Option<String>,
}

/// In-memory note store
type NoteStore = Arc<RwLock<HashMap<String, Note>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    init_tracing();
    
    tracing::info!("Yaiko Notes - A simple note-taking app");
    
    // Initialize in-memory store with sample notes
    let store: NoteStore = Arc::new(RwLock::new(HashMap::new()));
    
    // Add sample notes
    {
        let mut notes = store.write().unwrap();
        let sample_notes = vec![
            ("Welcome to Yaiko Notes", "This is a simple note-taking application built with Yaiko framework."),
            ("Getting Started", "Create, edit, and delete notes using the interface below."),
            ("Features", "- Create new notes\n- Edit existing notes\n- Delete notes\n- All stored in memory"),
        ];
        
        for (title, content) in sample_notes {
            let id = Uuid::new_v4().to_string();
            notes.insert(id.clone(), Note {
                id,
                title: title.to_string(),
                content: content.to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });
        }
    }
    
    let store_clone = store.clone();
    let router = Router::new()
        // Health check endpoint
        .get("/health", HealthCheck::new())
        // Pages
        .get("/", {
            let store = store.clone();
            move |req| home_handler(req, store.clone())
        })
        // API
        .get("/api/notes", {
            let store = store.clone();
            move |req| list_notes(req, store.clone())
        })
        .get("/api/notes/:id", {
            let store = store.clone();
            move |req| get_note(req, store.clone())
        })
        .post("/api/notes", {
            let store = store.clone();
            move |req| create_note(req, store.clone())
        })
        .put("/api/notes/:id", {
            let store = store.clone();
            move |req| update_note(req, store.clone())
        })
        .delete("/api/notes/:id", {
            let store = store_clone;
            move |req| delete_note(req, store.clone())
        })
        // Static files
        .static_files("/static", "./public")
        // Middleware
        .use_middleware(LoggingMiddleware::new())
        .use_middleware(SecurityHeaders::new());
    
    let app = App::new().router(router);
    
    let addr: SocketAddr = "127.0.0.1:3000".parse()?;
    let server = Server::new(app, addr);
    
    tracing::info!("Server running at http://{}", addr);
    server.run().await?;
    
    Ok(())
}

/// Home page with full UI
async fn home_handler(_req: Request, _store: NoteStore) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let html = include_str!("../templates/index.html");
    Ok(Response::new().html(html))
}

/// List all notes
async fn list_notes(_req: Request, store: NoteStore) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let notes = store.read().unwrap();
    let mut list: Vec<&Note> = notes.values().collect();
    list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    
    Ok(Response::new().json(&json!({ "notes": list }))?)
}

/// Get a single note
async fn get_note(req: Request, store: NoteStore) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let id = req.param("id").map(|s| s.to_string()).unwrap_or_default();
    let notes = store.read().unwrap();
    
    match notes.get(&id) {
        Some(note) => Ok(Response::new().json(&json!({ "note": note }))?),
        None => Ok(Response::new()
            .status(StatusCode::NOT_FOUND)
            .json(&json!({ "error": "Note not found" }))?),
    }
}

/// Create a new note
async fn create_note(mut req: Request, store: NoteStore) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let value = req.json().await?;
    let body: CreateNote = serde_json::from_value(value)?;
    
    let note = Note {
        id: Uuid::new_v4().to_string(),
        title: body.title,
        content: body.content,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    let mut notes = store.write().unwrap();
    notes.insert(note.id.clone(), note.clone());
    
    Ok(Response::new()
        .status(StatusCode::CREATED)
        .json(&json!({ "note": note }))?)
}

/// Update an existing note
async fn update_note(mut req: Request, store: NoteStore) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let id = req.param("id").map(|s| s.to_string()).unwrap_or_default();
    let value = req.json().await?;
    let body: UpdateNote = serde_json::from_value(value)?;
    
    let mut notes = store.write().unwrap();
    
    match notes.get_mut(&id) {
        Some(note) => {
            if let Some(title) = body.title {
                note.title = title;
            }
            if let Some(content) = body.content {
                note.content = content;
            }
            note.updated_at = Utc::now();
            
            let updated = note.clone();
            Ok(Response::new().json(&json!({ "note": updated }))?)
        }
        None => Ok(Response::new()
            .status(StatusCode::NOT_FOUND)
            .json(&json!({ "error": "Note not found" }))?),
    }
}

/// Delete a note
async fn delete_note(req: Request, store: NoteStore) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let id = req.param("id").map(|s| s.to_string()).unwrap_or_default();
    
    let mut notes = store.write().unwrap();
    
    match notes.remove(&id) {
        Some(_) => Ok(Response::new().json(&json!({ "message": "Note deleted" }))?),
        None => Ok(Response::new()
            .status(StatusCode::NOT_FOUND)
            .json(&json!({ "error": "Note not found" }))?),
    }
}
