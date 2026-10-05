//! 決定的な合成信号ビルダ / 変換。
//!
//! 全てシード固定の疑似乱数(LCG)か決定的な変換のみ。外部乱数
//! クレートには依存しない。信号は `f64`、本番側の decoded_pcm と
//! 同じく「native scale の f64」を想定する。

/// シード固定 LCG(Numerical Recipes 定数)。同一シードからは常に
/// 同一系列が出る。
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// `seed` で初期化する。同一シードからは常に同一系列が出る。
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// 次の 64bit 疑似乱数。
    pub fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    /// `[0, 1)` の一様乱数。
    pub fn next_f64(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) / ((1u64 << 53) as f64)
    }

    /// `[-1, 1)` の一様乱数。
    pub fn symmetric(&mut self) -> f64 {
        self.next_f64() * 2.0 - 1.0
    }
}

/// 擬似ホワイトノイズ `amp * uniform(-1, 1)`。
pub fn noise(len: usize, seed: u64, amp: f64) -> Vec<f64> {
    let mut rng = Rng::new(seed);
    (0..len).map(|_| rng.symmetric() * amp).collect()
}

/// 単一周波数の正弦波。
pub fn sine(freq_hz: f64, len: usize, sample_rate: u32, amp: f64) -> Vec<f64> {
    (0..len)
        .map(|i| {
            (2.0 * std::f64::consts::PI * freq_hz * i as f64 / f64::from(sample_rate)).sin() * amp
        })
        .collect()
}

/// 無音(ゼロ列)。
pub fn silence(len: usize) -> Vec<f64> {
    vec![0.0; len]
}

/// スカラーゲイン。
pub fn apply_gain(x: &[f64], gain: f64) -> Vec<f64> {
    x.iter().map(|&v| v * gain).collect()
}

/// 極性反転。
pub fn invert(x: &[f64]) -> Vec<f64> {
    x.iter().map(|&v| -v).collect()
}

/// 整数サンプル遅延。先頭に `delay` 個のゼロを挿入し、末尾を切って
/// 長さを保つ。
pub fn delay(x: &[f64], delay: usize) -> Vec<f64> {
    let mut out = vec![0.0; x.len()];
    let n = x.len().saturating_sub(delay);
    out[delay..delay + n].copy_from_slice(&x[..n]);
    out
}

/// 線形補間による小数サンプル遅延。`delay` は負にもできる。
/// 範囲外はゼロ。
pub fn frac_delay(x: &[f64], delay: f64) -> Vec<f64> {
    let n = x.len();
    let at = |j: isize| {
        if j >= 0 && (j as usize) < n {
            x[j as usize]
        } else {
            0.0
        }
    };
    (0..n)
        .map(|i| {
            let pos = i as f64 - delay;
            let j = pos.floor() as isize;
            let frac = pos - j as f64;
            at(j) * (1.0 - frac) + at(j + 1) * frac
        })
        .collect()
}

/// ブロック単位のジッタ付き遅延。`block` サンプル毎に
/// `base + uniform(-jitter, jitter)` サンプルの遅延を掛ける。
/// ブロック境界の不連続は実機のバーストジッタを模す意図的なもの。
pub fn jitter_delay(x: &[f64], base: f64, jitter: f64, block: usize, seed: u64) -> Vec<f64> {
    let mut rng = Rng::new(seed);
    let block = block.max(1);
    let mut out = vec![0.0; x.len()];
    let mut start = 0;
    while start < x.len() {
        let d = base + rng.symmetric() * jitter;
        let end = (start + block).min(x.len());
        for (i, o) in out.iter_mut().enumerate().take(end).skip(start) {
            let pos = i as f64 - d;
            let j = pos.floor() as isize;
            let frac = pos - j as f64;
            let at = |j: isize| {
                if j >= 0 && (j as usize) < x.len() {
                    x[j as usize]
                } else {
                    0.0
                }
            };
            *o = at(j) * (1.0 - frac) + at(j + 1) * frac;
        }
        start = end;
    }
    out
}

/// `dst` の `offset` 以降に `gain * src` を加算する(はみ出しは切る)。
pub fn embed(dst: &mut [f64], src: &[f64], offset: usize, gain: f64) {
    let n = src.len().min(dst.len().saturating_sub(offset));
    for (i, &v) in src.iter().take(n).enumerate() {
        dst[offset + i] += gain * v;
    }
}

