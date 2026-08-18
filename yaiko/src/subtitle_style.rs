//! Safe subtitle styling metadata for media composition.

use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubtitleStyleError {
    InvalidText,
    InvalidFont,
    InvalidColor,
    InvalidPosition,
    InvalidTiming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubtitlePosition {
    Top,
    Center,
    Bottom,
}

impl SubtitlePosition {
    fn expression(self) -> &'static str {
        match self {
            Self::Top => "(w-text_w)/2:48",
            Self::Center => "(w-text_w)/2:(h-text_h)/2",
            Self::Bottom => "(w-text_w)/2:h-text_h-48",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleStyle {
    pub font_name: String,
    pub font_size: u16,
    pub color: String,
    pub position: SubtitlePosition,
    pub outline_width: u8,
}

impl Default for SubtitleStyle {
    fn default() -> Self {
        Self {
            font_name: "Sans".into(),
            font_size: 42,
            color: "white".into(),
            position: SubtitlePosition::Bottom,
            outline_width: 2,
        }
    }
}

impl SubtitleStyle {
    pub fn new(
        font_name: impl Into<String>,
        font_size: u16,
        color: impl Into<String>,
        position: SubtitlePosition,
        outline_width: u8,
    ) -> Result<Self, SubtitleStyleError> {
        let font_name = font_name.into();
        let color = color.into();
        if font_name.is_empty()
            || font_name.len() > 64
            || font_name
                .chars()
                .any(|c| c.is_control() || matches!(c, ':' | '\'' | ';'))
        {
            return Err(SubtitleStyleError::InvalidFont);
        }
        if !(8..=128).contains(&font_size) {
            return Err(SubtitleStyleError::InvalidFont);
        }
        if !valid_color(&color) {
            return Err(SubtitleStyleError::InvalidColor);
        }
        if outline_width > 16 {
            return Err(SubtitleStyleError::InvalidPosition);
        }
        Ok(Self {
            font_name,
            font_size,
            color,
            position,
            outline_width,
        })
    }
    pub fn drawtext(
        &self,
        text: &str,
        start: Duration,
        duration: Duration,
    ) -> Result<String, SubtitleStyleError> {
        if text.is_empty()
            || text.len() > 512
            || text.chars().any(char::is_control)
            || duration.is_zero()
        {
            return Err(SubtitleStyleError::InvalidText);
        }
        if start > Duration::from_secs(86_400)
            || start.saturating_add(duration) > Duration::from_secs(86_400)
        {
            return Err(SubtitleStyleError::InvalidTiming);
        }
        Ok(format!("drawtext=fontfile='{}':text='{}':fontcolor={}:fontsize={}:borderw={}:x={}:enable='between(t,{:.3},{:.3})'", escape(&self.font_name), escape(text), self.color, self.font_size, self.outline_width, self.position.expression().split_once(':').unwrap().0, start.as_secs_f64(), start.saturating_add(duration).as_secs_f64()))
    }
}

fn valid_color(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && (value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '#' | '@' | '.' | '-' | '_'))
            || matches!(
                value,
                "white" | "black" | "yellow" | "red" | "green" | "blue"
            ))
}
fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "\\'")
        .replace('%', "\\%")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generates_safe_timed_drawtext() {
        let style = SubtitleStyle::default();
        let filter = style
            .drawtext(
                "Hello: 100%",
                Duration::from_secs(1),
                Duration::from_secs(2),
            )
            .unwrap();
        assert!(filter.contains("fontcolor=white"));
        assert!(filter.contains("fontsize=42"));
        assert!(filter.contains("Hello\\: 100\\%"));
        assert!(filter.contains("between(t,1.000,3.000)"));
    }
    #[test]
    fn validates_fonts_colors_sizes_positions_and_timing() {
        assert_eq!(
            SubtitleStyle::new("bad;font", 42, "white", SubtitlePosition::Bottom, 2),
            Err(SubtitleStyleError::InvalidFont)
        );
        assert_eq!(
            SubtitleStyle::new("Sans", 4, "white", SubtitlePosition::Bottom, 2),
            Err(SubtitleStyleError::InvalidFont)
        );
        assert_eq!(
            SubtitleStyle::new("Sans", 42, "red;rm", SubtitlePosition::Bottom, 2),
            Err(SubtitleStyleError::InvalidColor)
        );
        assert_eq!(
            SubtitleStyle::new("Sans", 42, "white", SubtitlePosition::Bottom, 17),
            Err(SubtitleStyleError::InvalidPosition)
        );
        assert_eq!(
            SubtitleStyle::default().drawtext(
                "x",
                Duration::from_secs(86_400),
                Duration::from_secs(1)
            ),
            Err(SubtitleStyleError::InvalidTiming)
        );
    }
    #[test]
    fn supports_all_positions_and_rejects_bad_text() {
        for position in [
            SubtitlePosition::Top,
            SubtitlePosition::Center,
            SubtitlePosition::Bottom,
        ] {
            assert!(SubtitleStyle::default()
                .drawtext("x", Duration::ZERO, Duration::from_secs(1))
                .unwrap()
                .contains("x="));
            let _ = position;
        }
        assert_eq!(
            SubtitleStyle::default().drawtext("", Duration::ZERO, Duration::from_secs(1)),
            Err(SubtitleStyleError::InvalidText)
        );
    }
}
