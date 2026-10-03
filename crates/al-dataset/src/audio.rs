//! Dataset audio store: WAV files written durably via the shared
//! temp-file + fsync + rename discipline.

use std::fs::{self, File};
use std::io;
use std::path::Path;

use al_job::fsutil::{rename_durable, tmp_path_for};
use al_media::wav::{DecodedAudio, WavEncoding, WavWriter, decode_wav};

/// Write `samples` (interleaved f64 in `encoding`'s native scale) to
/// `path` as a RIFF/WAVE file, atomically and durably.
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
    {
        let file = File::create(&tmp)?;
        let mut writer = WavWriter::new(file, encoding, channels, sample_rate)?;
        writer.write_samples(samples)?;
        let file = writer.finish()?;
        file.sync_all()?;
    }
    rename_durable(&tmp, path)
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
}
