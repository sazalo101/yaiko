//! Filesystem-style route matching without filesystem access.
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsRouteError {
    InvalidPattern,
    DuplicatePattern,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsMatch {
    pub target: String,
    pub params: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Default)]
pub struct FsRouter {
    routes: Vec<(String, String)>,
}
impl FsRouter {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add(
        mut self,
        pattern: impl Into<String>,
        target: impl Into<String>,
    ) -> Result<Self, FsRouteError> {
        let pattern = pattern.into();
        if !valid_pattern(&pattern) || self.routes.iter().any(|(p, _)| p == &pattern) {
            return Err(if self.routes.iter().any(|(p, _)| p == &pattern) {
                FsRouteError::DuplicatePattern
            } else {
                FsRouteError::InvalidPattern
            });
        }
        self.routes.push((pattern, target.into()));
        self.routes
            .sort_by_key(|(p, _)| std::cmp::Reverse(score(p)));
        Ok(self)
    }
    pub fn match_path(&self, path: &str) -> Option<FsMatch> {
        if !path.starts_with('/')
            || path.contains("..")
            || path.chars().any(|c| c.is_whitespace() || c.is_control())
        {
            return None;
        }
        for (pattern, target) in &self.routes {
            let mut params = BTreeMap::new();
            let p = pattern
                .trim_matches('/')
                .split('/')
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>();
            let actual = path
                .trim_matches('/')
                .split('/')
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>();
            if p.len() != actual.len() {
                continue;
            }
            let mut ok = true;
            for (a, b) in p.iter().zip(actual.iter()) {
                if a.starts_with('[') && a.ends_with(']') {
                    params.insert(a[1..a.len() - 1].to_string(), (*b).to_string());
                } else if a != b {
                    ok = false;
                    break;
                }
            }
            if ok {
                return Some(FsMatch {
                    target: target.clone(),
                    params,
                });
            }
        }
        None
    }
}
fn valid_pattern(p: &str) -> bool {
    p.starts_with('/')
        && !p.contains("..")
        && !p.chars().any(|c| c.is_control() || c.is_whitespace())
}
fn score(p: &str) -> usize {
    p.split('/')
        .filter(|s| !s.is_empty() && !s.starts_with('['))
        .count()
        * 10
        + p.split('/').count()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_static_before_dynamic_and_extracts_params() {
        let r = FsRouter::new()
            .add("/users/[id]", "user")
            .unwrap()
            .add("/users/me", "me")
            .unwrap();
        assert_eq!(r.match_path("/users/me").unwrap().target, "me");
        let m = r.match_path("/users/42").unwrap();
        assert_eq!(m.params["id"], "42")
    }
    #[test]
    fn rejects_bad_patterns_and_paths() {
        assert!(FsRouter::new().add("users/[id]", "x").is_err());
        assert!(FsRouter::new()
            .add("/x", "a")
            .unwrap()
            .match_path("/../x")
            .is_none())
    }
}