/// 線形補間の決定的リサンプラ。`out[i] = x[i * ratio]`。
/// `ratio > 1` で時間圧縮(出力は短くなる)、`ratio < 1` で伸長。
/// 補間位置は常に入力範囲内に収まるように出力長を決める。
pub fn resample_linear(x: &[f64], ratio: f64) -> Vec<f64> {
    if x.is_empty() || ratio <= 0.0 {
        return Vec::new();
    }
    let out_len = ((x.len() - 1) as f64 / ratio).floor() as usize + 1;
    (0..out_len)
        .map(|i| {
            let pos = i as f64 * ratio;
            let j = pos as usize;
            let frac = pos - j as f64;
            let b = x[(j + 1).min(x.len() - 1)];
            x[j] * (1.0 - frac) + b * frac
        })
        .collect()
}

/// ハードクリップ `[-limit, limit]`。
pub fn clip(x: &[f64], limit: f64) -> Vec<f64> {
    let limit = limit.abs();
    x.iter().map(|&v| v.clamp(-limit, limit)).collect()
}

/// ブロック欠落: `[start, start+len)` を取り除き後続を詰める。
/// 出力長は `x.len() - len`。
pub fn drop_block(x: &[f64], start: usize, len: usize) -> Vec<f64> {
    let start = start.min(x.len());
    let end = (start + len).min(x.len());
    let mut out = Vec::with_capacity(x.len() - (end - start));
    out.extend_from_slice(&x[..start]);
    out.extend_from_slice(&x[end..]);
    out
}

/// ブロック無音化: `[start, start+len)` をゼロにする(長さ不変)。
pub fn zero_block(x: &[f64], start: usize, len: usize) -> Vec<f64> {
    let mut out = x.to_vec();
    let start = start.min(out.len());
    let end = (start + len).min(out.len());
    out[start..end].fill(0.0);
    out
}

/// ブロック反復: `[start, start+len)` をその場でもう一度挿入する。
/// 出力長は `x.len() + len`。バッファ反復バグの模倣。
pub fn repeat_block(x: &[f64], start: usize, len: usize) -> Vec<f64> {
    let start = start.min(x.len());
    let end = (start + len).min(x.len());
    let mut out = Vec::with_capacity(x.len() + (end - start));
    out.extend_from_slice(&x[..end]);
    out.extend_from_slice(&x[start..end]);
    out.extend_from_slice(&x[end..]);
    out
}

/// スタックサンプル: `[start, start+len)` を `x[start-1]` の値で
/// 埋める(出力が張り付くバグの模倣)。
pub fn stuck_block(x: &[f64], start: usize, len: usize) -> Vec<f64> {
    let mut out = x.to_vec();
    let start = start.min(out.len());
    let end = (start + len).min(out.len());
    let held = if start == 0 { 0.0 } else { out[start - 1] };
    for v in &mut out[start..end] {
        *v = held;
    }
    out
}

/// フィードフォワード・エコー: `y[n] = x[n] + g * x[n - delay]`。
pub fn echo(x: &[f64], delay: usize, gain: f64) -> Vec<f64> {
    let mut out = x.to_vec();
    for i in delay..out.len() {
        out[i] += gain * x[i - delay];
    }
    out
}

/// フィードバック・コムフィルタ: `y[n] = x[n] + g * y[n - delay]`。
pub fn comb(x: &[f64], delay: usize, gain: f64) -> Vec<f64> {
    let delay = delay.max(1);
    let mut out = x.to_vec();
    for i in delay..out.len() {
        out[i] += gain * out[i - delay];
    }
    out
}

/// ゼロ詰めアップサンプル(イメージング)。`factor` 倍の長さで、
/// 元サンプル間に `factor - 1` 個のゼロを挿入する。フィルタを
/// 掛けないのでスペクトルにイメージが残る。
pub fn zero_stuff(x: &[f64], factor: usize) -> Vec<f64> {
    let factor = factor.max(1);
    let mut out = vec![0.0; x.len() * factor];
    for (i, &v) in x.iter().enumerate() {
        out[i * factor] = v;
    }
    out
}

/// RBJ バイクアッドの係数。全て決定的。
#[derive(Debug, Clone, Copy)]
pub struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

