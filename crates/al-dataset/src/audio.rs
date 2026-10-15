//! Dataset audio store: WAV files written durably via the shared
//! temp-file + fsync + rename discipline.

use std::fs::{self, File};
use std::io;
use std::path::Path;

use al_job::fsutil::{rename_atomic, tmp_path_for};
use al_media::wav::{DecodedAudio, WavEncoding, WavWriter, decode_wav};

/// Write `samples` (interleaved f64 in `encoding`'s native scale) to
/// `path` as a RIFF/WAVE file, atomically but without fsyncs
/// (relaxed tier — regenerable bulk data).
pub fn write_wav(
    path: &Path,
    encoding: WavEncoding,
    channels: u16,
    sample_rate: u32,
    samples: &[f64],
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = tmp_path_for(path);
    let result = (|| {
        let file = File::create(&tmp)?;
        let mut writer = WavWriter::new(file, encoding, channels, sample_rate)?;
        // Encode in bounded blocks: the streaming encoder exists so a
        // large file never needs a second full-size buffer. Blocks must
        // hold whole frames — `write_samples` rejects a count that is
        // not a multiple of `channels`, so a fixed power-of-two block
        // would split mid-frame for e.g. 3ch/6ch audio.
        let channels_per_frame = usize::from(channels).max(1);
        let block_frames = ((1 << 20) / channels_per_frame).max(1);
        for block in samples.chunks(block_frames * channels_per_frame) {
            writer.write_samples(block)?;
        }
        let file = writer.finish()?;
        // Relaxed tier: WAV payloads are regenerable bulk data —
        // skip fsync; resume verification regenerates on loss.
        drop(file);
        rename_atomic(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Decode a WAV file from the dataset.
pub fn read_wav(path: &Path) -> io::Result<DecodedAudio> {
    let bytes = fs::read(path)?;
    decode_wav(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_media::metadata::SampleFormat;

    #[test]
    fn wav_round_trip_through_dataset() {
        let dir = std::env::temp_dir().join(format!("ds-audio-{}", al_core::JobId::new()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("residual").join("src.wav");
        let samples = vec![0.5_f64, -0.5, 0.25, -0.25];
        write_wav(&path, WavEncoding::Float32, 2, 48000, &samples).unwrap();
        assert!(path.exists());
        assert!(!tmp_path_for(&path).exists());
        let audio = read_wav(&path).unwrap();
        assert_eq!(audio.sample_format, SampleFormat::Float32);
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.samples, samples);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn multi_block_write_keeps_frame_alignment() {
        // 1<<20 samples is not a multiple of 6 channels: a block split
        // there would cut a frame in half. Spanning the block boundary
        // exercises the frame-aligned chunking.
        let dir = std::env::temp_dir().join(format!("ds-audio-{}", al_core::JobId::new()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("six_ch.wav");
        // One frame past the ~1M-sample block boundary, in whole frames.
        let frames = (1 << 20) / 6 + 1;
        let samples: Vec<f64> = (0..frames * 6)
            .map(|i| ((i * 2654435761usize) % 2000) as f64 - 1000.0)
            .collect();
        write_wav(&path, WavEncoding::Float32, 6, 48000, &samples).unwrap();
        let audio = read_wav(&path).unwrap();
        assert_eq!(audio.channels, 6);
        assert_eq!(audio.samples.len(), samples.len());
        fs::remove_dir_all(&dir).ok();
    }
}
