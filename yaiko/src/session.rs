use crate::{Middleware, Request, Response};
use async_trait::async_trait;
use cookie::{Cookie, SameSite};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub data: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl Session {
    pub fn new(duration: chrono::Duration) -> Self {
        let now = chrono::Utc::now();
        Session {
            id: Uuid::new_v4().to_string(),
            data: HashMap::new(),
            created_at: now,
            expires_at: now + duration,
        }
    }

    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }

    pub fn set<T: Serialize>(&mut self, key: &str, value: T) -> Result<(), serde_json::Error> {
        self.data
            .insert(key.to_string(), serde_json::to_value(value)?);
        Ok(())
    }

    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.data.remove(key)
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn is_expired(&self) -> bool {
        chrono::Utc::now() > self.expires_at
    }

    pub fn renew_id(&mut self) -> String {
        let previous = self.id.clone();
        self.id = Uuid::new_v4().to_string();
        previous
    }

    pub fn extend(&mut self, duration: chrono::Duration) {
        self.expires_at = chrono::Utc::now() + duration;
    }
}

#[derive(Debug, Clone)]
pub struct SessionHandle {
    inner: Arc<Mutex<SessionState>>,
}

#[derive(Debug, Clone)]
struct SessionState {
    session: Session,
    destroy: bool,
    previous_ids: Vec<String>,
}

#[derive(Debug, Clone)]
struct PersistedSession {
    session: Session,
    destroy: bool,
    previous_ids: Vec<String>,
}

impl SessionHandle {
    pub fn new(session: Session) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SessionState {
                session,
                destroy: false,
                previous_ids: Vec::new(),
            })),
        }
    }

    pub fn id(&self) -> String {
        self.inner.lock().unwrap().session.id.clone()
    }

    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.inner.lock().unwrap().session.get(key)
    }

    pub fn set<T: Serialize>(&self, key: &str, value: T) -> Result<(), serde_json::Error> {
        self.inner.lock().unwrap().session.set(key, value)
    }

    pub fn remove(&self, key: &str) -> Option<serde_json::Value> {
        self.inner.lock().unwrap().session.remove(key)
    }

    pub fn clear(&self) {
        self.inner.lock().unwrap().session.clear();
    }

    pub fn destroy(&self) {
        let mut state = self.inner.lock().unwrap();
        state.destroy = true;
        state.session.clear();
    }

    pub fn rotate_id(&self) {
        let mut state = self.inner.lock().unwrap();
        let previous = state.session.renew_id();
        state.previous_ids.push(previous);
    }

    pub fn extend(&self, duration: chrono::Duration) {
        self.inner.lock().unwrap().session.extend(duration);
    }

    fn snapshot(&self) -> PersistedSession {
        let state = self.inner.lock().unwrap();
        PersistedSession {
            session: state.session.clone(),
            destroy: state.destroy,
            previous_ids: state.previous_ids.clone(),
        }
    }
}

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn get(
        &self,
        id: &str,
    ) -> Result<Option<Session>, Box<dyn std::error::Error + Send + Sync>>;
    async fn set(&self, session: Session) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn delete(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

pub struct MemorySessionStore {
    sessions: Arc<tokio::sync::RwLock<HashMap<String, Session>>>,
}

impl Default for MemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemorySessionStore {
    pub fn new() -> Self {
        MemorySessionStore {
            sessions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl SessionStore for MemorySessionStore {
    async fn get(
        &self,
        id: &str,
    ) -> Result<Option<Session>, Box<dyn std::error::Error + Send + Sync>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(id).cloned())
    }

    async fn set(&self, session: Session) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.id.clone(), session);
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(id);
        Ok(())
    }

    async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut sessions = self.sessions.write().await;
        let now = chrono::Utc::now();
        sessions.retain(|_, session| session.expires_at > now);
        Ok(())
    }
}

pub struct SessionMiddleware {
    store: Arc<dyn SessionStore>,
    cookie_name: String,
    session_duration: chrono::Duration,
    secure_cookies: Option<bool>,
    same_site: SameSite,
}

