//! Container / codec signature detection.
//!
//! Detection relies on magic bytes only; file extensions are never trusted.
//! Formats we cannot decode fail explicitly instead of being guessed at.

/// A media format we can positively identify from its byte signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaFormat {
    /// RIFF/WAVE container (`RIFF....WAVE`).
    Wav,
    /// RIFF/AVI container (`RIFF....AVI `).
    Avi,
    /// ISO-BMFF / MP4 container whose first box is `ftyp`.
    Mp4,
}

const FORMAT_SIGNATURE_PREFIX_LEN: usize = 12;

fn signature_format_from_prefix(
    len: usize,
    prefix: &[u8; FORMAT_SIGNATURE_PREFIX_LEN],
) -> Option<MediaFormat> {
    if len >= 12 && &prefix[0..4] == b"RIFF" {
        return match &prefix[8..12] {
            b"WAVE" => Some(MediaFormat::Wav),
            b"AVI " => Some(MediaFormat::Avi),
            _ => None,
        };
    }
    if len >= 8 && &prefix[4..8] == b"ftyp" {
        return Some(MediaFormat::Mp4);
    }
    None
}

/// Detect the container format from the leading bytes of `data`.
///
/// Returns `None` when no known signature matches. Callers must treat that
/// as `decode/input unavailable`, not as a soft failure.
///
/// MP4 is identified only by a leading `ftyp` box: the box type field at
/// offset 4. Files that start directly with `free`/`wide`/`styp`/`mdat`
/// (legal ISO-BMFF without an `ftyp`) stay `None` — we do not attempt
/// MP4 observation on containers we cannot positively identify.
pub fn detect_format(data: &[u8]) -> Option<MediaFormat> {
    let mut prefix = [0u8; FORMAT_SIGNATURE_PREFIX_LEN];
    let captured = data.len().min(FORMAT_SIGNATURE_PREFIX_LEN);
    prefix[..captured].copy_from_slice(&data[..captured]);
    signature_format_from_prefix(data.len(), &prefix)
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
    fn detects_avi_signature() {
        let mut bytes = vec![0u8; 64];
        bytes[0..4].copy_from_slice(b"RIFF");
        bytes[8..12].copy_from_slice(b"AVI ");
        assert_eq!(detect_format(&bytes), Some(MediaFormat::Avi));
    }

    #[test]
    fn detects_mp4_ftyp_signature() {
        let mut bytes = vec![0u8; 24];
        bytes[0..4].copy_from_slice(&24u32.to_be_bytes());
        bytes[4..8].copy_from_slice(b"ftyp");
        bytes[8..12].copy_from_slice(b"isom");
        assert_eq!(detect_format(&bytes), Some(MediaFormat::Mp4));
    }

    #[test]
    fn mp4_without_leading_ftyp_is_not_detected() {
        // Legal ISO-BMFF may start with free/mdat/styp, but we only
        // claim formats we positively identify.
        for tag in [b"free", b"wide", b"styp", b"mdat", b"moov"] {
            let mut bytes = vec![0u8; 16];
            bytes[4..8].copy_from_slice(tag);
            assert_eq!(detect_format(&bytes), None, "{tag:?}");
        }
    }

    #[test]
    fn unknown_signature_is_not_detected() {
        assert_eq!(detect_format(b"PK\x03\x04...."), None);
        assert_eq!(detect_format(b"RIFF\x00\x00\x00\x00WEBP"), None);
    }

    #[test]
    fn short_input_is_not_detected() {
        assert_eq!(detect_format(b"RIFF"), None);
        assert_eq!(detect_format(&[]), None);
    }

    #[test]
    fn length_thresholds_follow_signature_rules() {
        let mut seven = vec![0u8; 7];
        seven[4..7].copy_from_slice(b"fty");
        assert_eq!(detect_format(&seven), None);

        for len in [8usize, 11, 12, 13] {
            let mut bytes = vec![0u8; len];
            bytes[4..8].copy_from_slice(b"ftyp");
            assert_eq!(detect_format(&bytes), Some(MediaFormat::Mp4), "len={len}");
        }

        let twelve = vec![0u8; 12];
        assert_eq!(detect_format(&twelve), None);
    }

    #[test]
    fn riff_form_type_wins_over_ftyp_without_fallback() {
        for len in [12usize, 13] {
            let mut bytes = vec![0u8; len];
            bytes[0..4].copy_from_slice(b"RIFF");
            bytes[4..8].copy_from_slice(b"ftyp");
            bytes[8..12].copy_from_slice(b"WEBP");
            assert_eq!(detect_format(&bytes), None, "len={len}");
        }

        for len in [8usize, 11] {
            let mut bytes = vec![0u8; len];
            bytes[0..4].copy_from_slice(b"RIFF");
            bytes[4..8].copy_from_slice(b"ftyp");
            assert_eq!(detect_format(&bytes), Some(MediaFormat::Mp4), "len={len}");
        }
    }

    #[test]
    fn trailing_bytes_do_not_change_detection() {
        for trailing in [0u8, b'X', 0xff] {
            let mut bytes = vec![0u8; 13];
            bytes[0..4].copy_from_slice(b"RIFF");
            bytes[8..12].copy_from_slice(b"WAVE");
            bytes[12] = trailing;
            assert_eq!(detect_format(&bytes), Some(MediaFormat::Wav));
        }
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    fn contract_signature_format(
        len: usize,
        prefix: &[u8; FORMAT_SIGNATURE_PREFIX_LEN],
    ) -> Option<MediaFormat> {
        if len >= 12 && &prefix[0..4] == b"RIFF" {
            return match &prefix[8..12] {
                b"WAVE" => Some(MediaFormat::Wav),
                b"AVI " => Some(MediaFormat::Avi),
                _ => None,
            };
        }
        if len >= 8 && &prefix[4..8] == b"ftyp" {
            return Some(MediaFormat::Mp4);
        }
        None
    }

    fn contract_detect_format(data: &[u8]) -> Option<MediaFormat> {
        if data.len() >= 12 && &data[0..4] == b"RIFF" {
            return match &data[8..12] {
                b"WAVE" => Some(MediaFormat::Wav),
                b"AVI " => Some(MediaFormat::Avi),
                _ => None,
            };
        }
        if data.len() >= 8 && &data[4..8] == b"ftyp" {
            return Some(MediaFormat::Mp4);
        }
        None
    }

    #[kani::proof]
    #[kani::unwind(16)]
    fn format_signature_detection_bounded_exact() {
        let len: usize = kani::any();
        let prefix: [u8; FORMAT_SIGNATURE_PREFIX_LEN] = kani::any();
        let actual = signature_format_from_prefix(len, &prefix);
        let expected = contract_signature_format(len, &prefix);

        assert_eq!(actual, expected);
        if len < 8 {
            assert_eq!(actual, None);
        } else if len < 12 {
            assert_eq!(
                actual,
                if &prefix[4..8] == b"ftyp" {
                    Some(MediaFormat::Mp4)
                } else {
                    None
                }
            );
        } else if &prefix[0..4] == b"RIFF" {
            assert_eq!(
                actual,
                match &prefix[8..12] {
                    b"WAVE" => Some(MediaFormat::Wav),
                    b"AVI " => Some(MediaFormat::Avi),
                    _ => None,
                }
            );
        } else {
            assert_eq!(
                actual,
                if &prefix[4..8] == b"ftyp" {
                    Some(MediaFormat::Mp4)
                } else {
                    None
                }
            );
        }

        let input: [u8; 13] = kani::any();
        for len in [0usize, 7, 8, 11, 12, 13] {
            assert_eq!(
                detect_format(&input[..len]),
                contract_detect_format(&input[..len])
            );
        }
        assert_eq!(detect_format(&input[..12]), detect_format(&input[..13]));
    }
}
