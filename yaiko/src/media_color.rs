//! Pixel-format and color-space policy validation for media exports.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Yuv420p,
    Yuv422p,
    Yuv444p,
    Rgb24,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    Bt601,
    Bt709,
    Bt2020,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorRange {
    Limited,
    Full,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transfer {
    Sdr,
    Hlg,
    Pq,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorPolicyError {
    UnsupportedFormat,
    IncompatibleColor,
    HdrNotAllowed,
    InvalidMetadata,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaColorPolicy {
    pub allow_422: bool,
    pub allow_444: bool,
    pub allow_rgb: bool,
    pub allow_hdr: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaColorMetadata {
    pub format: PixelFormat,
    pub space: ColorSpace,
    pub range: ColorRange,
    pub transfer: Transfer,
}

impl MediaColorPolicy {
    pub fn validate(&self, metadata: MediaColorMetadata) -> Result<(), ColorPolicyError> {
        match metadata.format {
            PixelFormat::Yuv422p if !self.allow_422 => {
                return Err(ColorPolicyError::UnsupportedFormat)
            }
            PixelFormat::Yuv444p if !self.allow_444 => {
                return Err(ColorPolicyError::UnsupportedFormat)
            }
            PixelFormat::Rgb24 if !self.allow_rgb => {
                return Err(ColorPolicyError::UnsupportedFormat)
            }
            _ => {}
        }
        if matches!(metadata.transfer, Transfer::Hlg | Transfer::Pq) && !self.allow_hdr {
            return Err(ColorPolicyError::HdrNotAllowed);
        }
        if matches!(metadata.transfer, Transfer::Pq | Transfer::Hlg)
            && matches!(metadata.space, ColorSpace::Bt601)
        {
            return Err(ColorPolicyError::IncompatibleColor);
        }
        if matches!(metadata.range, ColorRange::Full)
            && matches!(
                metadata.format,
                PixelFormat::Yuv420p | PixelFormat::Yuv422p | PixelFormat::Yuv444p
            )
            && matches!(metadata.space, ColorSpace::Bt601)
        {
            return Err(ColorPolicyError::IncompatibleColor);
        }
        Ok(())
    }
    pub fn ffmpeg_args(
        &self,
        metadata: MediaColorMetadata,
    ) -> Result<Vec<String>, ColorPolicyError> {
        self.validate(metadata)?;
        let format = match metadata.format {
            PixelFormat::Yuv420p => "yuv420p",
            PixelFormat::Yuv422p => "yuv422p",
            PixelFormat::Yuv444p => "yuv444p",
            PixelFormat::Rgb24 => "rgb24",
        };
        let space = match metadata.space {
            ColorSpace::Bt601 => "bt601",
            ColorSpace::Bt709 => "bt709",
            ColorSpace::Bt2020 => "bt2020",
        };
        Ok(vec![
            "-pix_fmt".into(),
            format.into(),
            "-colorspace".into(),
            space.into(),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn policy() -> MediaColorPolicy {
        MediaColorPolicy {
            allow_422: true,
            allow_444: false,
            allow_rgb: false,
            allow_hdr: false,
        }
    }
    #[test]
    fn accepts_safe_sdr_metadata_and_builds_args() {
        let metadata = MediaColorMetadata {
            format: PixelFormat::Yuv420p,
            space: ColorSpace::Bt709,
            range: ColorRange::Limited,
            transfer: Transfer::Sdr,
        };
        assert_eq!(policy().validate(metadata), Ok(()));
        assert_eq!(
            policy().ffmpeg_args(metadata).unwrap(),
            vec!["-pix_fmt", "yuv420p", "-colorspace", "bt709"]
        );
    }
    #[test]
    fn rejects_unsupported_formats_hdr_and_incompatible_colors() {
        let mut metadata = MediaColorMetadata {
            format: PixelFormat::Yuv444p,
            space: ColorSpace::Bt709,
            range: ColorRange::Limited,
            transfer: Transfer::Sdr,
        };
        assert_eq!(
            policy().validate(metadata),
            Err(ColorPolicyError::UnsupportedFormat)
        );
        metadata.format = PixelFormat::Yuv420p;
        metadata.transfer = Transfer::Pq;
        assert_eq!(
            policy().validate(metadata),
            Err(ColorPolicyError::HdrNotAllowed)
        );
        metadata.transfer = Transfer::Sdr;
        metadata.space = ColorSpace::Bt601;
        metadata.range = ColorRange::Full;
        assert_eq!(
            policy().validate(metadata),
            Err(ColorPolicyError::IncompatibleColor)
        );
    }
}
