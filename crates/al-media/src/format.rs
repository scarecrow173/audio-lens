//! Container / codec signature detection.
//!
//! Detection relies on magic bytes only; file extensions are never trusted.
//! Formats we cannot decode fail explicitly instead of being guessed at.

/// A media format we can positively identify from its byte signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaFormat {
    /// RIFF/WAVE container (`RIFF....WAVE`).
    Wav,
}

/// Detect the container format from the leading bytes of `data`.
///
/// Returns `None` when no known signature matches. Callers must treat that
/// as `decode/input unavailable`, not as a soft failure.
pub fn detect_format(data: &[u8]) -> Option<MediaFormat> {
    if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WAVE" {
        return Some(MediaFormat::Wav);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_wav_signature() {
        let mut bytes = vec![0u8; 64];
        bytes[0..4].copy_from_slice(b"RIFF");
        bytes[8..12].copy_from_slice(b"WAVE");
        assert_eq!(detect_format(&bytes), Some(MediaFormat::Wav));
    }

    #[test]
    fn unknown_signature_is_not_detected() {
        assert_eq!(detect_format(b"PK\x03\x04...."), None);
        assert_eq!(detect_format(b"RIFF\x00\x00\x00\x00AVI "), None);
    }

    #[test]
    fn short_input_is_not_detected() {
        assert_eq!(detect_format(b"RIFF"), None);
        assert_eq!(detect_format(&[]), None);
    }
}
