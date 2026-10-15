//! 2 のべき乗長の複素 FFT と実数 FFT。
//! 長さ調整とゼロ埋めは呼び出し側が所有する。
//! 複素数は実部・虚部を分離した等長スライス(split re/im)で受け渡す。

use crate::{F64s, Simd, WithSimd, dispatch};
use std::f64::consts::TAU;

/// 変換長ごとの不変係数。複数の変換・スレッドで再利用できる。
pub struct Fft {
    n: usize,
    // stage ごとに [w1 re | pad | w1 im | pad | w2 re | pad | w2 im | pad]。
    // split re/im 化で同時アクセスが 8 データ + 4 twiddle ストリームに増え、
    // 4K 境界で揃う quarter stride が同一キャッシュセットへ集中するため、
    // pad で twiddle ストリームを別セットへずらす。
    twiddles: Vec<f64>,
}

// 副表間のパディング要素数。64B ずらしで別の L1 セットに乗せる。
const TW_PAD: usize = 8;
// 1 stage の twiddle セグメント長。副表は q = len/4 要素 + pad。
#[inline(always)]
fn tw_segment(q: usize) -> usize {
    4 * (q + TW_PAD)
}
// セグメント内の副表オフセット。[w1r|w1i|w2r|w2i] が pad 区切りで並ぶ。
#[inline(always)]
fn tw_offset(idx: usize, q: usize) -> usize {
    idx * (q + TW_PAD)
}

fn roots(n: usize, count: usize) -> (Vec<f64>, Vec<f64>) {
    let angle = -TAU / n as f64;
    let step_re = angle.cos();
    let step_im = angle.sin();
    let mut re = Vec::with_capacity(count);
    let mut im = Vec::with_capacity(count);
    let (mut w_re, mut w_im) = (1.0, 0.0);
    for k in 0..count {
        // 長い逐次再帰の振幅・位相ドリフトを抑える。
        if k % 256 == 0 {
            let a = angle * k as f64;
            w_re = a.cos();
            w_im = a.sin();
        }
        re.push(w_re);
        im.push(w_im);
        // w *= step。複素乗算と同じ乗算・加算順を保つ。
        let next_re = w_re * step_re - w_im * step_im;
        w_im = w_re * step_im + w_im * step_re;
        w_re = next_re;
    }
    (re, im)
}

impl Fft {
    /// 非ゼロの 2 のべき乗長を受ける。不正長は panic する。
    pub fn new(n: usize) -> Self {
        assert!(
            n.is_power_of_two(),
            "FFT length must be a nonzero power of two"
        );
        let mut twiddles = Vec::new();
        let mut len = n;
        while len > 2 {
            let q = len / 4;
            // w1 = roots(len)[0..q]、w2 = roots(len/2)[0..q] (= w_len^{2k})。
            let (w1r, w1i) = roots(len, q);
            let (w2r, w2i) = roots(len / 2, q);
            for part in [&w1r, &w1i, &w2r, &w2i] {
                twiddles.extend(part.iter().copied());
                twiddles.resize(twiddles.len() + TW_PAD, 0.0);
            }
            len /= 4;
        }
        Self { n, twiddles }
    }

    /// 自然順序の in-place 変換。forward は負符号、inverse は正符号と 1/N。
    /// `re`/`im` は等長で、長さが生成時の長さと異なる場合は panic する。
    pub fn transform(&self, re: &mut [f64], im: &mut [f64], inverse: bool) {
        assert_eq!(re.len(), im.len(), "FFT re/im buffer length mismatch");
        assert_eq!(re.len(), self.n, "FFT buffer length mismatch");
        dispatch(Transform {
            plan: self,
            re,
            im,
            inverse,
        });
    }
}

struct Transform<'a> {
    plan: &'a Fft,
    re: &'a mut [f64],
    im: &'a mut [f64],
    inverse: bool,
}

impl WithSimd for Transform<'_> {
    type Output = ();
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) {
        transform_ws(simd, self.plan, self.re, self.im, self.inverse);
    }
}

// workspace 経路を使う最小長。小さな変換は L1/L2 に収まり skew の
// 効果より確保・複写コストが支配的になるため in-place のままとする。
const WS_MIN: usize = 1 << 14;
// workspace の im 部を re からずらす要素数。512B で 4K 境界の一致を解く。
const WS_SKEW: usize = 64;

thread_local! {
    // 変換ごとの再利用可能な作業領域。呼び出し側配列と独立しているため
    // skew した配置を保証できる。
    static FFT_WORK: std::cell::RefCell<Vec<f64>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[inline(always)]
fn transform_ws<S: Simd>(simd: S, plan: &Fft, re: &mut [f64], im: &mut [f64], inverse: bool) {
    let n = re.len();
    if n < WS_MIN {
        transform(simd, plan, re, im, inverse);
        return;
    }
    // 呼び出し側の re/im が同一ページオフセットに揃うと、大きな len の
    // stage で quarter stride が 4K の倍数となり、8 本のデータストリームが
    // 同一 L1/L2 セットへ集中する。skew した workspace で stage を回し、
    // DIF 出力の digit-reversed 置換を gather 書き戻しに畳み込む。
    FFT_WORK.with(|work| {
        let mut ws = work.borrow_mut();
        let need = 2 * n + WS_SKEW;
        // re/im 領域は stage_out が読み出しより先に全要素を書き込み、
        // pad 領域は読まれないため、成長分のゼロ初期化だけで足りる。
        if ws.len() < need {
            ws.resize(need, 0.0);
        }
        let (wre, rest) = ws.split_at_mut(n + WS_SKEW);
        let wim = &mut rest[..n];
        let wre = &mut wre[..n];
        // thread_local の借用を保持した closure 内で直接 SIMD カーネルを
        // 呼ぶと target_feature 文脈が切れてベクトル命令がスカラー化する。
        // stage 実行は再度 dispatch し、正しい ISA コンテキストで行う。
        dispatch(StageRun {
            plan,
            src_re: re,
            src_im: im,
            ws_re: wre,
            ws_im: wim,
            inverse,
        });
        let scale = if inverse { 1.0 / n as f64 } else { 1.0 };
        gather_permute(wre, wim, re, im, scale);
    });
}

// workspace への転送込みで stage を実行する再 dispatch 用。
// 先頭 stage は入力側配列から直接読む out-of-place で行い、複写を省く。
// 置換とスケールは呼び出し側の gather 書き戻しへ畳み込んである。
struct StageRun<'a> {
    plan: &'a Fft,
    src_re: &'a [f64],
    src_im: &'a [f64],
    ws_re: &'a mut [f64],
    ws_im: &'a mut [f64],
    inverse: bool,
}