impl Biquad {
    fn new(b0: f64, b1: f64, b2: f64, a0: f64, a1: f64, a2: f64) -> Self {
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// ローパス。`q` = 0.7071 で Butterworth 相当。
    pub fn lowpass(sample_rate: u32, freq_hz: f64, q: f64) -> Self {
        let w = 2.0 * std::f64::consts::PI * freq_hz / f64::from(sample_rate);
        let alpha = w.sin() / (2.0 * q.max(1e-6));
        let cw = w.cos();
        Self::new(
            (1.0 - cw) / 2.0,
            1.0 - cw,
            (1.0 - cw) / 2.0,
            1.0 + alpha,
            -2.0 * cw,
            1.0 - alpha,
        )
    }

    /// ハイパス。
    pub fn highpass(sample_rate: u32, freq_hz: f64, q: f64) -> Self {
        let w = 2.0 * std::f64::consts::PI * freq_hz / f64::from(sample_rate);
        let alpha = w.sin() / (2.0 * q.max(1e-6));
        let cw = w.cos();
        Self::new(
            (1.0 + cw) / 2.0,
            -(1.0 + cw),
            (1.0 + cw) / 2.0,
            1.0 + alpha,
            -2.0 * cw,
            1.0 - alpha,
        )
    }

    /// ピーキング EQ。`gain_db` で `freq_hz` 付近を増減させる。
    pub fn peaking(sample_rate: u32, freq_hz: f64, gain_db: f64, q: f64) -> Self {
        let w = 2.0 * std::f64::consts::PI * freq_hz / f64::from(sample_rate);
        let alpha = w.sin() / (2.0 * q.max(1e-6));
        let cw = w.cos();
        let a = 10f64.powf(gain_db / 40.0);
        Self::new(
            1.0 + alpha * a,
            -2.0 * cw,
            1.0 - alpha * a,
            1.0 + alpha / a,
            -2.0 * cw,
            1.0 - alpha / a,
        )
    }

    /// 1 次オールパス風バイクアッド。`|H| = 1` で位相のみ変える。
    /// `a` は -1..1 の係数。
    pub fn allpass(a: f64) -> Self {
        // y[n] = a*x[n] + x[n-1] - a*y[n-1] をバイクアッド形で表す
        // (b2 = 0): H(z) = (a + z^-1) / (1 + a z^-1)
        Self::new(a, 1.0, 0.0, 1.0, a, 0.0)
    }

    /// ダイレクトフォーム I でフィルタリングする。
    pub fn apply(&self, x: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; x.len()];
        let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
        for (i, &v) in x.iter().enumerate() {
            let y = self.b0 * v + self.b1 * x1 + self.b2 * x2 - self.a1 * y1 - self.a2 * y2;
            out[i] = y;
            x2 = x1;
            x1 = v;
            y2 = y1;
            y1 = y;
        }
        out
    }
}

/// チャンネル列をインターリーブする。各チャンネルの長さは同じこと。
/// 長いチャンネルの末尾は切り捨てる。
pub fn interleave(channels: &[Vec<f64>]) -> Vec<f64> {
    let frames = channels.iter().map(Vec::len).min().unwrap_or(0);
    let mut out = Vec::with_capacity(frames * channels.len());
    for i in 0..frames {
        for ch in channels {
            out.push(ch[i]);
        }
    }
    out
}

/// インターリーブ済みサンプルをチャンネル毎に分解する。
/// 末尾の不完全フレームは捨てる。
pub fn deinterleave(x: &[f64], channels: u16) -> Vec<Vec<f64>> {
    let channels = usize::from(channels);
    if channels == 0 {
        return Vec::new();
    }
    let frames = x.len() / channels;
    let mut out = vec![Vec::with_capacity(frames); channels];
    for frame in x.chunks_exact(channels) {
        for (ch, &v) in frame.iter().enumerate() {
            out[ch].push(v);
        }
    }
    out
}

