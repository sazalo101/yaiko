//! Safe reverse-proxy request policy primitives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyError {
    InvalidUpstream,
    MethodNotAllowed,
    BodyTooLarge,
    InvalidHeader,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyRequest {
    pub method: ProxyMethod,
    pub upstream: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}
#[derive(Debug, Clone)]
pub struct ProxyPolicy {
    pub upstream: String,
    pub max_body: usize,
    allowed: Vec<ProxyMethod>,
}
impl ProxyPolicy {
    pub fn new(upstream: impl Into<String>, max_body: usize) -> Result<Self, ProxyError> {
        let upstream = upstream.into();
        if !upstream.starts_with("https://")
            || upstream
                .chars()
                .any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(ProxyError::InvalidUpstream);
        }
        Ok(Self {
            upstream,
            max_body,
            allowed: vec![ProxyMethod::Get],
        })
    }
    pub fn allow(mut self, method: ProxyMethod) -> Self {
        if !self.allowed.contains(&method) {
            self.allowed.push(method)
        }
        self
    }
    pub fn build(
        &self,
        method: ProxyMethod,
        path: impl Into<String>,
        headers: impl IntoIterator<Item = (String, String)>,
        body: impl Into<Vec<u8>>,
    ) -> Result<ProxyRequest, ProxyError> {
        if !self.allowed.contains(&method) {
            return Err(ProxyError::MethodNotAllowed);
        }
        let path = path.into();
        if !path.starts_with('/')
            || path.contains("..")
            || path.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(ProxyError::InvalidUpstream);
        }
        let body = body.into();
        if body.len() > self.max_body {
            return Err(ProxyError::BodyTooLarge);
        }
        let headers = headers
            .into_iter()
            .filter_map(|(k, v)| {
                let lk = k.to_ascii_lowercase();
                if matches!(
                    lk.as_str(),
                    "host" | "connection" | "content-length" | "x-forwarded-for"
                ) || k.is_empty()
                    || k.chars().any(|c| c.is_control() || c.is_whitespace())
                    || v.chars().any(|c| c.is_control())
                {
                    None
                } else {
                    Some((k, v))
                }
            })
            .collect();
        Ok(ProxyRequest {
            method,
            upstream: self.upstream.clone(),
            path,
            headers,
            body,
        })
    }
    pub fn describe(&self) -> String {
        format!("{} (max_body={})", self.upstream, self.max_body)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_safe_forward_request() {
        let p = ProxyPolicy::new("https://api.example.com", 8)
            .unwrap()
            .allow(ProxyMethod::Post);
        let r = p
            .build(
                ProxyMethod::Post,
                "/v1/media",
                [
                    ("authorization".into(), "Bearer x".into()),
                    ("host".into(), "bad".into()),
                ],
                b"x".to_vec(),
            )
            .unwrap();
        assert_eq!(r.headers.len(), 1);
        assert_eq!(p.describe(), "https://api.example.com (max_body=8)")
    }
    #[test]
    fn rejects_unsafe_upstream_method_path_and_body() {
        assert!(ProxyPolicy::new("http://bad", 1).is_err());
        let p = ProxyPolicy::new("https://api.example.com", 1).unwrap();
        assert!(p
            .build(ProxyMethod::Post, "/x", Vec::new(), Vec::new())
            .is_err());
        assert!(p
            .build(ProxyMethod::Get, "/../x", Vec::new(), Vec::new())
            .is_err());
        assert!(p
            .build(ProxyMethod::Get, "/x", Vec::new(), b"xx".to_vec())
            .is_err())
    }
}
