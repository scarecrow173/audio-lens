//! 動作確認用サンプル WAV 生成: `cargo run -p al-cli --example gen_samples -- samples/`
//!
//! 決定的(シード固定)な合成信号のみを書き出す。実マテリアルは
//! 含めない。

use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::path::{Path, PathBuf};

use al_media::wav::{WavEncoding, WavWriter};

const RATE: u32 = 48_000;

/// シード固定 LCG による ±32767 の擬似ホワイトノイズ。
fn noise(seed: u64, n: usize, amp: f64) -> Vec<f64> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((s >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
        })
        .map(|v| v * amp)
        .collect()
}

fn sine(freq: f64, n: usize, amp: f64) -> Vec<f64> {
    (0..n)
        .map(|i| (2.0 * std::f64::consts::PI * freq * i as f64 / f64::from(RATE)).sin() * amp)
        .collect()
}

fn write_wav(path: &Path, channels: u16, samples: &[f64]) {
    let file = File::create(path).unwrap();
    let mut w = WavWriter::new(BufWriter::new(file), WavEncoding::PcmS16, channels, RATE).unwrap();
    w.write_samples(samples).unwrap();
    w.finish().unwrap().flush().unwrap();
    println!("wrote {}", path.display());
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("samples"));
    std::fs::create_dir_all(&dir).unwrap();

    // 1. クリーンな 440 Hz 正弦波(capture 単独・ベースライン)
    write_wav(
        &dir.join("capture-tone-440hz.wav"),
        1,
        &sine(440.0, RATE as usize, 8_000.0),
    );

    // 2. locate 用: ノイズバースト source + 埋め込み capture
    //    source 4096 frame を capture の 9600 frame 目(0.2 s)に 0.5 ゲインで埋め込み
    let source = noise(7, 4_096, 20_000.0);
    let mut cap = noise(11, RATE as usize * 2, 3_000.0);
    for (i, &v) in source.iter().enumerate() {
        cap[9_600 + i] += 0.5 * v;
    }
    write_wav(&dir.join("source-noise-burst.wav"), 1, &source);
    write_wav(&dir.join("capture-embedded.wav"), 1, &cap);

    // 3. クリッピング: フルスケール超過の正弦波 → i16 飽和でハードクリップ
    write_wav(
        &dir.join("capture-clipped.wav"),
        1,
        &sine(1_000.0, RATE as usize, 60_000.0),
    );

    // 4. ドロップアウト: 正弦波の 0.5-0.8 s 区間を無音化
    let mut drop = sine(500.0, RATE as usize, 8_000.0);
    for v in &mut drop[24_000..38_400] {
        *v = 0.0;
    }
    write_wav(&dir.join("capture-dropout.wav"), 1, &drop);

    // 5. ステレオ: source を L=0.6 / R=-0.3(極性反転)でダウンミックス
    let src_l = noise(21, 8_192, 18_000.0);
    let src_r = noise(22, 8_192, 18_000.0);
    let mut stereo_src = Vec::with_capacity(src_l.len() * 2);
    for i in 0..src_l.len() {
        stereo_src.push(src_l[i]);
        stereo_src.push(src_r[i]);
    }
    let bg_l = noise(31, RATE as usize * 3, 2_000.0);
    let bg_r = noise(32, RATE as usize * 3, 2_000.0);
    let mut stereo_cap = Vec::with_capacity(bg_l.len() * 2);
    for i in 0..bg_l.len() {
        let (mut l, mut r) = (bg_l[i], bg_r[i]);
        if (48_000..48_000 + src_l.len()).contains(&i) {
            l += 0.6 * src_l[i - 48_000];
            r += -0.3 * src_r[i - 48_000];
        }
        stereo_cap.push(l);
        stereo_cap.push(r);
    }
    write_wav(&dir.join("source-stereo-pair.wav"), 2, &stereo_src);
    write_wav(&dir.join("capture-stereo-mixed.wav"), 2, &stereo_cap);
}