impl WithSimd for StageRun<'_> {
    type Output = ();
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) {
        run_stages(
            simd,
            self.plan,
            self.src_re,
            self.src_im,
            self.ws_re,
            self.ws_im,
            self.inverse,
        );
    }
}

#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn run_stages<S: Simd>(
    simd: S,
    plan: &Fft,
    src_re: &[f64],
    src_im: &[f64],
    wre: &mut [f64],
    wim: &mut [f64],
    inverse: bool,
) {
    let n = src_re.len();
    let tw = &plan.twiddles;
    if inverse {
        stage_out::<S, true>(simd, src_re, src_im, wre, wim, tw, n);
        butterfly_from::<S, true>(simd, wre, wim, &tw[tw_segment(n / 4)..], n / 4);
    } else {
        stage_out::<S, false>(simd, src_re, src_im, wre, wim, tw, n);
        butterfly_from::<S, false>(simd, wre, wim, &tw[tw_segment(n / 4)..], n / 4);
    }
}

#[inline(always)]
fn transform_explicit_ws<S: Simd>(
    simd: S,
    plan: &Fft,
    re: &mut [f64],
    im: &mut [f64],
    stage: &mut [f64],
    inverse: bool,
) {
    let n = re.len();
    if n < WS_MIN {
        transform(simd, plan, re, im, inverse);
        return;
    }
    let (wre, rest) = stage.split_at_mut(n + WS_SKEW);
    let wre = &mut wre[..n];
    let wim = &mut rest[..n];
    run_stages(simd, plan, re, im, wre, wim, inverse);
    let scale = if inverse { 1.0 / n as f64 } else { 1.0 };
    gather_permute(wre, wim, re, im, scale);
}

// workspace 上の digit-reversed 出力を呼び出し側配列へ書き戻す。
// in-place 置換の代わりに sequential write + gather read で済ませ、
// inverse の 1/N スケールもここへ畳み込む。読み出し側は in-place 置換と
// 同じタイル順で局所性を保つ。
fn gather_permute(wre: &[f64], wim: &[f64], re: &mut [f64], im: &mut [f64], scale: f64) {
    let n = re.len();
    // workspace 経路 (n >= WS_MIN) からのみ呼ばれるため、タイル化の
    // 前提は n >= 1024。小さな変換は in-place `transform` が置換を担う。
    debug_assert!(n >= 1024);
    let middle_bits = n.trailing_zeros() - 10;
    for middle in 0..(n >> 10) {
        let reversed = if middle_bits == 0 {
            0
        } else {
            middle.reverse_bits() >> (usize::BITS - middle_bits)
        };
        for high in 0usize..32 {
            let low_out = high.reverse_bits() >> (usize::BITS - 5);
            for low in 0usize..32 {
                let high_out = low.reverse_bits() >> (usize::BITS - 5);
                let i = (high << (middle_bits + 5)) | (middle << 5) | low;
                let j = (high_out << (middle_bits + 5)) | (reversed << 5) | low_out;
                re[i] = wre[j] * scale;
                im[i] = wim[j] * scale;
            }
        }
    }
}

// (ar·br − ai·bi, ar·bi + ai·br) の複素乗算。pulp `mul_c64s` と同系の
// FMA 融合形だが融合する側が逆で、こちらは ar·br 側でなく ai·bi 側
// (実部) / ai·br 側 (虚部) を融合するため、mul_c64s とは 1ulp 程度の
// 差があり得る。実用上の差は許容範囲でこの形が速いため、性能優先で
// 意図的に採用している。
#[inline(always)]
fn cmul<S: Simd>(
    simd: S,
    ar: F64s<S>,
    ai: F64s<S>,
    br: F64s<S>,
    bi: F64s<S>,
) -> (F64s<S>, F64s<S>) {
    (
        simd.negate_mul_add_f64s(ai, bi, simd.mul_f64s(ar, br)),
        simd.mul_add_f64s(ar, bi, simd.mul_f64s(ai, br)),
    )
}

// 上記と同じ乗算・加算順の scalar 版。
#[inline(always)]
fn cmul1(ar: f64, ai: f64, br: f64, bi: f64) -> (f64, f64) {
    (ar * br - ai * bi, ar * bi + ai * br)
}

#[inline(always)]
fn transform<S: Simd>(simd: S, plan: &Fft, re: &mut [f64], im: &mut [f64], inverse: bool) {
    // 内側ループの符号分岐を monomorphize で除去する。
    if inverse {
        butterfly::<S, true>(simd, re, im, &plan.twiddles);
    } else {
        butterfly::<S, false>(simd, re, im, &plan.twiddles);
    }
    let n = re.len();
    if n < 1024 {
        for i in 1..n {
            let j = i.reverse_bits() >> (usize::BITS - n.trailing_zeros());
            if i < j {
                swap_pair(re, im, i, j);
            }
        }
    } else {
        // 高位/低位 5 bit をタイル化し、相互の行をキャッシュに留める。
        let middle_bits = n.trailing_zeros() - 10;
        for middle in 0..(n >> 10) {
            let reversed = if middle_bits == 0 {
                0
            } else {
                middle.reverse_bits() >> (usize::BITS - middle_bits)
            };
            if middle > reversed {
                continue;
            }
            for high in 0usize..32 {
                let low_out = high.reverse_bits() >> (usize::BITS - 5);
                for low in 0usize..32 {
                    let high_out = low.reverse_bits() >> (usize::BITS - 5);
                    let i = (high << (middle_bits + 5)) | (middle << 5) | low;
                    let j = (high_out << (middle_bits + 5)) | (reversed << 5) | low_out;
                    if middle < reversed || i < j {
                        swap_pair(re, im, i, j);
                    }
                }
            }
        }
    }
    if inverse {
        let scale = simd.splat_f64s(1.0 / n as f64);
        let (vectors, tail) = S::as_mut_simd_f64s(re);
        for v in vectors {
            *v = simd.mul_f64s(*v, scale);
        }
        for v in tail {
            *v /= n as f64;
        }
        let (vectors, tail) = S::as_mut_simd_f64s(im);
        for v in vectors {
            *v = simd.mul_f64s(*v, scale);
        }
        for v in tail {
            *v /= n as f64;
        }
    }
}

