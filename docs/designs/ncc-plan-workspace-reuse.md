---
id: "DESIGN-034FPWNHT7czdzzGSNBU2Q"
type: "design"
status: "approved"
title: "NCC plan/workspace 分離と2サイト再利用設計"
created: "2026-10-14"
updated: "2026-10-14"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/ncc-plan-workspace-reuse.md"
    - "docs/adr/ncc-plan-workspace-reuse.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# NCC plan/workspace 分離と2サイト再利用設計

## Context and Scope

反復相関の不変係数と排他的scratchを分離し、band_gain::windowsとcheap IPD driftの
2サイトだけを移行する。先にIPD caller検索窓とlag原点の不具合を独立commitで修正する。
今回spec/ADRは意思決定のみ承認済み。本designはdraftで動的実証は未実施。
新依存、TLS/pool、al-locate移行、FFT kernel交換、並列実行機能追加はない。

## Components and Public API

### FFT ownership boundary

`RealFftPlan::new(n)` は既存契約どおり非ゼロpow2をassertする。
不変 n、Fft、roots のみを持ち Send + Sync とする。`RealFftWorkspace::new(&plan)` は
packing scratchと、大変換のstage用skew scratchをconstructorで全確保する。
`m=max(n/2,1)` としpacking領域は `2*m+WS_SKEW`、stage領域は
`m>=WS_MIN(16384)` のとき `2*m+WS_SKEW`、それ未満は空。
2領域はforward/inverse時に同時使用するため別領域とし、演算時resizeしない。
長さnと各必要領域長でworkspaceを検証する。
`plan.forward(&mut workspace, input, out_re, out_im)` と inverse を提供し、
入力n・出力n/2+1のsplit re/im、1/N正規化、DC/Nyquist虚部無視、ISA選択を維持する。
不一致は既存FFT同様panic、演算前にassertする。

### Large FFT explicit workspace boundary

現 RealTransform は forward/inverse とも `transform_ws` に入り、m>=16384時に
FFT_WORK TLSをborrow/resizeする。packing scratch分離だけではN=262144の初回に
約2MiBのhidden stage領域が確保されスレッドに残るため、本scopeで実FFT経路の責務も修復する。

`transform_explicit_ws<S: Simd>(simd, fft, re, im, stage_ws, inverse)` を内部primitiveとする。
小shapeは既存 `transform`、大shapeはstage_wsのskew re/imへ `stage_out` +
`butterfly_from` を実行し、既存 `gather_permute` でwriteback/scaleする。
StageRun::with_simdのstage本体を小内部 `run_stages<S>` primitiveへ抽出し、
explicit pathと既存StageRunが同じprimitiveへ委譲する。stage順序、twiddle、
DIF/digit-reverse置換、inverse scaleの場所/値を変えない。

外側RealTransform/Correlationの `dispatch` で得た同じS/simdをprimitiveへ渡す。
TLS closure外のexplicit pathは追加dispatchせず、inline generic呼出しでISA文脈を保つ。
既存Fft::transformのTLS pathはそのままStageRunをdispatchし、共通primitiveへ委譲する。
backend key/公開ISA enum/ISA別plan等の過剰APIは追加しない。

RealFftPlan forward/inverseとCorrelationPlanは全shapeでexplicit pathを通り、
FFT_WORK.with/RefCell borrow/resizeを呼ばない。所有型RealFftの便利wrapperもこのpathへ委譲する。
対象外 `Fft::transform` のTLS全廃・全caller移行・kernel交換は行わない。

既存 `RealFft::new/forward/inverse` はplan+workspaceを所有する便利な構成として維持し、
新planのauthoritative変換処理へ委譲する。別実装やdeprecation互換layerは作らない。
これにより移行対象外のFFT callerを変更せず、実装された規則は1箇所に集約する。

### Correlation ownership boundary

`CorrelationPlan::new(template_len, signal_len)` はexactshapeと必要なRealFftPlanを持つ。
FFT長は `(signal_len + template_len - 1).next_power_of_two()`、mode/max_lagはplanに含めない。
空template/短signalは演算不要shapeとしてFFT planを省略する。長さ演算overflowはchecked算術で
constructorがpanicする。外部データ検証は既存stage境界の責務であり新fallbackは作らない。

`CorrelationWorkspace::new(&plan)` はshape情報、FFT workspace、real長n、4本のspectrum、
template square、signal square/cumulative、norms、PHAT energy、result領域を一度確保する。
各bufferはplanに必要な最大サイズで保持、実行中resize/reallocateしない。

