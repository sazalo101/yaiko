//! Scoped role-based access control primitives.
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RbacError {
    InvalidName,
    DuplicateRole,
    MissingRole,
    Cycle,
    InvalidPermission,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    pub name: String,
    pub permissions: BTreeSet<String>,
    pub inherits: BTreeSet<String>,
}
#[derive(Debug, Clone, Default)]
pub struct RbacPolicy {
    roles: BTreeMap<String, Role>,
}
impl RbacPolicy {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_role(
        &mut self,
        name: impl Into<String>,
        permissions: impl IntoIterator<Item = impl Into<String>>,
        inherits: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<(), RbacError> {
        let name = valid(name.into())?;
        if self.roles.contains_key(&name) {
            return Err(RbacError::DuplicateRole);
        }
        let permissions = permissions
            .into_iter()
            .map(|p| valid_permission(p.into()))
            .collect::<Result<BTreeSet<_>, _>>()?;
        let inherits = inherits
            .into_iter()
            .map(|p| valid(p.into()))
            .collect::<Result<BTreeSet<_>, _>>()?;
        if inherits.contains(&name) {
            return Err(RbacError::Cycle);
        }
        if inherits.iter().any(|r| !self.roles.contains_key(r)) {
            return Err(RbacError::MissingRole);
        }
        self.roles.insert(
            name.clone(),
            Role {
                name,
                permissions,
                inherits,
            },
        );
        Ok(())
    }
    pub fn allows(
        &self,
        roles: impl IntoIterator<Item = impl AsRef<str>>,
        permission: &str,
    ) -> Result<bool, RbacError> {
        let permission = valid_permission(permission.to_string())?;
        let mut seen = BTreeSet::new();
        for role in roles {
            if self.has(role.as_ref(), &permission, &mut seen)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn has(
        &self,
        name: &str,
        permission: &str,
        seen: &mut BTreeSet<String>,
    ) -> Result<bool, RbacError> {
        if !seen.insert(name.into()) {
            return Err(RbacError::Cycle);
        }
        let r = self.roles.get(name).ok_or(RbacError::MissingRole)?;
        if r.permissions.contains(permission) || r.permissions.contains("*") {
            return Ok(true);
        }
        for parent in &r.inherits {
            if self.has(parent, permission, seen)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
fn valid(v: String) -> Result<String, RbacError> {
    if v.is_empty() || v.len() > 64 || v.chars().any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        Err(RbacError::InvalidName)
    } else {
        Ok(v)
    }
}
fn valid_permission(v: String) -> Result<String, RbacError> {
    if v.is_empty() || v.len() > 128 || v.chars().any(|c| c.is_control() || c.is_whitespace()) {
        Err(RbacError::InvalidPermission)
    } else {
        Ok(v)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inherits_and_matches_permissions() {
        let mut p = RbacPolicy::new();
        p.add_role("viewer", ["media.read"], [] as [&str; 0])
            .unwrap();
        p.add_role("editor", ["media.write"], ["viewer"]).unwrap();
        assert!(p.allows(["editor"], "media.read").unwrap());
        assert!(p.allows(["editor"], "media.write").unwrap());
        assert!(!p.allows(["viewer"], "media.write").unwrap());
    }
    #[test]
    fn validates_missing_roles_and_cycles() {
        let mut p = RbacPolicy::new();
        assert_eq!(
            p.add_role("editor", ["media.write"], ["missing"]),
            Err(RbacError::MissingRole)
        );
        p.add_role("admin", ["*"], [] as [&str; 0]).unwrap();
        assert!(p.allows(["admin"], "anything").unwrap());
        assert_eq!(p.allows(["missing"], "x"), Err(RbacError::MissingRole));
    }
}
