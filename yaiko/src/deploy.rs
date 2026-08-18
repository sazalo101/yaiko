//! Deterministic deployment configuration facade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployTarget {
    Docker,
    Standalone,
    Serverless,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeployError {
    InvalidName,
    InvalidVersion,
    MissingEnvironment,
    Capacity,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploySpec {
    pub name: String,
    pub version: String,
    pub target: DeployTarget,
    pub required_env: Vec<String>,
}
#[derive(Debug, Clone, Default)]
pub struct DeployPlan {
    specs: Vec<DeploySpec>,
    capacity: usize,
}
impl DeployPlan {
    pub fn new(capacity: usize) -> Self {
        Self {
            specs: Vec::new(),
            capacity,
        }
    }
    pub fn add(
        mut self,
        name: impl Into<String>,
        version: impl Into<String>,
        target: DeployTarget,
        required_env: impl Into<Vec<String>>,
    ) -> Result<Self, DeployError> {
        let name = name.into();
        let version = version.into();
        if name.is_empty()
            || name.len() > 128
            || name.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(DeployError::InvalidName);
        }
        if version.is_empty()
            || version.len() > 64
            || version.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(DeployError::InvalidVersion);
        }
        if self.specs.len() >= self.capacity {
            return Err(DeployError::Capacity);
        }
        let mut env = required_env.into();
        env.sort();
        env.dedup();
        self.specs.push(DeploySpec {
            name,
            version,
            target,
            required_env: env,
        });
        self.specs.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(self)
    }
    pub fn validate_environment(
        &self,
        present: impl IntoIterator<Item = String>,
    ) -> Result<(), DeployError> {
        let present: std::collections::BTreeSet<_> = present.into_iter().collect();
        if self
            .specs
            .iter()
            .any(|s| s.required_env.iter().any(|e| !present.contains(e)))
        {
            Err(DeployError::MissingEnvironment)
        } else {
            Ok(())
        }
    }
    pub fn snapshot(&self) -> &[DeploySpec] {
        &self.specs
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sorts_specs_and_validates_environment() {
        let p = DeployPlan::new(2)
            .add("web", "1.0", DeployTarget::Docker, vec!["PORT".into()])
            .unwrap()
            .add("api", "1.0", DeployTarget::Standalone, Vec::new())
            .unwrap();
        assert_eq!(p.snapshot()[0].name, "api");
        assert!(p.validate_environment(vec!["PORT".into()]).is_ok())
    }
    #[test]
    fn rejects_bad_metadata_missing_environment_and_capacity() {
        assert!(DeployPlan::new(1)
            .add("bad name", "1", DeployTarget::Docker, Vec::new())
            .is_err());
        let p = DeployPlan::new(1)
            .add("app", "1", DeployTarget::Docker, vec!["SECRET".into()])
            .unwrap();
        assert!(p.validate_environment(Vec::<String>::new()).is_err());
        assert!(p
            .add("other", "1", DeployTarget::Docker, Vec::new())
            .is_err())
    }
}