// re/im の同一 index スワップ。等しいページオフセットを持つ 2 配列では
// `re.swap` のストア直後の `im` ロードが 4K エイリアシングを起こすため、
// 両配列のロードを先に発行してからストアする。
#[inline(always)]
fn swap_pair(re: &mut [f64], im: &mut [f64], i: usize, j: usize) {
    let ar = re[i];
    let br = re[j];
    let ai = im[i];
    let bi = im[j];
    re[i] = br;
    im[i] = bi;
    re[j] = ar;
    im[j] = ai;
}

// SIMD 演算を dispatch の ISA コンテキストへインラインする。
#[inline(always)]
fn butterfly<S: Simd, const INVERSE: bool>(simd: S, re: &mut [f64], im: &mut [f64], tw: &[f64]) {
    butterfly_from::<S, INVERSE>(simd, re, im, tw, re.len());
}

// len から開始する stage 列。workspace 経路では先頭 stage が
// out-of-place のため、残りを len = n/4 から始める。
#[inline(always)]
fn butterfly_from<S: Simd, const INVERSE: bool>(
    simd: S,
    re: &mut [f64],
    im: &mut [f64],
    mut tw: &[f64],
    mut len: usize,
) {
    while len > 1024 {
        stage::<S, INVERSE>(simd, re, im, tw, len);
        tw = &tw[tw_segment(len / 4)..];
        len /= 4;
    }
    for (block_re, block_im) in re.chunks_exact_mut(len).zip(im.chunks_exact_mut(len)) {
        let mut rest = tw;
        let mut size = len;
        while size > 1 {
            stage::<S, INVERSE>(simd, block_re, block_im, rest, size);
            let used = if size == 2 { 0 } else { tw_segment(size / 4) };
            rest = &rest[used..];
            size = if size == 2 { 1 } else { size / 4 };
        }
    }
}

// radix-2 の 2 段を融合する。出力の組順は 0,2,1,3 で binary bit reversal を保つ。
#[inline(always)]
fn stage<S: Simd, const INVERSE: bool>(
    simd: S,
    re: &mut [f64],
    im: &mut [f64],
    tw: &[f64],
    len: usize,
) {
    if len == 2 {
        for (pair_re, pair_im) in re
            .as_chunks_mut::<2>()
            .0
            .iter_mut()
            .zip(im.as_chunks_mut::<2>().0.iter_mut())
        {
            let ar = pair_re[0];
            let ai = pair_im[0];
            pair_re[0] = ar + pair_re[1];
            pair_im[0] = ai + pair_im[1];
            pair_re[1] = ar - pair_re[1];
            pair_im[1] = ai - pair_im[1];
        }
        return;
    }
    let q = len / 4;
    let w1r = &tw[tw_offset(0, q)..tw_offset(0, q) + q];
    let w1i = &tw[tw_offset(1, q)..tw_offset(1, q) + q];
    let w2r = &tw[tw_offset(2, q)..tw_offset(2, q) + q];
    let w2i = &tw[tw_offset(3, q)..tw_offset(3, q) + q];
    let (w1rv, _) = S::as_simd_f64s(w1r);
    let (w1iv, _) = S::as_simd_f64s(w1i);
    let (w2rv, _) = S::as_simd_f64s(w2r);
    let (w2iv, _) = S::as_simd_f64s(w2i);
    // forward は -i、inverse は +i の回転。
    let rotate = simd.splat_f64s(if INVERSE { 1.0 } else { -1.0 });
    let neg_rotate = simd.neg_f64s(rotate);
    for (block_re, block_im) in re.chunks_exact_mut(len).zip(im.chunks_exact_mut(len)) {
        let (a_re, tail) = block_re.split_at_mut(q);
        let (b_re, tail) = tail.split_at_mut(q);
        let (c_re, d_re) = tail.split_at_mut(q);
        let (a_im, tail) = block_im.split_at_mut(q);
        let (b_im, tail) = tail.split_at_mut(q);
        let (c_im, d_im) = tail.split_at_mut(q);
        let (arv, art) = S::as_mut_simd_f64s(a_re);
        let (aiv, ait) = S::as_mut_simd_f64s(a_im);
        let (brv, brt) = S::as_mut_simd_f64s(b_re);
        let (biv, bit) = S::as_mut_simd_f64s(b_im);
        let (crv, crt) = S::as_mut_simd_f64s(c_re);
        let (civ, cit) = S::as_mut_simd_f64s(c_im);
        let (drv, drt) = S::as_mut_simd_f64s(d_re);
        let (div, dit) = S::as_mut_simd_f64s(d_im);
        // 8 データ + 4 twiddle の多重 zip はイテレータごとのポインタ更新で
        // レジスタを圧迫するため、全て同長のベクトル列を index で共有する。
        let nv = arv.len();
        assert_eq!(aiv.len(), nv);
        assert_eq!(brv.len(), nv);
        assert_eq!(biv.len(), nv);
        assert_eq!(crv.len(), nv);
        assert_eq!(civ.len(), nv);
        assert_eq!(drv.len(), nv);
        assert_eq!(div.len(), nv);
        assert_eq!(w1rv.len(), nv);
        assert_eq!(w1iv.len(), nv);
        assert_eq!(w2rv.len(), nv);
        assert_eq!(w2iv.len(), nv);
        for k in 0..nv {
            let ar = &mut arv[k];
            let ai = &mut aiv[k];
            let br = &mut brv[k];
            let bi = &mut biv[k];
            let cr = &mut crv[k];
            let ci = &mut civ[k];
            let dr = &mut drv[k];
            let di = &mut div[k];
            let w1i = if INVERSE {
                simd.neg_f64s(w1iv[k])
            } else {
                w1iv[k]
            };
            let w2i = if INVERSE {
                simd.neg_f64s(w2iv[k])
            } else {
                w2iv[k]
            };
            let w1r = w1rv[k];
            let w2r = w2rv[k];
            let s0r = simd.add_f64s(*ar, *cr);
            let s0i = simd.add_f64s(*ai, *ci);
            let s1r = simd.add_f64s(*br, *dr);
            let s1i = simd.add_f64s(*bi, *di);
            let d0r = simd.sub_f64s(*ar, *cr);
            let d0i = simd.sub_f64s(*ai, *ci);
            let e1r = simd.sub_f64s(*br, *dr);
            let e1i = simd.sub_f64s(*bi, *di);
            // ±i 回転: forward は (e1i, −e1r)、inverse は (−e1i, e1r)。
            let d1r = simd.mul_f64s(e1i, neg_rotate);
            let d1i = simd.mul_f64s(e1r, rotate);
            *ar = simd.add_f64s(s0r, s1r);
            *ai = simd.add_f64s(s0i, s1i);
            let (vr, vi) = cmul(
                simd,
                simd.sub_f64s(s0r, s1r),
                simd.sub_f64s(s0i, s1i),
                w2r,
                w2i,
            );
            *br = vr;
            *bi = vi;
            let (vr, vi) = cmul(
                simd,
                simd.add_f64s(d0r, d1r),
                simd.add_f64s(d0i, d1i),
                w1r,
                w1i,
            );
            *cr = vr;
            *ci = vi;
            let (w12r, w12i) = cmul(simd, w1r, w1i, w2r, w2i);
            let (vr, vi) = cmul(
                simd,
                simd.sub_f64s(d0r, d1r),
                simd.sub_f64s(d0i, d1i),
                w12r,
                w12i,
            );
            *dr = vr;
            *di = vi;
        }
        let first = q - art.len();
        let sign = if INVERSE { 1.0 } else { -1.0 };
        for (k, (((((((ar, ai), br), bi), cr), ci), dr), di)) in art
            .iter_mut()
            .zip(ait)
            .zip(brt)
            .zip(bit)
            .zip(crt)
            .zip(cit)
            .zip(drt)
            .zip(dit)
            .enumerate()
        {
            let k = first + k;
            let (w1rk, w1ik) = if INVERSE {
                (w1r[k], -w1i[k])
            } else {
                (w1r[k], w1i[k])
            };
            let (w2rk, w2ik) = if INVERSE {
                (w2r[k], -w2i[k])
            } else {
                (w2r[k], w2i[k])
            };
            let s0r = *ar + *cr;
            let s0i = *ai + *ci;
            let s1r = *br + *dr;
            let s1i = *bi + *di;
            let d0r = *ar - *cr;
            let d0i = *ai - *ci;
            let e1r = *br - *dr;
            let e1i = *bi - *di;
            let d1r = -sign * e1i;
            let d1i = sign * e1r;
            *ar = s0r + s1r;
            *ai = s0i + s1i;
            let (vr, vi) = cmul1(s0r - s1r, s0i - s1i, w2rk, w2ik);
            *br = vr;
            *bi = vi;
            let (vr, vi) = cmul1(d0r + d1r, d0i + d1i, w1rk, w1ik);
            *cr = vr;
            *ci = vi;
            let (vr, vi) = cmul1(d0r - d1r, d0i - d1i, w1rk, w1ik);
            let (vr, vi) = cmul1(vr, vi, w2rk, w2ik);
            *dr = vr;
            *di = vi;
        }
    }
}