```rust
impl CorrelationPlan {
    pub fn ncc<'w>(&self, ws: &'w mut CorrelationWorkspace,
        template: &[f64], signal: &[f64], max_lag: usize) -> &'w [f64];
    pub fn gcc_phat<'w>(&self, ws: &'w mut CorrelationWorkspace,
        template: &[f64], signal: &[f64], max_lag: usize) -> &'w [f64];
}
```

2methodは同一kernelへのruntime PHAT指定として共有する。modeごとの別planを作らない。
workspace/inputのexactshape不一致は処理前assertでpanic。前回結果やpartial出力を返さない。
workspaceは同一shapeの別planでも利用可能。shared `&plan` + exclusive `&mut ws` で所有権を表現し、
resultの借用中はworkspaceを再利用できない。returned sliceはws由来で入力/planの寿命に依存しない。
利用側が保存するときだけcopyする。workspaceをclone/shared-lock化しない。

既存 ncc/gcc_phat/fft_ncc は同じplan/workspace pathを呼んでowned Vecへcopyする便利な経路として
維持する。hot callerはborrowed curveをその場で消費しcurve copyをしない。
同じ責務のnormalize/cross-spectrumを別実装にしない。

## Numerical and Boundary Contracts

max_lagはusize（負のAPI入力は存在しない）。実効lagは既存と同じ
`min(max_lag, signal_len-template_len)`、出力indexはsignal内の非負offset。
max_lag=0および同長shapeは合法な候補1件を返す。signed負lagはcallerの窓原点で表す。
空template/短signalはempty、NCCのゼロエネルギーは0、PHATのゼロcross binは0。
NCCの符号・極性を維持し、候補順位/validityは利用側の責務。

実行ごとreal padding、cumulative先頭、使用領域のsquare/energy/norms、result範囲を上書き/clearする。
非empty→silent→PHAT→NCC→異なるmax_lagの連続再利用で前回state混入を検証する。
FFT係数生成、padding、reduction順序、FMA形は変更しない。
FFT/NCC/PHATの小shape独立直接計算oracleは既存 `1e-10` absolute基準を維持する。
大shapeは既存scalar/SIMD・goldenの許容誤差を引き継ぎ、合格のため閾値を緩めない。
freewrapper対reuseだけでoracleを代替せず、signed峰位置・反転極性も別assertする。

## IPD Independent Fix Before Reuse

現callerは同長blockをNCCへ渡し合法clamp=0となる。index0からmax_lagを引くと
各block同じ偽lagでdrift0になる。責務はcaller検索窓/座標でありNCCclampは変更しない。

`max = round(ipd_max_lag_s*4*rate)` の現非負usize変換を維持する。負設定は既存float→usize
飽和で0となるため、このIssueで新しい設定許可/拒否policyを追加しない。
100ms block長は既存max(64)を維持、startはmaxから `n-max-block` までblock間隔、
左右marginを完全に持つblockだけを分析する。checked/sub条件で短入力はlag観測なし。
`template=x[start..start+block]`、`signal=y[start-max..start+block+max]`、
requested max_lag=2*max。signed lagは `peak_index-max`。
max=0はmarginなし合法、n==blockも1blockを分析する。

小helper `ipd_block_lag(template, search, origin)` はNCCを呼び既存のsigned-correlation
max選択規則を保持する（band_gainのabsolute峰/tie policyをIPDへ移植しない）。
originの責務はcheap内で一元化し、誤変換を複数箇所に実装しない。
非周期広帯域fixtureの2区間lag -3/+5→drift8を修正前red/修正後greenにする。
helperでsigned原点、一定lag、max0、短入力/insufficient marginを確認する。
IPD先行fix+回帰+証拠はplan APIを含めない独立commitにする。

## Two Hot Caller Lifetimes

band_gain windows: short-input early return後にshape(window,window+2*max)のplan/wsを作成し、
window iteratorの外で保持。guard、signedlag、abs-corr tie、polarity/gain/validityは維持。
結果はclosure内で消費して次iteration前に借用を終了する。bandをまたぐcacheは追加しない。

cheap IPD: 先行fix後の同一full-margin block shapeでplan/wsをblock loop外に作成。
helperのsigned原点計算は維持し、曲線生成だけreuse APIへ置換する。
両siteともplan構築は呼出単位で1回、workspace確保は1回。各scope終了時解放。
グローバル共有cacheやstage間受渡しは追加しない。

## Verification and Measurement Fixtures

