//! Tenant context and isolation primitives for multi-tenant applications.

use crate::{MemoryRateLimiter, QuotaPolicy, RateLimitDecision, Request};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TenantId(String);

impl TenantId {
    pub fn new(value: impl Into<String>) -> Result<Self, TenantError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || value
                .chars()
                .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.')))
        {
            return Err(TenantError::InvalidTenantId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantContext {
    pub tenant_id: TenantId,
    pub user_id: Option<String>,
}

impl TenantContext {
    pub fn new(tenant_id: TenantId) -> Self {
        Self {
            tenant_id,
            user_id: None,
        }
    }

    pub fn user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn scoped_key(&self, namespace: &str, key: &str) -> String {
        format!("tenant:{}:{}:{}", self.tenant_id, namespace, key)
    }

    pub fn permits(&self, requested: &TenantId) -> bool {
        &self.tenant_id == requested
    }

    pub fn require(&self, requested: &TenantId) -> Result<(), TenantError> {
        if self.permits(requested) {
            Ok(())
        } else {
            Err(TenantError::CrossTenantAccess)
        }
    }
}

pub struct TenantResolver {
    header_name: String,
}

impl TenantResolver {
    pub fn from_header(header_name: impl Into<String>) -> Self {
        Self {
            header_name: header_name.into(),
        }
    }

    pub fn resolve(&self, request: &Request) -> Result<TenantContext, TenantError> {
        let raw = request
            .header(&self.header_name)
            .ok_or(TenantError::MissingTenant)?;
        Ok(TenantContext::new(TenantId::new(raw.to_string())?))
    }
}

impl Default for TenantResolver {
    fn default() -> Self {
        Self::from_header("x-tenant-id")
    }
}

#[derive(Clone)]
pub struct TenantQuota {
    limiter: MemoryRateLimiter,
}

impl TenantQuota {
    pub fn new(policy: QuotaPolicy) -> Self {
        Self {
            limiter: MemoryRateLimiter::new(policy),
        }
    }

    pub fn check(&self, tenant: &TenantId, cost: u32) -> RateLimitDecision {
        self.limiter.check(format!("tenant:{}", tenant), cost)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TenantError {
    InvalidTenantId,
    MissingTenant,
    CrossTenantAccess,
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::{Body, Method, Request as HyperRequest};
    use std::time::Instant;

    #[test]
    fn scopes_keys_and_rejects_cross_tenant_access() {
        let tenant_a = TenantId::new("acme").unwrap();
        let tenant_b = TenantId::new("globex").unwrap();
        let context = TenantContext::new(tenant_a.clone()).user("user-1");
        assert_eq!(
            context.scoped_key("cache", "profile"),
            "tenant:acme:cache:profile"
        );
        assert!(context.require(&tenant_a).is_ok());
        assert_eq!(
            context.require(&tenant_b),
            Err(TenantError::CrossTenantAccess)
        );
    }

    #[test]
    fn tenant_quota_isolated_by_tenant() {
        let quota = TenantQuota::new(QuotaPolicy::new(1, 1.0));
        let first = TenantId::new("acme").unwrap();
        let second = TenantId::new("globex").unwrap();
        assert!(quota.check(&first, 1).allowed);
        assert!(!quota.check(&first, 1).allowed);
        assert!(quota.check(&second, 1).allowed);
    }

    #[tokio::test]
    async fn resolver_validates_header_tenant() {
        let request = Request::from_hyper(
            HyperRequest::builder()
                .method(Method::GET)
                .uri("/")
                .header("x-tenant-id", "acme")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        let context = TenantResolver::default().resolve(&request).unwrap();
        assert_eq!(context.tenant_id.as_str(), "acme");
        assert!(TenantId::new("bad tenant").is_err());
        let _ = Instant::now();
    }
}
