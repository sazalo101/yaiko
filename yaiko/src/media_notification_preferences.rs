//! Scoped notification preferences for media workflows.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationChannel {
    Email,
    Webhook,
    InApp,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestMode {
    Immediate,
    Hourly,
    Daily,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationPreferenceError {
    Invalid,
    Missing,
    Duplicate,
    Capacity,
    ScopeMismatch,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaNotificationPreference {
    pub id: String,
    pub scope: String,
    pub owner: String,
    pub channel: NotificationChannel,
    pub event: String,
    pub digest: DigestMode,
    pub enabled: bool,
}
#[derive(Debug, Clone)]
pub struct MediaNotificationPreferenceStore {
    inner: Arc<Mutex<HashMap<String, Vec<MediaNotificationPreference>>>>,
    max_scopes: usize,
    max_preferences: usize,
}
impl MediaNotificationPreferenceStore {
    pub fn new(max_scopes: usize, max_preferences: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_scopes: max_scopes.max(1),
            max_preferences: max_preferences.max(1),
        }
    }
    pub fn add(
        &self,
        id: impl Into<String>,
        scope: impl Into<String>,
        owner: impl Into<String>,
        channel: NotificationChannel,
        event: impl Into<String>,
        digest: DigestMode,
    ) -> Result<MediaNotificationPreference, NotificationPreferenceError> {
        let id = v(id.into())?;
        let scope = v(scope.into())?;
        let owner = v(owner.into())?;
        let event = v(event.into())?;
        let mut g = self.inner.lock().unwrap();
        if !g.contains_key(&scope) && g.len() >= self.max_scopes {
            return Err(NotificationPreferenceError::Capacity);
        }
        let p = g.entry(scope.clone()).or_default();
        if p.len() >= self.max_preferences {
            return Err(NotificationPreferenceError::Capacity);
        }
        if p.iter().any(|x| x.id == id) {
            return Err(NotificationPreferenceError::Duplicate);
        }
        let x = MediaNotificationPreference {
            id,
            scope,
            owner,
            channel,
            event,
            digest,
            enabled: true,
        };
        p.push(x.clone());
        Ok(x)
    }
    pub fn matches(
        &self,
        scope: &str,
        owner: &str,
        event: &str,
    ) -> Result<Vec<MediaNotificationPreference>, NotificationPreferenceError> {
        let g = self.inner.lock().unwrap();
        let p = g.get(scope).ok_or(NotificationPreferenceError::Missing)?;
        let mut out = p
            .iter()
            .filter(|x| x.owner == owner && x.enabled && (x.event == event || x.event == "*"))
            .cloned()
            .collect::<Vec<_>>();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }
    pub fn set_enabled(
        &self,
        scope: &str,
        id: &str,
        enabled: bool,
    ) -> Result<(), NotificationPreferenceError> {
        let mut g = self.inner.lock().unwrap();
        let p = g
            .get_mut(scope)
            .ok_or(NotificationPreferenceError::Missing)?;
        let x = p
            .iter_mut()
            .find(|x| x.id == id)
            .ok_or(NotificationPreferenceError::Missing)?;
        x.enabled = enabled;
        Ok(())
    }
}
fn v(x: String) -> Result<String, NotificationPreferenceError> {
    if x.is_empty() || x.len() > 128 || x.chars().any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        Err(NotificationPreferenceError::Invalid)
    } else {
        Ok(x)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_wildcards_and_orders() {
        let s = MediaNotificationPreferenceStore::new(2, 3);
        s.add(
            "b",
            "t",
            "u",
            NotificationChannel::Email,
            "media.ready",
            DigestMode::Immediate,
        )
        .unwrap();
        s.add(
            "a",
            "t",
            "u",
            NotificationChannel::InApp,
            "*",
            DigestMode::Daily,
        )
        .unwrap();
        assert_eq!(
            s.matches("t", "u", "media.ready")
                .unwrap()
                .iter()
                .map(|x| x.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        )
    }
    #[test]
    fn validates_and_disables() {
        let s = MediaNotificationPreferenceStore::new(1, 1);
        let p = s
            .add(
                "x",
                "t",
                "u",
                NotificationChannel::Webhook,
                "media.failed",
                DigestMode::Hourly,
            )
            .unwrap();
        s.set_enabled("t", "x", false).unwrap();
        assert!(s.matches("t", "u", "media.failed").unwrap().is_empty());
        assert_eq!(
            s.add(
                "y",
                "t",
                "u",
                NotificationChannel::Email,
                "x",
                DigestMode::Daily
            ),
            Err(NotificationPreferenceError::Capacity)
        );
        assert_eq!(p.id, "x")
    }
}