// `stage` と同じ演算を src → dst の out-of-place で行う。workspace 経路の
// 先頭 stage で使い、入力側配列からの複写 pass を省く。
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn stage_out<S: Simd, const INVERSE: bool>(
    simd: S,
    src_re: &[f64],
    src_im: &[f64],
    dst_re: &mut [f64],
    dst_im: &mut [f64],
    tw: &[f64],
    len: usize,
) {
    // workspace 経路の先頭 stage (len = n >= WS_MIN) でのみ使われるため
    // radix-4 に限定する。len == 2 の終端は in-place `stage` 側が担う。
    debug_assert!(len >= 4);
    let q = len / 4;
    let w1r = &tw[tw_offset(0, q)..tw_offset(0, q) + q];
    let w1i = &tw[tw_offset(1, q)..tw_offset(1, q) + q];
    let w2r = &tw[tw_offset(2, q)..tw_offset(2, q) + q];
    let w2i = &tw[tw_offset(3, q)..tw_offset(3, q) + q];
    let (w1rv, _) = S::as_simd_f64s(w1r);
    let (w1iv, _) = S::as_simd_f64s(w1i);
    let (w2rv, _) = S::as_simd_f64s(w2r);
    let (w2iv, _) = S::as_simd_f64s(w2i);
    // forward は -i、inverse は +i の回転。
    let rotate = simd.splat_f64s(if INVERSE { 1.0 } else { -1.0 });
    let neg_rotate = simd.neg_f64s(rotate);
    for (((block_src_re, block_src_im), block_dst_re), block_dst_im) in src_re
        .chunks_exact(len)
        .zip(src_im.chunks_exact(len))
        .zip(dst_re.chunks_exact_mut(len))
        .zip(dst_im.chunks_exact_mut(len))
    {
        let (s_a_re, tail) = block_src_re.split_at(q);
        let (s_b_re, tail) = tail.split_at(q);
        let (s_c_re, s_d_re) = tail.split_at(q);
        let (s_a_im, tail) = block_src_im.split_at(q);
        let (s_b_im, tail) = tail.split_at(q);
        let (s_c_im, s_d_im) = tail.split_at(q);
        let (d_a_re, tail) = block_dst_re.split_at_mut(q);
        let (d_b_re, tail) = tail.split_at_mut(q);
        let (d_c_re, d_d_re) = tail.split_at_mut(q);
        let (d_a_im, tail) = block_dst_im.split_at_mut(q);
        let (d_b_im, tail) = tail.split_at_mut(q);
        let (d_c_im, d_d_im) = tail.split_at_mut(q);
        let (arv, art) = S::as_simd_f64s(s_a_re);
        let (aiv, ait) = S::as_simd_f64s(s_a_im);
        let (brv, brt) = S::as_simd_f64s(s_b_re);
        let (biv, bit) = S::as_simd_f64s(s_b_im);
        let (crv, crt) = S::as_simd_f64s(s_c_re);
        let (civ, cit) = S::as_simd_f64s(s_c_im);
        let (drv, drt) = S::as_simd_f64s(s_d_re);
        let (div, dit) = S::as_simd_f64s(s_d_im);
        let (oarv, oart) = S::as_mut_simd_f64s(d_a_re);
        let (oaiv, oait) = S::as_mut_simd_f64s(d_a_im);
        let (obrv, obrt) = S::as_mut_simd_f64s(d_b_re);
        let (obiv, obit) = S::as_mut_simd_f64s(d_b_im);
        let (ocrv, ocrt) = S::as_mut_simd_f64s(d_c_re);
        let (ociv, ocit) = S::as_mut_simd_f64s(d_c_im);
        let (odrv, odrt) = S::as_mut_simd_f64s(d_d_re);
        let (odiv, odit) = S::as_mut_simd_f64s(d_d_im);
        let nv = arv.len();
        let first = q - art.len();
        for k in 0..nv {
            let ar = arv[k];
            let ai = aiv[k];
            let br = brv[k];
            let bi = biv[k];
            let cr = crv[k];
            let ci = civ[k];
            let dr = drv[k];
            let di = div[k];
            let w1i = if INVERSE {
                simd.neg_f64s(w1iv[k])
            } else {
                w1iv[k]
            };
            let w2i = if INVERSE {
                simd.neg_f64s(w2iv[k])
            } else {
                w2iv[k]
            };
            let w1r = w1rv[k];
            let w2r = w2rv[k];
            let s0r = simd.add_f64s(ar, cr);
            let s0i = simd.add_f64s(ai, ci);
            let s1r = simd.add_f64s(br, dr);
            let s1i = simd.add_f64s(bi, di);
            let d0r = simd.sub_f64s(ar, cr);
            let d0i = simd.sub_f64s(ai, ci);
            let e1r = simd.sub_f64s(br, dr);
            let e1i = simd.sub_f64s(bi, di);
            let d1r = simd.mul_f64s(e1i, neg_rotate);
            let d1i = simd.mul_f64s(e1r, rotate);
            oarv[k] = simd.add_f64s(s0r, s1r);
            oaiv[k] = simd.add_f64s(s0i, s1i);
            let (vr, vi) = cmul(
                simd,
                simd.sub_f64s(s0r, s1r),
                simd.sub_f64s(s0i, s1i),
                w2r,
                w2i,
            );
            obrv[k] = vr;
            obiv[k] = vi;
            let (vr, vi) = cmul(
                simd,
                simd.add_f64s(d0r, d1r),
                simd.add_f64s(d0i, d1i),
                w1r,
                w1i,
            );
            ocrv[k] = vr;
            ociv[k] = vi;
            let (w12r, w12i) = cmul(simd, w1r, w1i, w2r, w2i);
            let (vr, vi) = cmul(
                simd,
                simd.sub_f64s(d0r, d1r),
                simd.sub_f64s(d0i, d1i),
                w12r,
                w12i,
            );
            odrv[k] = vr;
            odiv[k] = vi;
        }
        // src/dst で scalar 端数の位置がずれないよう要素 index を揃える。
        let sign = if INVERSE { 1.0 } else { -1.0 };
        for (k, ((((((((&ar, &ai), &br), &bi), &cr), &ci), &dr), &di), dst)) in art
            .iter()
            .zip(ait)
            .zip(brt)
            .zip(bit)
            .zip(crt)
            .zip(cit)
            .zip(drt)
            .zip(dit)
            .zip(
                oart.iter_mut()
                    .zip(oait)
                    .zip(obrt)
                    .zip(obit)
                    .zip(ocrt)
                    .zip(ocit)
                    .zip(odrt)
                    .zip(odit),
            )
            .enumerate()
        {
            let (((((((oar, oai), obr), obi), ocr), oci), odr), odi) = dst;
            let k = first + k;
            let (w1rk, w1ik) = if INVERSE {
                (w1r[k], -w1i[k])
            } else {
                (w1r[k], w1i[k])
            };
            let (w2rk, w2ik) = if INVERSE {
                (w2r[k], -w2i[k])
            } else {
                (w2r[k], w2i[k])
            };
            let s0r = ar + cr;
            let s0i = ai + ci;
            let s1r = br + dr;
            let s1i = bi + di;
            let d0r = ar - cr;
            let d0i = ai - ci;
            let e1r = br - dr;
            let e1i = bi - di;
            let d1r = -sign * e1i;
            let d1i = sign * e1r;
            *oar = s0r + s1r;
            *oai = s0i + s1i;
            let (vr, vi) = cmul1(s0r - s1r, s0i - s1i, w2rk, w2ik);
            *obr = vr;
            *obi = vi;
            let (vr, vi) = cmul1(d0r + d1r, d0i + d1i, w1rk, w1ik);
            *ocr = vr;
            *oci = vi;
            let (vr, vi) = cmul1(d0r - d1r, d0i - d1i, w1rk, w1ik);
            let (vr, vi) = cmul1(vr, vi, w2rk, w2ik);
            *odr = vr;
            *odi = vi;
        }
    }
}