| 項目 | 条件/保証 |
| --- | --- |
| shape/境界 | n=1/2/64/4096、odd input lengths、empty/short/silent、max0/大requested clamp、shape mismatch panic、再利用混入なし |
| 同値 | deterministic非周期noise(seed0x52)、±極性、既知offset、NCC/PHAT独立oracle/既存golden、既存誤差を維持 |
| ownership | `assert_send_sync::<RealFftPlan/CorrelationPlan>()`、Arc共有plan + scoped threads各ws、逐次と同じcurve/peak |
| IPD | rate48000/block4800/max192、非周期noiseの区間-3/+5→8、固定lag、max0、短入力、signed原点単体 |
| allocation first use | fresh thread上で入力/出力/plan/wsと計数器を構築し、その後warmupなしのfirst forward/inverse、first NCC/PHATを計数。各phase alloc/realloc=0を要求。modeごとfresh wsで検証 |
| allocation reuse/drop | NCC/PHAT各100回もalloc/realloc=0。2siteはplan構築1・ws初期1、loop内0。ws dropでpacking/stage/correlation bufferの所有分deallocを追跡しTLSに残らないことを確認 |
| large threshold | RealFFT N=16384/32768/65536/262144（internal m=8192/16384/32768/131072）。threshold直前/閾値/上、large NCC/PHATはFFT N=262144を含める。fresh threadのfirst useで検証 |
| timing/RSS | release・同一host/toolchain/ISA、input noise seed0x52、cold plan+ws+firstcall、steady reuse100call×30batch、p50/p95、各phase RSS/peak、shape/回数をartifactへ記録 |

測定shape: band_gain window=4800/max=240(signal5280)、IPD block4800/max192(signal5184)、
large nonpow2 template96000/signal100800。PHAT/NCC両方、signal構築はtimed scope外。
実caller経路は10秒stereo非周期fixtureと48kHz rateで計測し、baseline/reuseの同一fixture・設定を保存する。
allocator計数は独立test/benchの測定scopeだけを対象にし、他threadの確保やassert/出力の確保を除外する。
fresh threadの入力/出力と所有資源の構築は計数開始前、計数器初期化も開始前。
OS allocatorがfree領域を保持するためdrop後RSS低下を解放のassertに使わず、所有領域deallocの
証拠とRSSを分けて報告する。packing/stageを含む容量、構築時bytes、drop時解放bytesを保存する。
RSSは既存Windows GetProcess情報/標準shell手段で採取し依存を追加しない。不能なら未計測としAC未達を報告。

baselineはIPD独立fix後・reuse導入前commitをpinし、bug修正によるworkload変更をreuse効果と混同しない。
同一hot-path手順で計測し、実測ratioを報告。3x/10%等をこのIssueの未承認gateとして追加しない。
確保0と正しさが必須、性能回帰が再現すれば原因調査して改善効果を主張しない。
#42再ベースライン/rustfft判断はこのIssue完了後の別作業。

## Rollback and Risks

先行IPD修正を独立commitに保つため、reuseだけ逆適用しても正しいlag計測が残る。
workspaceのbuffer初期化漏れ、oversized shape、borrow lifetimeが主risk。境界テストと
明示assertで所有者が保証する。FFTwrapperの既存callerを全移行しない。

## Next Checkpoint

design review承認後、1planにIPD先行fix・math所有分離・2site移行/測定の依存順を具体化する。
plan承認前にtaskは生成しない。動的実証をdesign承認として先取りしない。

## 2026-10-15 Design Review Correction 1

Reviewer A/Bが同rootのP1を提示、ValidatorがCONFIRMED、Judge BLOCKED。
RealFFTの隠れたlarge-stage TLS workspaceを見落としたため、所有境界を上記の通り修復した。
packingだけでなくskew stage領域もconstructorが所有し、explicit realFFT/correlation経路はTLSを
使わず、fresh threadのwarmupなしfirstcall・閾値/large shape・dropを検証する。
今回修正はdesignのみ、動的実証未実施。第2reviewへ提出する。

## 2026-10-15 Design Review 2

A/B candidates各0、Validator追加0、Judge APPROVED/ACCEPT、Blocker0。
前P1のTLS workspace責務はdesign上resolved。実装のfirstcold/解放/等価性/性能は未検証。

## 2026-10-15 Façade適用境界

al-analysisは既存al_locate::dsp経由で数学APIを消費する。新depを追加せず
crates/al-locate/src/dsp.rsの既存pubuseへCorrelationPlan/Workspace2公開型を1行追加する。
coordinatorが既存backend中立境界の最小拡張と確認。al-locatecallerの移行はしない。
