#![deny(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VideoCodec { H264, Vp9, ProRes, Hevc }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PixelFormat { Yuv420, Yuv422, Yuv444, Rgba }

/// Validate codec + pixel format combination and dimensions.
/// Returns Ok(()) if valid, Err with description if not.
pub fn validate_codec_pixel_format(
    codec: VideoCodec,
    format: PixelFormat,
    width: u32,
    height: u32,
) -> Result<(), String> {
    // YUV420/422 require even dimensions
    if matches!(format, PixelFormat::Yuv420 | PixelFormat::Yuv422) {
        if !width.is_multiple_of(2) {
            return Err(format!("width {} must be even for {:?}", width, format));
        }
        if !height.is_multiple_of(2) {
            return Err(format!("height {} must be even for {:?}", height, format));
        }
    }
    // ProRes doesn't support YUV420
    if codec == VideoCodec::ProRes && format == PixelFormat::Yuv420 {
        return Err("ProRes does not support Yuv420 pixel format".into());
    }
    // VP9 doesn't support RGBA directly
    if codec == VideoCodec::Vp9 && format == PixelFormat::Rgba {
        return Err("Vp9 does not support Rgba pixel format; use Yuv420".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// H264 + Yuv420 with even dimensions must pass.
    #[test]
    fn test_h264_yuv420_valid() {
        let result = validate_codec_pixel_format(VideoCodec::H264, PixelFormat::Yuv420, 1920, 1080);
        assert!(result.is_ok(), "H264 + Yuv420 with even dimensions must be valid");
    }

    /// Yuv420 with odd width must fail.
    #[test]
    fn test_yuv420_odd_width_fails() {
        let result = validate_codec_pixel_format(VideoCodec::H264, PixelFormat::Yuv420, 1921, 1080);
        assert!(result.is_err(), "odd width must fail for Yuv420");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("width") && msg.contains("even"),
            "error must mention width and even, got: {msg}"
        );
    }

    /// Yuv420 with odd height must fail.
    #[test]
    fn test_yuv420_odd_height_fails() {
        let result = validate_codec_pixel_format(VideoCodec::H264, PixelFormat::Yuv420, 1920, 1081);
        assert!(result.is_err(), "odd height must fail for Yuv420");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("height") && msg.contains("even"),
            "error must mention height and even, got: {msg}"
        );
    }

    /// ProRes + Yuv420 must fail regardless of dimensions.
    #[test]
    fn test_prores_yuv420_fails() {
        let result = validate_codec_pixel_format(VideoCodec::ProRes, PixelFormat::Yuv420, 1920, 1080);
        assert!(result.is_err(), "ProRes + Yuv420 must fail");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("ProRes") && msg.contains("Yuv420"),
            "error must mention ProRes and Yuv420, got: {msg}"
        );
    }

    /// VP9 + Rgba must fail.
    #[test]
    fn test_vp9_rgba_fails() {
        let result = validate_codec_pixel_format(VideoCodec::Vp9, PixelFormat::Rgba, 1920, 1080);
        assert!(result.is_err(), "Vp9 + Rgba must fail");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("Vp9") && msg.contains("Rgba"),
            "error must mention Vp9 and Rgba, got: {msg}"
        );
    }
}