/// 実数 FFT の不変係数。複数スレッドで共有できる。
/// 非負周波数 N/2+1 bins (N=1 は 1 bin) を split re/im で扱う。
pub struct RealFftPlan {
    n: usize,
    fft: Fft,
    roots: Vec<f64>,
}

/// 実数 FFT の排他的作業領域。変換時の確保や TLS 借用は不要。
pub struct RealFftWorkspace {
    n: usize,
    scratch: Vec<f64>,
    stage: Vec<f64>,
}

impl RealFftWorkspace {
    /// plan に必要な packing と大変換用 stage 領域を全確保する。
    pub fn new(plan: &RealFftPlan) -> Self {
        let m = (plan.n / 2).max(1);
        Self {
            n: plan.n,
            scratch: vec![0.0; 2 * m + WS_SKEW],
            stage: vec![0.0; if m >= WS_MIN { 2 * m + WS_SKEW } else { 0 }],
        }
    }
}

/// 不変 plan と排他的 workspace を所有する実数 FFT。
pub struct RealFft {
    plan: RealFftPlan,
    workspace: RealFftWorkspace,
}

impl RealFft {
    /// 非ゼロの 2 のべき乗長を受ける。不正長は panic する。
    pub fn new(n: usize) -> Self {
        let plan = RealFftPlan::new(n);
        let workspace = RealFftWorkspace::new(&plan);
        Self { plan, workspace }
    }
    /// 実数から非負周波数へ変換する。
    pub fn forward(&mut self, input: &[f64], out_re: &mut [f64], out_im: &mut [f64]) {
        self.plan
            .forward(&mut self.workspace, input, out_re, out_im);
    }
    /// 非負周波数から実数へ 1/N 正規化して復元する。
    pub fn inverse(&mut self, in_re: &[f64], in_im: &[f64], output: &mut [f64]) {
        self.plan.inverse(&mut self.workspace, in_re, in_im, output);
    }
    #[inline(always)]
    pub(crate) fn forward_with_simd<S: Simd>(
        &mut self,
        simd: S,
        input: &[f64],
        out_re: &mut [f64],
        out_im: &mut [f64],
    ) {
        self.plan
            .forward_with_simd(simd, &mut self.workspace, input, out_re, out_im);
    }
    #[inline(always)]
    pub(crate) fn inverse_with_simd<S: Simd>(
        &mut self,
        simd: S,
        in_re: &[f64],
        in_im: &[f64],
        output: &mut [f64],
    ) {
        self.plan
            .inverse_with_simd(simd, &mut self.workspace, in_re, in_im, output);
    }
}

