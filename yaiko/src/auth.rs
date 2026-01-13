use crate::{Request, Response, Middleware, Handler};
use async_trait::async_trait;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// bcrypt is available for password hashing when needed:
// use bcrypt::{hash, verify, DEFAULT_COST};

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

    pub fn generate_token(&self, user_id: &str, roles: Vec<String>) -> Result<String, jsonwebtoken::errors::Error> {
        let now = chrono::Utc::now();
        let exp = now + chrono::Duration::hours(24);
        
        let claims = Claims {
            sub: user_id.to_string(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
            roles,
        };

        encode(&Header::default(), &claims, &EncodingKey::from_secret(self.secret.as_ref()))
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &Validation::default(),
        ).map(|data| data.claims)
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
        // Skip authentication for certain paths
        if self.skip_paths.contains(&req.uri.path().to_string()) {
            return next.handle(req).await;
        }

        // Extract JWT token from Authorization header
        let token = req.headers
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
                Err(_) => {
                    Ok(Response::new()
                        .status(hyper::StatusCode::UNAUTHORIZED)
                        .json(&serde_json::json!({"error": "Invalid token"}))?)
                }
            }
        } else {
            Ok(Response::new()
                .status(hyper::StatusCode::UNAUTHORIZED)
                .json(&serde_json::json!({"error": "Missing token"}))?)
        }
    }
}