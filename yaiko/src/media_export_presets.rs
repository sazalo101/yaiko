//! Scoped, versioned presets built on top of validated media export profiles.

use crate::media_export::{AudioCodec, Container, ExportError, ExportProfile, VideoCodec};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportPresetError {
    InvalidId,
    InvalidScope,
    InvalidName,
    Duplicate,
    Missing,
    Capacity,
    RevisionConflict,
    ScopeMismatch,
    Export(ExportError),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaExportPreset {
    pub id: String,
    pub scope: String,
    pub name: String,
    pub version: u64,
    pub width: u16,
    pub height: u16,
    pub video_bitrate_kbps: u32,
    pub audio_bitrate_kbps: u32,
    pub video_codec: VideoCodec,
    pub audio_codec: AudioCodec,
    pub container: Container,
    pub duration: Option<Duration>,
}
#[derive(Debug, Clone)]
pub struct MediaExportPresetStore {
    inner: Arc<Mutex<HashMap<String, MediaExportPreset>>>,
    max_entries: usize,
}
impl MediaExportPresetStore {
    pub fn new(max_entries: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_entries: max_entries.max(1),
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        &self,
        id: impl Into<String>,
        scope: impl Into<String>,
        name: impl Into<String>,
        width: u16,
        height: u16,
        vb: u32,
        ab: u32,
        vc: VideoCodec,
        ac: AudioCodec,
        c: Container,
    ) -> Result<MediaExportPreset, ExportPresetError> {
        let id = valid(id.into(), ExportPresetError::InvalidId)?;
        let scope = valid(scope.into(), ExportPresetError::InvalidScope)?;
        let name = valid(name.into(), ExportPresetError::InvalidName)?;
        ExportProfile::new("preset.mp4", width, height, vb, ab, vc, ac, c)
            .map_err(ExportPresetError::Export)?;
        let mut g = self.inner.lock().unwrap();
        if g.len() >= self.max_entries {
            return Err(ExportPresetError::Capacity);
        }
        if g.contains_key(&id) {
            return Err(ExportPresetError::Duplicate);
        }
        let p = MediaExportPreset {
            id: id.clone(),
            scope,
            name,
            version: 1,
            width,
            height,
            video_bitrate_kbps: vb,
            audio_bitrate_kbps: ab,
            video_codec: vc,
            audio_codec: ac,
            container: c,
            duration: None,
        };
        g.insert(id, p.clone());
        Ok(p)
    }
    pub fn duration(
        &self,
        id: &str,
        scope: &str,
        expected: u64,
        d: Duration,
    ) -> Result<MediaExportPreset, ExportPresetError> {
        if d.is_zero() || d > Duration::from_secs(86400) {
            return Err(ExportPresetError::Export(ExportError::InvalidDuration));
        }
        let mut g = self.inner.lock().unwrap();
        let p = g.get_mut(id).ok_or(ExportPresetError::Missing)?;
        if p.scope != scope {
            return Err(ExportPresetError::ScopeMismatch);
        }
        if p.version != expected {
            return Err(ExportPresetError::RevisionConflict);
        }
        p.duration = Some(d);
        p.version += 1;
        Ok(p.clone())
    }
    pub fn command_line(
        &self,
        id: &str,
        scope: &str,
        version: u64,
        input: impl Into<String>,
    ) -> Result<Vec<String>, ExportPresetError> {
        let g = self.inner.lock().unwrap();
        let p = g.get(id).ok_or(ExportPresetError::Missing)?;
        if p.scope != scope {
            return Err(ExportPresetError::ScopeMismatch);
        }
        if p.version != version {
            return Err(ExportPresetError::RevisionConflict);
        }
        let mut args = vec!["-i".into(), input.into()];
        let profile = ExportProfile::new(
            format!(
                "renders/{}.{}",
                p.id,
                match p.container {
                    Container::Mp4 => "mp4",
                    Container::Webm => "webm",
                    Container::Mkv => "mkv",
                }
            ),
            p.width,
            p.height,
            p.video_bitrate_kbps,
            p.audio_bitrate_kbps,
            p.video_codec,
            p.audio_codec,
            p.container,
        )
        .map_err(ExportPresetError::Export)?;
        let mut rest = profile.command_line();
        args.append(&mut rest);
        Ok(args)
    }
}
fn valid(v: String, e: ExportPresetError) -> Result<String, ExportPresetError> {
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
    fn creates_versions_duration_and_args() {
        let s = MediaExportPresetStore::new(2);
        let p = s
            .create(
                "social",
                "tenant",
                "Social MP4",
                1920,
                1080,
                8000,
                192,
                VideoCodec::H264,
                AudioCodec::Aac,
                Container::Mp4,
            )
            .unwrap();
        let p = s
            .duration("social", "tenant", p.version, Duration::from_secs(30))
            .unwrap();
        let a = s
            .command_line("social", "tenant", p.version, "input.mp4")
            .unwrap();
        assert!(a.windows(2).any(|w| w == ["-i", "input.mp4"]));
        assert!(a.iter().any(|v| v == "libx264"));
    }
    #[test]
    fn rejects_scope_revision_capacity_and_invalid_codecs() {
        let s = MediaExportPresetStore::new(1);
        assert_eq!(
            s.create(
                "bad/id",
                "tenant",
                "x",
                1920,
                1080,
                8000,
                128,
                VideoCodec::H264,
                AudioCodec::Aac,
                Container::Mp4
            ),
            Err(ExportPresetError::InvalidId)
        );
        let p = s
            .create(
                "x",
                "tenant",
                "x",
                1920,
                1080,
                8000,
                128,
                VideoCodec::H264,
                AudioCodec::Aac,
                Container::Mp4,
            )
            .unwrap();
        assert_eq!(
            s.command_line("x", "other", p.version, "in.mp4"),
            Err(ExportPresetError::ScopeMismatch)
        );
        assert_eq!(
            s.command_line("x", "tenant", 99, "in.mp4"),
            Err(ExportPresetError::RevisionConflict)
        );
        assert_eq!(
            s.create(
                "y",
                "tenant",
                "y",
                1920,
                1080,
                8000,
                128,
                VideoCodec::H264,
                AudioCodec::Opus,
                Container::Mp4
            ),
            Err(ExportPresetError::Export(ExportError::IncompatibleCodec))
        );
    }
}