impl RealFftPlan {
    /// 非ゼロの 2 のべき乗長を受ける。不正長は panic する。
    pub fn new(n: usize) -> Self {
        assert!(
            n.is_power_of_two(),
            "real FFT length must be a nonzero power of two"
        );
        let (re, im) = roots(n, n / 2 + 1);
        let m = (n / 2).max(1);
        let mut roots = Vec::with_capacity(2 * (n / 2 + 1) + TW_PAD);
        // 組立て係数 (0.5·im, −0.5·re) を split 格納する。
        roots.extend(im.iter().map(|&w| 0.5 * w));
        roots.resize(roots.len() + TW_PAD, 0.0);
        roots.extend(re.iter().map(|&w| -0.5 * w));
        Self {
            n,
            fft: Fft::new(m),
            roots,
        }
    }

    #[inline(always)]
    fn validate_workspace(&self, workspace: &RealFftWorkspace) {
        let m = (self.n / 2).max(1);
        assert_eq!(workspace.n, self.n, "real FFT workspace shape mismatch");
        assert_eq!(workspace.scratch.len(), 2 * m + WS_SKEW);
        assert_eq!(
            workspace.stage.len(),
            if m >= WS_MIN { 2 * m + WS_SKEW } else { 0 }
        );
    }

    /// 実数から非負周波数へ変換する。入力/出力長が不一致なら panic する。
    pub fn forward(
        &self,
        workspace: &mut RealFftWorkspace,
        input: &[f64],
        out_re: &mut [f64],
        out_im: &mut [f64],
    ) {
        self.validate_workspace(workspace);
        assert_eq!(input.len(), self.n);
        assert_eq!(out_re.len(), self.n / 2 + 1);
        assert_eq!(out_im.len(), self.n / 2 + 1);
        dispatch(RealTransform {
            plan: self,
            workspace,
            real: RealBuffer::Input(input),
            spectrum: SpectrumBuffer::Output(out_re, out_im),
        });
    }

    /// 非負周波数から実数へ 1/N 正規化して復元する。
    /// DC/Nyquist の虚部は使用しない。入力/出力長が不一致なら panic する。
    /// `in_re` と `in_im` は同一スライスを共有してはならない。
    pub fn inverse(
        &self,
        workspace: &mut RealFftWorkspace,
        in_re: &[f64],
        in_im: &[f64],
        output: &mut [f64],
    ) {
        self.validate_workspace(workspace);
        assert_eq!(output.len(), self.n);
        assert_eq!(in_re.len(), self.n / 2 + 1);
        assert_eq!(in_im.len(), self.n / 2 + 1);
        debug_assert!(
            !std::ptr::eq(in_re, in_im),
            "real FFT inverse re/im slices must not be shared"
        );
        dispatch(RealTransform {
            plan: self,
            workspace,
            real: RealBuffer::Output(output),
            spectrum: SpectrumBuffer::Input(in_re, in_im),
        });
    }

    // 複合カーネルは一度選択した ISA を全変換で共有する。
    #[inline(always)]
    pub(crate) fn forward_with_simd<S: Simd>(
        &self,
        simd: S,
        workspace: &mut RealFftWorkspace,
        input: &[f64],
        out_re: &mut [f64],
        out_im: &mut [f64],
    ) {
        self.validate_workspace(workspace);
        assert_eq!(input.len(), self.n);
        assert_eq!(out_re.len(), self.n / 2 + 1);
        assert_eq!(out_im.len(), self.n / 2 + 1);
        RealTransform {
            plan: self,
            workspace,
            real: RealBuffer::Input(input),
            spectrum: SpectrumBuffer::Output(out_re, out_im),
        }
        .with_simd(simd);
    }

    #[inline(always)]
    pub(crate) fn inverse_with_simd<S: Simd>(
        &self,
        simd: S,
        workspace: &mut RealFftWorkspace,
        in_re: &[f64],
        in_im: &[f64],
        output: &mut [f64],
    ) {
        self.validate_workspace(workspace);
        assert_eq!(output.len(), self.n);
        assert_eq!(in_re.len(), self.n / 2 + 1);
        assert_eq!(in_im.len(), self.n / 2 + 1);
        debug_assert!(
            !std::ptr::eq(in_re, in_im),
            "real FFT inverse re/im slices must not be shared"
        );
        RealTransform {
            plan: self,
            workspace,
            real: RealBuffer::Output(output),
            spectrum: SpectrumBuffer::Input(in_re, in_im),
        }
        .with_simd(simd);
    }
}

enum RealBuffer<'a> {
    Input(&'a [f64]),
    Output(&'a mut [f64]),
}
enum SpectrumBuffer<'a> {
    Input(&'a [f64], &'a [f64]),
    Output(&'a mut [f64], &'a mut [f64]),
}
struct RealTransform<'a> {
    plan: &'a RealFftPlan,
    workspace: &'a mut RealFftWorkspace,
    real: RealBuffer<'a>,
    spectrum: SpectrumBuffer<'a>,
}

