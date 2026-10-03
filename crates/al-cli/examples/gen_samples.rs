//! 動作確認用サンプル WAV 生成: `cargo run -p al-cli --example gen_samples -- samples/`
//!
//! 決定的(シード固定)な合成信号のみを書き出す。実マテリアルは
//! 含めない。

use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::path::{Path, PathBuf};

use al_media::avi::{AviSpec, write_avi};
use al_media::wav::{WavEncoding, WavWriter};

const RATE: u32 = 48_000;
const VIDEO_W: u32 = 128;
const VIDEO_H: u32 = 96;
const VIDEO_FPS: u32 = 10;

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

/// f64 PCM → インターリーブ済み s16 LE バイト列(AVI 音声チャンク用)。
fn to_s16le(samples: &[f64]) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|&v| (v.round().clamp(-32768.0, 32767.0) as i16).to_le_bytes())
        .collect()
}

/// 決定的なテスト映像: 縦グラデーション背景 + 横に掃引するバー +
/// `flash` フレームで全面発光。BGR24・ボトムアップ行順。
fn video_frame(frame: usize, bar_speed: usize, flash: usize) -> Vec<u8> {
    let (w, h) = (VIDEO_W as usize, VIDEO_H as usize);
    let mut px = vec![0u8; w * h * 3];
    let bar_x = (frame * bar_speed) % w;
    for y in 0..h {
        // ボトムアップ: ファイル先頭が最下行
        let row = (h - 1 - y) * w * 3;
        for x in 0..w {
            let i = row + x * 3;
            let mut b = (y * 2) as u8;
            let mut g = (x * 2) as u8;
            let mut r = 40u8;
            if x.abs_diff(bar_x) < 4 {
                r = 200;
            }
            if frame == flash {
                b = b.saturating_add(180);
                g = g.saturating_add(180);
                r = r.saturating_add(180);
            }
            px[i] = b;
            px[i + 1] = g;
            px[i + 2] = r;
        }
    }
    px
}

fn write_avi_file(path: &Path, audio: &[f64], bar_speed: usize, flash: usize) {
    let n_frames = (audio.len() as u64 * u64::from(VIDEO_FPS) / u64::from(RATE)) as usize;
    let bytes = write_avi(&AviSpec {
        width: VIDEO_W,
        height: VIDEO_H,
        fps: VIDEO_FPS,
        frames: (0..n_frames)
            .map(|f| video_frame(f, bar_speed, flash))
            .collect(),
        audio: to_s16le(audio),
        channels: 1,
        sample_rate: RATE,
    });
    std::fs::write(path, bytes).unwrap();
    println!("wrote {}", path.display());
}

/// 簡易 BGM フレーズ: 4 音の連続する和音(根音 + 3度 + 5度)。
fn bgm(frames: usize, amp: f64) -> Vec<f64> {
    let chord = [[220.0, 277.2, 329.6], [196.0, 246.9, 293.7]];
    let seg = frames / 4;
    (0..frames)
        .map(|i| {
            let notes = chord[(i / seg) % 2];
            notes
                .iter()
                .map(|f| (2.0 * std::f64::consts::PI * f * i as f64 / f64::from(RATE)).sin())
                .sum::<f64>()
                * amp
                / notes.len() as f64
        })
        .collect()
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

    // 6. 動画ワークフロー用: 元音源 BGM アセット(2 s)と、それを内蔵した
    //    ゲームプレイ動画 3 本。映像は BGR24 非圧縮・音声は PCM s16。
    let bgm_asset = bgm(RATE as usize * 2, 24_000.0);
    write_wav(&dir.join("source-bgm.wav"), 1, &bgm_asset);

    /// capture ベッドノイズに BGM を `offset` から `gain` で重畳するヘルパー。
    fn gameplay_audio(seed: u64, frames: usize, bgm: &[f64], offset: usize, gain: f64) -> Vec<f64> {
        let mut a = noise(seed, frames, 2_500.0);
        for (i, &v) in bgm.iter().enumerate() {
            if offset + i < a.len() {
                a[offset + i] += gain * v;
            }
        }
        a
    }

    // 6a. 正常動画: BGM を 0.4 s 目から 0.5 ゲインで内蔵
    let normal = gameplay_audio(41, RATE as usize * 3, &bgm_asset, 19_200, 0.5);
    write_avi_file(&dir.join("video-gameplay-normal.avi"), &normal, 4, 15);

    // 6b. バグ動画: 同一レシピ + 1.0-1.5 s ドロップアウト + 2.2-2.4 s クリップ
    let mut bug = gameplay_audio(41, RATE as usize * 3, &bgm_asset, 19_200, 0.5);
    for v in &mut bug[48_000..72_000] {
        *v = 0.0;
    }
    for v in &mut bug[105_600..115_200] {
        *v *= 40.0;
    }
    write_avi_file(&dir.join("video-gameplay-bug.avi"), &bug, 4, 15);

    // 6c. 非同一リプレイ動画: 別テイク — BGM 位置・ゲイン・テンポ、
    //     ノイズ床、映像の掃引速度がすべて微妙に異なる
    let mut bgm_shrunk = Vec::with_capacity(bgm_asset.len());
    // 約 1% のテンポ差を線形補間で再現
    for i in 0..(bgm_asset.len() as f64 / 1.01) as usize {
        let pos = i as f64 * 1.01;
        let j = pos as usize;
        let frac = pos - j as f64;
        if j + 1 < bgm_asset.len() {
            bgm_shrunk.push(bgm_asset[j] * (1.0 - frac) + bgm_asset[j + 1] * frac);
        }
    }
    let run2 = gameplay_audio(43, RATE as usize * 3, &bgm_shrunk, 28_800, 0.45);
    write_avi_file(&dir.join("video-gameplay-run2.avi"), &run2, 5, 8);
}
