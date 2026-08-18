//! Deterministic URL and path helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlError {
    InvalidPath,
    InvalidQuery,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    pub path: String,
    pub query: Vec<(String, String)>,
}
impl Url {
    pub fn new(path: impl Into<String>) -> Result<Self, UrlError> {
        let path = normalize(path.into())?;
        Ok(Self {
            path,
            query: Vec::new(),
        })
    }
    pub fn query(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, UrlError> {
        let k = key.into();
        let v = value.into();
        if k.is_empty() || k.chars().any(|c| c.is_control()) || v.chars().any(|c| c.is_control()) {
            return Err(UrlError::InvalidQuery);
        }
        self.query.push((k, v));
        self.query.sort();
        Ok(self)
    }
    pub fn join(&self, segment: impl Into<String>) -> Result<Self, UrlError> {
        let segment = segment.into();
        if segment.split('/').any(|p| p == ".." || p == ".") {
            return Err(UrlError::InvalidPath);
        }
        let path = format!(
            "{}/{}",
            self.path.trim_end_matches('/'),
            segment.trim_matches('/')
        );
        Url::new(path)
    }
    pub fn render(&self) -> String {
        let mut out = self.path.clone();
        if !self.query.is_empty() {
            out.push('?');
            out.push_str(
                &self
                    .query
                    .iter()
                    .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
                    .collect::<Vec<_>>()
                    .join("&"),
            )
        }
        out
    }
}
fn normalize(mut p: String) -> Result<String, UrlError> {
    if p.is_empty()
        || !p.starts_with('/')
        || p.contains('\0')
        || p.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(UrlError::InvalidPath);
    }
    let mut parts = Vec::new();
    for part in p.split('/') {
        match part {
            "" | "." => {}
            ".." => return Err(UrlError::InvalidPath),
            x => parts.push(x),
        }
    }
    p = format!("/{}", parts.join("/"));
    Ok(p)
}
fn encode(x: &str) -> String {
    x.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{:02X}", b)
            }
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalizes_and_encodes_deterministically() {
        let u = Url::new("//editor//timeline/")
            .unwrap()
            .query("z", "hello world")
            .unwrap()
            .query("a", "x&y")
            .unwrap();
        assert_eq!(u.render(), "/editor/timeline?a=x%26y&z=hello%20world");
        assert_eq!(
            u.join("captions").unwrap().render(),
            "/editor/timeline/captions"
        )
    }
    #[test]
    fn rejects_traversal_and_unsafe_values() {
        assert_eq!(Url::new("/../secret"), Err(UrlError::InvalidPath));
        assert!(Url::new("/x").unwrap().join("../bad").is_err());
        assert!(Url::new("/x").unwrap().query("k", "bad\n").is_err())
    }
}