impl WithSimd for RealTransform<'_> {
    type Output = ();
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) {
        let p = self.plan;
        let n1 = p.n == 1;
        let m = p.n / 2;
        // im 副表は re の直後 + TW_PAD に続く。
        let RealFftPlan { fft, roots, .. } = p;
        let RealFftWorkspace { scratch, stage, .. } = self.workspace;
        let roots_re = &roots[..m + 1];
        let roots_im = &roots[m + 1 + TW_PAD..m + 1 + TW_PAD + m + 1];
        let (sre, sim_) = {
            let (re, rest) = scratch.split_at_mut(m);
            (re, &mut rest[WS_SKEW..WS_SKEW + m])
        };
        match (self.real, self.spectrum) {
            (RealBuffer::Input(input), SpectrumBuffer::Output(out_re, out_im)) => {
                if n1 {
                    out_re[0] = input[0];
                    out_im[0] = 0.0;
                    return;
                }
                // 偶数標本を re、奇数標本を im へ詰め込む。
                for i in 0..m {
                    sre[i] = input[2 * i];
                    sim_[i] = input[2 * i + 1];
                }
                transform_explicit_ws(simd, fft, sre, sim_, stage, false);
                let zr = sre[0];
                let zi = sim_[0];
                out_re[0] = zr + zi;
                out_im[0] = 0.0;
                out_re[m] = zr - zi;
                out_im[m] = 0.0;
                if m > 1 {
                    let mid = m / 2;
                    out_re[mid] = sre[mid];
                    out_im[mid] = -sim_[mid];
                    let (low_re, high_re) = out_re[1..m].split_at_mut(mid);
                    let (low_im, high_im) = out_im[1..m].split_at_mut(mid);
                    let low_re = &mut low_re[..mid - 1];
                    let low_im = &mut low_im[..mid - 1];
                    // 共役対を low に先行充填し、recombine_pair で上書きする。
                    for ((out_r, out_i), (&zr, &zi)) in
                        low_re.iter_mut().zip(low_im.iter_mut()).zip(
                            sre[mid + 1..m]
                                .iter()
                                .rev()
                                .zip(sim_[mid + 1..m].iter().rev()),
                        )
                    {
                        *out_r = zr;
                        *out_i = -zi;
                    }
                    recombine_pair(
                        simd,
                        &sre[1..mid],
                        &sim_[1..mid],
                        &roots_re[1..mid],
                        &roots_im[1..mid],
                        low_re,
                        low_im,
                        high_re,
                        high_im,
                    );
                    high_re.reverse();
                    high_im.reverse();
                }
            }
            (RealBuffer::Output(output), SpectrumBuffer::Input(in_re, in_im)) => {
                if n1 {
                    output[0] = in_re[0];
                    return;
                }
                sre[0] = (in_re[0] + in_re[m]) * 0.5;
                sim_[0] = (in_re[0] - in_re[m]) * 0.5;
                for ((zr, zi), (&xr, &xi)) in sre[1..m]
                    .iter_mut()
                    .zip(sim_[1..m].iter_mut())
                    .zip(in_re[1..m].iter().rev().zip(in_im[1..m].iter().rev()))
                {
                    *zr = xr;
                    *zi = -xi;
                }
                recombine(
                    simd,
                    &in_re[1..m],
                    &in_im[1..m],
                    &roots_re[1..m],
                    &roots_im[1..m],
                    &mut sre[1..m],
                    &mut sim_[1..m],
                    true,
                );
                transform_explicit_ws(simd, fft, sre, sim_, stage, true);
                for i in 0..m {
                    output[2 * i] = sre[i];
                    output[2 * i + 1] = sim_[i];
                }
            }
            _ => unreachable!(),
        }
    }
}