/// チャンネルミックス: `out_ch[c][i] = Σ_s matrix[c][s] * in_ch[s][i]`。
/// `matrix` は `[出力ch][入力ch]` の係数行列。
pub fn mix_channels(channels: &[Vec<f64>], matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let frames = channels.iter().map(Vec::len).min().unwrap_or(0);
    matrix
        .iter()
        .map(|row| {
            (0..frames)
                .map(|i| {
                    row.iter()
                        .zip(channels)
                        .map(|(&m, ch)| m * ch[i])
                        .sum::<f64>()
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_deterministic_and_bounded() {
        let a = noise(256, 42, 1.0);
        let b = noise(256, 42, 1.0);
        assert_eq!(a, b);
        assert!(a.iter().all(|v| v.abs() <= 1.0));
        assert!(a.iter().any(|&v| v != 0.0));
        let c = noise(256, 43, 1.0);
        assert_ne!(a, c);
    }

    #[test]
    fn delay_shifts_with_zero_fill() {
        let x = vec![1.0, 2.0, 3.0, 4.0];
        assert_eq!(delay(&x, 2), vec![0.0, 0.0, 1.0, 2.0]);
        assert_eq!(delay(&x, 0), x);
    }

    #[test]
    fn frac_delay_recovers_half_sample() {
        let x = vec![0.0, 1.0, 0.0, -1.0, 0.0];
        let y = frac_delay(&x, 1.0);
        assert_eq!(y[0], 0.0);
        assert_eq!(y[1], 0.0);
        assert!((y[2] - 1.0).abs() < 1e-12);
        let h = frac_delay(&x, 1.5);
        assert!((h[2] - 0.5).abs() < 1e-12);
    }

    #[test]
    fn resample_linear_halves_and_doubles() {
        let x: Vec<f64> = (0..8).map(|i| i as f64).collect();
        // ratio 2: 時間圧縮 — out[i] = x[2i]
        let half = resample_linear(&x, 2.0);
        assert_eq!(half.len(), 4);
        assert_eq!(half, vec![0.0, 2.0, 4.0, 6.0]);
        // ratio 0.5: 伸長 — 奇数番目は中間値
        let double = resample_linear(&x, 0.5);
        assert!((double[3] - 1.5).abs() < 1e-12);
    }

    #[test]
    fn block_transforms_have_expected_lengths() {
        let x: Vec<f64> = (0..100).map(|i| i as f64).collect();
        assert_eq!(drop_block(&x, 10, 20).len(), 80);
        assert_eq!(drop_block(&x, 10, 20)[10], x[30]);
        let z = zero_block(&x, 10, 20);
        assert_eq!(z.len(), 100);
        assert!(z[10..30].iter().all(|&v| v == 0.0));
        let r = repeat_block(&x, 10, 20);
        assert_eq!(r.len(), 120);
        assert_eq!(r[30], x[10]);
        let s = stuck_block(&x, 10, 20);
        assert!(s[10..30].iter().all(|&v| v == x[9]));
        assert_eq!(s[30], x[30]);
    }

    #[test]
    fn echo_and_comb_insert_delayed_copy() {
        let mut x = vec![0.0; 16];
        x[0] = 1.0;
        let e = echo(&x, 4, 0.5);
        assert_eq!(e[0], 1.0);
        assert_eq!(e[4], 0.5);
        let c = comb(&x, 4, 0.5);
        assert_eq!(c[4], 0.5);
        assert!((c[8] - 0.25).abs() < 1e-12);
    }

    #[test]
    fn biquad_lowpass_attenuates_high_band() {
        let rate = 48_000u32;
        let lo = sine(1_000.0, 4096, rate, 1.0);
        let hi = sine(15_000.0, 4096, rate, 1.0);
        let lp = Biquad::lowpass(rate, 4_000.0, 0.7071);
        let lo_out = lp.apply(&lo);
        let hi_out = lp.apply(&hi);
        let e = |v: &[f64]| v.iter().map(|x| x * x).sum::<f64>();
        // 通過域はほぼ保存、阻止域は強く減衰
        assert!(e(&lo_out) > e(&lo) * 0.8);
        assert!(e(&hi_out) < e(&hi) * 0.01);
    }

    #[test]
    fn allpass_preserves_magnitude_roughly() {
        let rate = 48_000u32;
        let x = noise(8192, 7, 1.0);
        let y = Biquad::allpass(0.5).apply(&x);
        let rms = |v: &[f64]| (v.iter().map(|a| a * a).sum::<f64>() / v.len() as f64).sqrt();
        // オールパスはエネルギーをほぼ保存する(定常状態で)
        assert!((rms(&x) - rms(&y)).abs() < rms(&x) * 0.05);
        let _ = rate;
    }

    #[test]
    fn interleave_and_mix_round_trip() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![10.0, 20.0, 30.0];
        let i = interleave(&[a.clone(), b.clone()]);
        assert_eq!(i, vec![1.0, 10.0, 2.0, 20.0, 3.0, 30.0]);
        assert_eq!(deinterleave(&i, 2), vec![a.clone(), b.clone()]);
        // スワップ行列
        let m = mix_channels(&[a.clone(), b.clone()], &[vec![0.0, 1.0], vec![1.0, 0.0]]);
        assert_eq!(m[0], b);
        assert_eq!(m[1], a);
    }
}