impl SessionMiddleware {
    pub fn new(store: Arc<dyn SessionStore>) -> Self {
        let cleanup_store = store.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                if let Err(error) = cleanup_store.cleanup().await {
                    tracing::error!("Session cleanup failed: {}", error);
                }
            }
        });

        SessionMiddleware {
            store,
            cookie_name: "yaiko_session".to_string(),
            session_duration: chrono::Duration::hours(24),
            secure_cookies: None,
            same_site: SameSite::Lax,
        }
    }

    pub fn cookie_name(mut self, name: &str) -> Self {
        self.cookie_name = name.to_string();
        self
    }

    pub fn duration(mut self, duration: chrono::Duration) -> Self {
        self.session_duration = duration;
        self
    }

    pub fn secure(mut self, secure: bool) -> Self {
        self.secure_cookies = Some(secure);
        self
    }

    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = same_site;
        self
    }

    fn cookie_is_secure(&self) -> bool {
        self.secure_cookies
            .unwrap_or_else(|| std::env::var("APP_ENV").unwrap_or_default() == "production")
    }

    fn session_cookie(&self, session: &Session) -> Cookie<'static> {
        Cookie::build(self.cookie_name.clone(), session.id.clone())
            .http_only(true)
            .secure(self.cookie_is_secure())
            .same_site(self.same_site)
            .path("/")
            .max_age(cookie::time::Duration::seconds(
                self.session_duration.num_seconds(),
            ))
            .finish()
    }

    fn expired_cookie(&self) -> Cookie<'static> {
        Cookie::build(self.cookie_name.clone(), "")
            .http_only(true)
            .secure(self.cookie_is_secure())
            .same_site(self.same_site)
            .path("/")
            .max_age(cookie::time::Duration::seconds(0))
            .finish()
    }
}

#[async_trait]
impl Middleware for SessionMiddleware {
    async fn handle(
        &self,
        mut req: Request,
        next: Arc<dyn crate::Handler>,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        let incoming_session_id = req.header("cookie").and_then(|cookie_header| {
            cookie_header.split(';').find_map(|cookie| {
                Cookie::parse(cookie.trim())
                    .ok()
                    .filter(|parsed| parsed.name() == self.cookie_name)
                    .map(|parsed| parsed.value().to_string())
            })
        });

        let loaded_session = if let Some(id) = incoming_session_id.as_deref() {
            match self.store.get(id).await? {
                Some(session) if !session.is_expired() => session,
                _ => Session::new(self.session_duration),
            }
        } else {
            Session::new(self.session_duration)
        };

        let handle = SessionHandle::new(loaded_session);
        req.session = Some(handle.clone());

        let mut response = next.handle(req).await?;
        let persisted = handle.snapshot();

        for previous_id in &persisted.previous_ids {
            self.store.delete(previous_id).await?;
        }

        if persisted.destroy {
            if let Some(original_id) = incoming_session_id.as_deref() {
                self.store.delete(original_id).await?;
            }
            self.store.delete(&persisted.session.id).await?;
            response = response.set_cookie_raw(&self.expired_cookie().to_string());
            return Ok(response);
        }

        self.store.set(persisted.session.clone()).await?;
        response = response.set_cookie_raw(&self.session_cookie(&persisted.session).to_string());
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatusCode;
    use hyper::Body;

    #[tokio::test]
    async fn session_mutations_persist_across_requests() {
        let store = Arc::new(MemorySessionStore::new());
        let middleware = SessionMiddleware::new(store.clone()).secure(false);

        let login = Arc::new(|req: Request| async move {
            let session = req.session.as_ref().unwrap();
            session.set("user_id", "abc123").unwrap();
            Ok(Response::new().text("ok"))
        });

        let first_req = Request::from_hyper(
            hyper::Request::builder()
                .method("GET")
                .uri("/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        let login_response = middleware.handle(first_req, login).await.unwrap();
        let set_cookie = login_response.headers.get("Set-Cookie").unwrap().clone();

        let me = Arc::new(|req: Request| async move {
            let user_id = req
                .session
                .as_ref()
                .and_then(|session| session.get::<String>("user_id"))
                .unwrap_or_default();
            Ok(Response::new().text(&user_id))
        });

        let second_req = Request::from_hyper(
            hyper::Request::builder()
                .method("GET")
                .uri("/me")
                .header("cookie", set_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        let second_response = middleware.handle(second_req, me).await.unwrap();
        let body = hyper::body::to_bytes(second_response.body).await.unwrap();

        assert_eq!(&body[..], b"abc123");
    }

    #[tokio::test]
    async fn destroyed_sessions_clear_cookie_and_store() {
        let store = Arc::new(MemorySessionStore::new());
        let middleware = SessionMiddleware::new(store.clone()).secure(false);

        let login = Arc::new(|req: Request| async move {
            req.session
                .as_ref()
                .unwrap()
                .set("user_id", "abc123")
                .unwrap();
            Ok(Response::new().status(StatusCode::OK))
        });
        let first_req = Request::from_hyper(
            hyper::Request::builder()
                .method("GET")
                .uri("/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        let login_response = middleware.handle(first_req, login).await.unwrap();
        let set_cookie = login_response.headers.get("Set-Cookie").unwrap().clone();

        let logout = Arc::new(|req: Request| async move {
            req.session.as_ref().unwrap().destroy();
            Ok(Response::new().status(StatusCode::OK))
        });
        let logout_req = Request::from_hyper(
            hyper::Request::builder()
                .method("POST")
                .uri("/logout")
                .header("cookie", &set_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        let logout_response = middleware.handle(logout_req, logout).await.unwrap();

        let cookie = logout_response.headers.get("Set-Cookie").unwrap();
        assert!(cookie.contains("Max-Age=0"));
    }
}
