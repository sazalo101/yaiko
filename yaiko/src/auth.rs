use crate::{Handler, Middleware, Request, Response};
use crate::session::SessionHandle;
use async_trait::async_trait;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub roles: Vec<String>,
}

#[allow(dead_code)]
pub struct JwtAuth {
    secret: String,
    algorithm: jsonwebtoken::Algorithm,
}

impl JwtAuth {
    pub fn new(secret: &str) -> Self {
        JwtAuth {
            secret: secret.to_string(),
            algorithm: jsonwebtoken::Algorithm::HS256,
        }
    }

    pub fn generate_token(
        &self,
        user_id: &str,
        roles: Vec<String>,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let now = chrono::Utc::now();
        let exp = now + chrono::Duration::hours(24);

        let claims = Claims {
            sub: user_id.to_string(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
            roles,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let mut validation = Validation::new(self.algorithm);
        validation.validate_exp = true;
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &validation,
        )
        .map(|data| data.claims)
    }
}

pub struct AuthMiddleware {
    jwt: Arc<JwtAuth>,
    skip_paths: Vec<String>,
}

impl AuthMiddleware {
    pub fn new(jwt: Arc<JwtAuth>) -> Self {
        AuthMiddleware {
            jwt,
            skip_paths: vec!["/login".to_string(), "/register".to_string()],
        }
    }

    pub fn skip_path(mut self, path: &str) -> Self {
        self.skip_paths.push(path.to_string());
        self
    }
}

#[async_trait]
impl Middleware for AuthMiddleware {
    async fn handle(
        &self,
        mut req: Request,
        next: Arc<dyn Handler>,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        if self.skip_paths.contains(&req.uri.path().to_string()) {
            return next.handle(req).await;
        }

        let token = req
            .headers
            .get("authorization")
            .and_then(|auth| auth.to_str().ok())
            .and_then(|auth| auth.strip_prefix("Bearer "));

        if let Some(token) = token {
            match self.jwt.verify_token(token) {
                Ok(claims) => {
                    req.user_id = Some(claims.sub);
                    req.user_roles = claims.roles;
                    next.handle(req).await
                }
                Err(_) => Ok(unauthorized_json("Invalid token")?),
            }
        } else {
            Ok(unauthorized_json("Missing token")?)
        }
    }
}

pub struct SessionAuth {
    skip_paths: Vec<String>,
    user_id_key: String,
    roles_key: String,
    optional: bool,
}

impl SessionAuth {
    pub fn new() -> Self {
        Self {
            skip_paths: vec!["/login".to_string(), "/register".to_string()],
            user_id_key: "user_id".to_string(),
            roles_key: "roles".to_string(),
            optional: false,
        }
    }

    pub fn skip_path(mut self, path: &str) -> Self {
        self.skip_paths.push(path.to_string());
        self
    }

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    pub fn user_id_key(mut self, key: &str) -> Self {
        self.user_id_key = key.to_string();
        self
    }

    pub fn roles_key(mut self, key: &str) -> Self {
        self.roles_key = key.to_string();
        self
    }
}

impl Default for SessionAuth {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for SessionAuth {
    async fn handle(
        &self,
        mut req: Request,
        next: Arc<dyn Handler>,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        if self.skip_paths.iter().any(|path| path == req.uri.path()) {
            return next.handle(req).await;
        }

        if let Some(session) = &req.session {
            if let Some(user_id) = session.get::<String>(&self.user_id_key) {
                req.user_id = Some(user_id);
                req.user_roles = session
                    .get::<Vec<String>>(&self.roles_key)
                    .unwrap_or_default();
                return next.handle(req).await;
            }
        }

        if self.optional {
            return next.handle(req).await;
        }

        Ok(unauthorized_json("Authentication required")?)
    }
}

pub fn login_session(
    session: &SessionHandle,
    user_id: &str,
    roles: &[String],
) -> Result<(), serde_json::Error> {
    session.rotate_id();
    session.set("user_id", user_id)?;
    session.set("roles", roles)?;
    Ok(())
}

pub fn logout_session(session: &SessionHandle) {
    session.destroy();
}

pub fn require_role(req: &Request, role: &str) -> Result<(), Response> {
    if req.user_roles.iter().any(|existing| existing == role) {
        Ok(())
    } else {
        Err(
            Response::new()
                .status(hyper::StatusCode::FORBIDDEN)
                .json(&serde_json::json!({ "error": "Forbidden" }))
                .expect("failed to serialize forbidden response"),
        )
    }
}

fn unauthorized_json(message: &str) -> Result<Response, serde_json::Error> {
    Response::new()
        .status(hyper::StatusCode::UNAUTHORIZED)
        .json(&serde_json::json!({ "error": message }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemorySessionStore, Request, SessionMiddleware, StatusCode};
    use hyper::Body;

    #[tokio::test]
    async fn session_auth_sets_user_context_and_allows_access() {
        let store = Arc::new(MemorySessionStore::new());
        let session_middleware = SessionMiddleware::new(store).secure(false);
        let auth_middleware = Arc::new(SessionAuth::new());

        let login_handler = Arc::new(|req: Request| async move {
            let session = req.session.as_ref().unwrap();
            login_session(session, "user-1", &["admin".to_string()]).unwrap();
            Ok(Response::new().status(StatusCode::OK))
        });
        let login_req = Request::from_hyper(
            hyper::Request::builder()
                .method("GET")
                .uri("/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        let login_response = session_middleware.handle(login_req, login_handler).await.unwrap();
        let cookie = login_response.headers.get("Set-Cookie").unwrap().clone();

        let protected_handler = Arc::new(|req: Request| async move {
            Ok(Response::new().text(req.user_id.as_deref().unwrap_or("missing")))
        });
        let protected_req = Request::from_hyper(
            hyper::Request::builder()
                .method("GET")
                .uri("/dashboard")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        let auth = auth_middleware.clone();
        let protected = protected_handler.clone();
        let hydrated = session_middleware
            .handle(protected_req, Arc::new(move |req: Request| {
                let auth = auth.clone();
                let protected = protected.clone();
                async move { auth.handle(req, protected).await }
            }))
            .await
            .unwrap();

        let body = hyper::body::to_bytes(hydrated.body).await.unwrap();
        assert_eq!(&body[..], b"user-1");
    }

    #[tokio::test]
    async fn session_auth_rejects_missing_session() {
        let auth = SessionAuth::new();
        let next = Arc::new(|_req: Request| async move { Ok(Response::new().text("ok")) });
        let req = Request::from_hyper(
            hyper::Request::builder()
                .method("GET")
                .uri("/protected")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        let response = auth.handle(req, next).await.unwrap();
        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }
}