// 共役対を一度に求め、和/差と twiddle 積を共有する。
#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn recombine_pair<S: Simd>(
    simd: S,
    in_re: &[f64],
    in_im: &[f64],
    roots_re: &[f64],
    roots_im: &[f64],
    low_re: &mut [f64],
    low_im: &mut [f64],
    high_re: &mut [f64],
    high_im: &mut [f64],
) {
    let half = simd.splat_f64s(0.5);

    let (arv, art) = S::as_simd_f64s(in_re);
    let (aiv, ait) = S::as_simd_f64s(in_im);
    let (wrv, wrt) = S::as_simd_f64s(roots_re);
    let (wiv, wit) = S::as_simd_f64s(roots_im);
    let (brv, brt) = S::as_mut_simd_f64s(low_re);
    let (biv, bit) = S::as_mut_simd_f64s(low_im);
    let (hrv, hrt) = S::as_mut_simd_f64s(high_re);
    let (hiv, hit) = S::as_mut_simd_f64s(high_im);
    for (((((((&ar, &ai), &wr), &wi), br), bi), hr), hi) in arv
        .iter()
        .zip(aiv)
        .zip(wrv)
        .zip(wiv)
        .zip(brv)
        .zip(biv)
        .zip(hrv)
        .zip(hiv)
    {
        let sum_r = simd.mul_f64s(simd.add_f64s(ar, *br), half);
        let sum_i = simd.mul_f64s(simd.add_f64s(ai, *bi), half);
        let (dfr, dfi) = cmul(simd, wr, wi, simd.sub_f64s(ar, *br), simd.sub_f64s(ai, *bi));
        *br = simd.add_f64s(sum_r, dfr);
        *bi = simd.add_f64s(sum_i, dfi);
        // high 側は conj(sum − diff)。
        *hr = simd.sub_f64s(sum_r, dfr);
        *hi = simd.sub_f64s(dfi, sum_i);
    }
    for (((((((&ar, &ai), &wr), &wi), br), bi), hr), hi) in art
        .iter()
        .zip(ait)
        .zip(wrt)
        .zip(wit)
        .zip(brt)
        .zip(bit)
        .zip(hrt)
        .zip(hit)
    {
        let sum_r = (ar + *br) * 0.5;
        let sum_i = (ai + *bi) * 0.5;
        let (dfr, dfi) = cmul1(wr, wi, ar - *br, ai - *bi);
        *br = sum_r + dfr;
        *bi = sum_i + dfi;
        *hr = sum_r - dfr;
        *hi = dfi - sum_i;
    }
}
// mirrored は反転済み共役 spectrum。前後方向とも同一の復元規則を使う。
#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn recombine<S: Simd>(
    simd: S,
    in_re: &[f64],
    in_im: &[f64],
    roots_re: &[f64],
    roots_im: &[f64],
    mir_re: &mut [f64],
    mir_im: &mut [f64],
    inverse: bool,
) {
    let half = simd.splat_f64s(0.5);

    let (arv, art) = S::as_simd_f64s(in_re);
    let (aiv, ait) = S::as_simd_f64s(in_im);
    let (wrv, wrt) = S::as_simd_f64s(roots_re);
    let (wiv, wit) = S::as_simd_f64s(roots_im);
    let (brv, brt) = S::as_mut_simd_f64s(mir_re);
    let (biv, bit) = S::as_mut_simd_f64s(mir_im);
    for (((((&ar, &ai), &wr), &wi), br), bi) in
        arv.iter().zip(aiv).zip(wrv).zip(wiv).zip(brv).zip(biv)
    {
        let wi = if inverse { simd.neg_f64s(wi) } else { wi };
        let (dfr, dfi) = cmul(simd, wr, wi, simd.sub_f64s(ar, *br), simd.sub_f64s(ai, *bi));
        *br = simd.add_f64s(simd.mul_f64s(simd.add_f64s(ar, *br), half), dfr);
        *bi = simd.add_f64s(simd.mul_f64s(simd.add_f64s(ai, *bi), half), dfi);
    }
    for (((((&ar, &ai), &wr), &wi), br), bi) in
        art.iter().zip(ait).zip(wrt).zip(wit).zip(brt).zip(bit)
    {
        let wi = if inverse { -wi } else { wi };
        let (dfr, dfi) = cmul1(wr, wi, ar - *br, ai - *bi);
        *br = (ar + *br) * 0.5 + dfr;
        *bi = (ai + *bi) * 0.5 + dfi;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scalar_and_dispatch_share_the_kernel() {
        for power in 0..=10 {
            let n = 1 << power;
            let plan = Fft::new(n);
            for inverse in [false, true] {
                let mut native_re: Vec<_> = (0..n).map(|i| (i as f64 * 0.7).sin()).collect();
                let mut native_im: Vec<_> = (0..n).map(|i| (i as f64 * 0.3).cos()).collect();
                let mut scalar_re = native_re.clone();
                let mut scalar_im = native_im.clone();
                plan.transform(&mut native_re, &mut native_im, inverse);
                pulp::Scalar.vectorize(Transform {
                    plan: &plan,
                    re: &mut scalar_re,
                    im: &mut scalar_im,
                    inverse,
                });
                for ((&ar, &ai), (&br, &bi)) in native_re
                    .iter()
                    .zip(&native_im)
                    .zip(scalar_re.iter().zip(&scalar_im))
                {
                    let diff = (ar - br).abs() + (ai - bi).abs();
                    let norm = br.abs() + bi.abs();
                    assert!(diff < 1e-10 * (1.0 + norm));
                }
            }
            let input: Vec<_> = (0..n).map(|i| (i as f64 * 0.7).sin()).collect();
            let mut plan = RealFft::new(n);
            let mut native_re = vec![0.0; n / 2 + 1];
            let mut native_im = vec![0.0; n / 2 + 1];
            let mut scalar_re = native_re.clone();
            let mut scalar_im = native_im.clone();
            plan.forward(&input, &mut native_re, &mut native_im);
            pulp::Scalar.vectorize(RealTransform {
                plan: &plan.plan,
                workspace: &mut plan.workspace,
                real: RealBuffer::Input(&input),
                spectrum: SpectrumBuffer::Output(&mut scalar_re, &mut scalar_im),
            });
            for ((&ar, &ai), (&br, &bi)) in native_re
                .iter()
                .zip(&native_im)
                .zip(scalar_re.iter().zip(&scalar_im))
            {
                let diff = (ar - br).abs() + (ai - bi).abs();
                let norm = br.abs() + bi.abs();
                assert!(diff < 1e-10 * (1.0 + norm));
            }
            let mut output = vec![0.0; n];
            pulp::Scalar.vectorize(RealTransform {
                plan: &plan.plan,
                workspace: &mut plan.workspace,
                real: RealBuffer::Output(&mut output),
                spectrum: SpectrumBuffer::Input(&scalar_re, &scalar_im),
            });
            for (a, b) in output.iter().zip(&input) {
                assert!((a - b).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn ws_path_matches_scalar_and_survives_size_changes() {
        // n >= WS_MIN の変換は skew 付き thread_local workspace 経路を通る。
        // Scalar 強制と dispatch の一致、および連続する異サイズ変換での
        // workspace 再利用・再割当てが正しいことを見る。
        for power in [14, 16, 15] {
            let n = 1 << power;
            let plan = Fft::new(n);
            for inverse in [false, true] {
                let mut native_re: Vec<_> = (0..n).map(|i| (i as f64 * 0.7).sin()).collect();
                let mut native_im: Vec<_> = (0..n).map(|i| (i as f64 * 0.3).cos()).collect();
                let mut scalar_re = native_re.clone();
                let mut scalar_im = native_im.clone();
                plan.transform(&mut native_re, &mut native_im, inverse);
                pulp::Scalar.vectorize(Transform {
                    plan: &plan,
                    re: &mut scalar_re,
                    im: &mut scalar_im,
                    inverse,
                });
                for ((&ar, &ai), (&br, &bi)) in native_re
                    .iter()
                    .zip(&native_im)
                    .zip(scalar_re.iter().zip(&scalar_im))
                {
                    let diff = (ar - br).abs() + (ai - bi).abs();
                    let norm = br.abs() + bi.abs();
                    assert!(
                        diff < 1e-10 * (1.0 + norm),
                        "n={n}: ({ar},{ai}) != ({br},{bi})"
                    );
                }
            }
        }
    }

    #[test]
    fn concurrent_large_transforms_use_thread_local_workspace() {
        // n >= WS_MIN の変換を並行しても workspace はスレッドごとに独立。
        let plan = Fft::new(1 << 15);
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(|| {
                    let mut re = vec![0.0; 1 << 15];
                    let mut im = vec![0.0; 1 << 15];
                    re[0] = 1.0;
                    plan.transform(&mut re, &mut im, false);
                    assert_eq!(re, vec![1.0; 1 << 15]);
                    assert_eq!(im, vec![0.0; 1 << 15]);
                });
            }
        });
    }
}
