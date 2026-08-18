//! Reusable, scoped project templates for media-editor timelines.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateError {
    InvalidId,
    InvalidScope,
    InvalidDefinition,
    Missing,
    Duplicate,
    Capacity,
    RevisionConflict,
    ScopeMismatch,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTemplate {
    pub id: String,
    pub scope: String,
    pub version: u64,
    pub definition: String,
    pub placeholders: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializedTemplate {
    pub template_id: String,
    pub version: u64,
    pub definition: String,
}
#[derive(Debug, Clone)]
pub struct ProjectTemplateStore {
    inner: Arc<Mutex<HashMap<String, ProjectTemplate>>>,
    max_entries: usize,
}
impl ProjectTemplateStore {
    pub fn new(max_entries: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_entries: max_entries.max(1),
        }
    }
    pub fn create(
        &self,
        id: impl Into<String>,
        scope: impl Into<String>,
        definition: impl Into<String>,
        placeholders: Vec<String>,
    ) -> Result<ProjectTemplate, TemplateError> {
        let id = valid(id.into(), TemplateError::InvalidId)?;
        let scope = valid(scope.into(), TemplateError::InvalidScope)?;
        let definition = definition.into();
        if definition.is_empty()
            || definition.len() > 16384
            || definition.chars().any(char::is_control)
        {
            return Err(TemplateError::InvalidDefinition);
        }
        let mut names = placeholders;
        names.sort();
        names.dedup();
        if names.iter().any(|n| {
            valid(n.clone(), TemplateError::InvalidDefinition).is_err()
                || !definition.contains(&format!("{{{{{n}}}}}"))
        }) {
            return Err(TemplateError::InvalidDefinition);
        }
        let mut g = self.inner.lock().unwrap();
        if g.len() >= self.max_entries {
            return Err(TemplateError::Capacity);
        }
        if g.contains_key(&id) {
            return Err(TemplateError::Duplicate);
        }
        let t = ProjectTemplate {
            id: id.clone(),
            scope,
            version: 1,
            definition,
            placeholders: names,
        };
        g.insert(id, t.clone());
        Ok(t)
    }
    pub fn update(
        &self,
        id: &str,
        scope: &str,
        expected_version: u64,
        definition: impl Into<String>,
    ) -> Result<ProjectTemplate, TemplateError> {
        let definition = definition.into();
        if definition.is_empty()
            || definition.len() > 16384
            || definition.chars().any(char::is_control)
        {
            return Err(TemplateError::InvalidDefinition);
        }
        let mut g = self.inner.lock().unwrap();
        let t = g.get_mut(id).ok_or(TemplateError::Missing)?;
        if t.scope != scope {
            return Err(TemplateError::ScopeMismatch);
        }
        if t.version != expected_version {
            return Err(TemplateError::RevisionConflict);
        }
        if t.placeholders
            .iter()
            .any(|n| !definition.contains(&format!("{{{{{n}}}}}")))
        {
            return Err(TemplateError::InvalidDefinition);
        }
        t.definition = definition;
        t.version += 1;
        Ok(t.clone())
    }
    pub fn materialize(
        &self,
        id: &str,
        scope: &str,
        version: u64,
        values: BTreeMap<String, String>,
    ) -> Result<MaterializedTemplate, TemplateError> {
        let g = self.inner.lock().unwrap();
        let t = g.get(id).ok_or(TemplateError::Missing)?;
        if t.scope != scope {
            return Err(TemplateError::ScopeMismatch);
        }
        if t.version != version {
            return Err(TemplateError::RevisionConflict);
        }
        if values.len() != t.placeholders.len()
            || t.placeholders.iter().any(|n| !values.contains_key(n))
        {
            return Err(TemplateError::InvalidDefinition);
        }
        let mut out = t.definition.clone();
        for n in &t.placeholders {
            let v = values.get(n).unwrap();
            if v.is_empty() || v.len() > 4096 || v.chars().any(char::is_control) {
                return Err(TemplateError::InvalidDefinition);
            }
            out = out.replace(&format!("{{{{{n}}}}}"), v);
        }
        Ok(MaterializedTemplate {
            template_id: t.id.clone(),
            version: t.version,
            definition: out,
        })
    }
}
fn valid(v: String, e: TemplateError) -> Result<String, TemplateError> {
    if v.is_empty() || v.len() > 128 || v.chars().any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        Err(e)
    } else {
        Ok(v)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn materializes_and_updates_optimistically() {
        let s = ProjectTemplateStore::new(2);
        s.create(
            "intro",
            "tenant",
            "clip={{clip}} caption={{caption}}",
            vec!["caption".into(), "clip".into()],
        )
        .unwrap();
        let mut v = BTreeMap::new();
        v.insert("clip".into(), "a.mp4".into());
        v.insert("caption".into(), "Hello".into());
        assert_eq!(
            s.materialize("intro", "tenant", 1, v).unwrap().definition,
            "clip=a.mp4 caption=Hello"
        );
        assert_eq!(
            s.update("intro", "tenant", 1, "x={{clip}} caption={{caption}}")
                .unwrap()
                .version,
            2
        );
        assert_eq!(
            s.update("intro", "tenant", 1, "stale={{clip}} caption={{caption}}"),
            Err(TemplateError::RevisionConflict)
        );
    }
    #[test]
    fn validates_scope_capacity_and_missing_values() {
        let s = ProjectTemplateStore::new(1);
        assert_eq!(
            s.create("../bad", "tenant", "x", vec![]),
            Err(TemplateError::InvalidId)
        );
        s.create("x", "tenant", "x={{value}}", vec!["value".into()])
            .unwrap();
        assert_eq!(
            s.create("y", "tenant", "x", vec![]),
            Err(TemplateError::Capacity)
        );
        assert_eq!(
            s.materialize("x", "other", 1, BTreeMap::new()),
            Err(TemplateError::ScopeMismatch)
        );
    }
}
