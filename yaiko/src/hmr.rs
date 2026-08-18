//! Deterministic hot-module-reload policy metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadMode {
    Stylesheet,
    Module,
    Full,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HmrError {
    InvalidPath,
    InvalidVersion,
    Capacity,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HmrAsset {
    pub path: String,
    pub mode: ReloadMode,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HmrEvent {
    pub path: String,
    pub version: u64,
    pub mode: ReloadMode,
}
#[derive(Debug, Clone, Default)]
pub struct HmrPolicy {
    assets: Vec<HmrAsset>,
    capacity: usize,
}
impl HmrPolicy {
    pub fn new(capacity: usize) -> Self {
        Self {
            assets: Vec::new(),
            capacity,
        }
    }
    pub fn asset(mut self, path: impl Into<String>, mode: ReloadMode) -> Result<Self, HmrError> {
        let path = path.into();
        if path.is_empty()
            || !path.starts_with('/')
            || path.contains("..")
            || path.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(HmrError::InvalidPath);
        }
        if self.assets.len() >= self.capacity {
            return Err(HmrError::Capacity);
        }
        self.assets.push(HmrAsset { path, mode });
        self.assets.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(self)
    }
    pub fn event(&self, path: &str, version: u64) -> Result<HmrEvent, HmrError> {
        if version == 0 {
            return Err(HmrError::InvalidVersion);
        }
        self.assets
            .iter()
            .find(|a| a.path == path)
            .map(|a| HmrEvent {
                path: path.into(),
                version,
                mode: a.mode,
            })
            .ok_or(HmrError::InvalidPath)
    }
    pub fn snapshot(&self) -> &[HmrAsset] {
        &self.assets
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registers_and_builds_events() {
        let h = HmrPolicy::new(2)
            .asset("/app.css", ReloadMode::Stylesheet)
            .unwrap()
            .asset("/main.js", ReloadMode::Module)
            .unwrap();
        let e = h.event("/main.js", 2).unwrap();
        assert_eq!(e.mode, ReloadMode::Module);
        assert_eq!(h.snapshot()[0].path, "/app.css")
    }
    #[test]
    fn validates_paths_versions_and_capacity() {
        assert!(HmrPolicy::new(1)
            .asset("../x.js", ReloadMode::Full)
            .is_err());
        let h = HmrPolicy::new(1).asset("/x.js", ReloadMode::Full).unwrap();
        assert!(h.event("/x.js", 0).is_err());
        assert!(h.asset("/y.js", ReloadMode::Full).is_err())
    }
}
