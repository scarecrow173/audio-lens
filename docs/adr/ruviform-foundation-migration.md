---
id: "ADR-034P3nB0CZJO83ZX105Cfq"
type: "adr"
status: "accepted"
title: "汎用メディア I/O・DSP 基盤を Ruviform に移し AudioLens は解析意味論とデータ所有に集中する"
created: "2026-10-30"
updated: "2026-10-31"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/95"
    - "https://github.com/scarecrow173/audio-lens/issues/97"
    - "https://github.com/scarecrow173/ruviform/issues/150"
    - "https://github.com/scarecrow173/ruviform/issues/151"
    - "https://github.com/scarecrow173/ruviform/issues/152"
    - "https://github.com/scarecrow173/ruviform/issues/153"
    - "https://github.com/scarecrow173/ruviform/issues/154"
    - "https://github.com/scarecrow173/ruviform/issues/155"
    - "https://github.com/scarecrow173/ruviform/issues/156"
    - "https://github.com/scarecrow173/ruviform/issues/157"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/ruviform-foundation-migration.md"
  implemented-by:
    - "../designs/ruviform-migration-responsibility-map.md"
    - "../plans/ruviform-foundation-migration.md"
  depends-on:
    - "measurement-over-verdict.md"
    - "immutable-resumable-job.md"
    - "analysis-dataset-storage.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "observability-dedicated-crate.md"
    - "logging-sink-format-activation-contract.md"
    - "stage-timing-manifest-persistence.md"
    - "artifact-durability-tiers-relaxed-bulk-writes.md"
    - "artifact-id-read-only-pcm-view-cache.md"
    - "multi-method-locate-consensus.md"
    - "locate-argmax-ambiguity-representation.md"
    - "simd-reduction-lane-fma.md"
    - "ncc-plan-workspace-reuse.md"
    - "ncc-valid-fft-plan-gcc-phat-plan.md"
    - "ncc-plan-time-dispatch-issue-71-p3.md"
    - "lag-al-math-plan.md"
    - "al-math-api-fft-split-re-im.md"
    - "al-math-vec-primitives-slice-boundary.md"
    - "caller-side-optimizations-structure.md"
    - "bandfilter-inputcache.md"
    - "spectral-envelope-fft-size-assert.md"
    - "spec-residual-contract-finalization.md"
    - "residual-issue-boundaries.md"
    - "locate-plan-run-pool.md"
  refines:
    - "implementation-stack.md"
    - "media-decoder-backend.md"
    - "mp4-demux-decode.md"
    - "avi.md"
    - "audio-fallback-pipeline-native-container-facts-symphonia-decode.md"
    - "simd-arithmetic-pulp.md"
    - "derived-visualization-dataset.md"
  refined-by: []
  derives-from:
    - "../specs/ruviform-foundation-migration.md"
    - "https://github.com/scarecrow173/audio-lens/issues/95"
  derived-by:
    - "../designs/ruviform-migration-responsibility-map.md"
    - "../plans/ruviform-foundation-migration.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 汎用メディア I/O・DSP 基盤を Ruviform に移し AudioLens は解析意味論とデータ所有に集中する

## Context and Problem Statement

AudioLens は `implementation-stack.md` の「Rust を採用しコア処理を
自前モジュールで実装する」方針のもと、メディア demux/decode・
タイミング観測・f64 DSP カーネル・相関演算を `al-media` /
`al-math` / `al-locate` に自前実装してきた。一方、同じ共通基盤を
他ツールへ再利用可能な形で所有するリポジトリ Ruviform
(`scarecrow173/ruviform`) が存在し、移行追跡 Issue
[#95](https://github.com/scarecrow173/audio-lens/issues/95) で
「処理単位で汎用化可能な機能は Ruviform が所有し、ゲーム音声
デバッグ特有のワークフロー・解釈・永続化は AudioLens が所有する」
方針が採用済みである。本 ADR はその採用済み方針を正式な
アーキテクチャ決定として記録し、どの責務をどちらが所有するかの
判定基準・精度/RT 契約・既存 ADR との整合を固定する。処理単位の
責務マップと段階 DAG の正本は
`../designs/ruviform-migration-responsibility-map.md` と
`../plans/ruviform-foundation-migration.md` が担う。

## Decision Drivers

- 採用済み方針(Issue #95「採用済みの方針」7 項目、Issue #97
  「決定済みの前提」5 項目)は再投票しない。
- f64 解析精度と数値意味論を維持する。f32 への縮退、暗黙の
  f64→f32 変換、元 PCM 尺度の消失を認めない。
- メディア I/O で失われがちな観測事実(元 sample format / bit
  depth / raw scale、raw PTS/DTS、time base、container/stream/
  packet/decode 情報、不連続・欠落の判定材料)を Ruviform 側の
  取得契約として確保する。
- 依存方向は Ruviform → AudioLens の一方向。Ruviform に AudioLens
  固有の Job・Locate・Validity/Confidence・イベント意味付け・
  画面構成を持ち込まない。
- 移行は段階的で、現行アルゴリズム・実サンプル・Golden・性能を
  比較基準として保全し、差分が説明できてから旧実装を削除する。
- Ruviform 側に責務がある問題を AudioLens 側だけの恒久的な
  回避策や重複実装で隠さない(AGENTS.md マルチリポジトリ規約)。

## Considered Options

- A: 現行方針を維持し、コア処理を引き続き AudioLens 自前
  モジュールで実装・拡張する
- B: 処理単位で責務を分割し、汎用メディア I/O・数値・DSP・
  表示プリミティブ・実行基盤は Ruviform が所有し、AudioLens は
  解析意味論・Job・Dataset・CLI/GUI アプリを所有する
- C: AudioLens を薄い呼び出し層にし、locate 合議や解析 stage を
  含むパイプライン全体を Ruviform 側へ移す
- D: 所有権を決めず、都合のよい部分だけ Ruviform を利用する
  併用運用

## Decision Outcome

Chosen option: "B: 処理単位の責務分割", because Issue #95 で採用
済みの方針であり、汎用基盤の再利用性と AudioLens 固有の解析
意味論の独立性を両立できるため。

具体的な採用決定(Issue #97 の決定済み前提に一致):

1. 高精度 f64 オフライン API を Ruviform に追加する
   (Ruviform #150)。現在の f32/RT API は退行させない。
2. Media I/O は Ruviform 経由とし、解析に必要な raw
   timing/PCM/codec/container 観測情報は Ruviform に拡張する
   (Ruviform #151/#154)。AudioLens は解析意味論と結果の永続化を
   所有する。
3. 移植判定は crate 名ではなく処理の汎用性・不変条件・依存方向で
   行う。ゲーム音声デバッグ固有の処理は AudioLens に残す。
4. 現行 `al-viz` / Visualization Dataset / 静的 HTML レポートは
   最終的に廃止し、Ruviform GUI 基盤(#156/#157)で AudioLens の
   GUI アプリケーション(#105)を実装する。Analysis Dataset・中間
   成果物・Provenance・可搬な解析データ export は保持する。
5. Ruviform を GUI なしの CLI/headless アプリ構築にも利用できる
   ようにし(#155)、AudioLens の GUI と CLI が共通の非 UI 処理層に
   依存する。

### 責務分割の判定基準

- Ruviform が所有するもの: 用途に依存せず再利用できる処理。
  コンテナ demux/codec decode と基礎観測、f64 FFT/相関/帯域・
  スペクトル演算、plan/workspace、SIMD dispatch、チャンク駆動
  オフライン実行、波形/スペクトログラム/タイムライン表示
  プリミティブ、GUI ホスト/IPC 基盤。
- AudioLens が所有するもの: ゲーム音声デバッグ固有の意味論。
  複数 LocateMethod の探索/合議、loop/skip/gap/drift の piecewise
  time map、ミキサー推定・residual search、
  Cheap/Reference/Expensive 制御、FeatureRecord・Validity・
  Confidence・Support・Provenance、不変 Job・Manifest・
  Analysis Dataset・可搬 export、CLI/GUI のコマンド・画面意味。
- 判定に迷う処理は「別用途のツールがそのまま使えるか」と
  「AudioLens の解析判断を含むか」で決め、判断は責務マップの
  「境界判定記録」に記録する。

### 精度・時刻・RT 契約

- 解析経路は終端まで f64 を維持し、f64→f32 の暗黙ダウングレード
  を禁止する。Ruviform の既存 RT/f32 API は変更しない。
- 元 PCM の尺度を保持する(整数 PCM は整数値のまま、unsigned
  8bit は -128 オフセット、float32 は f64 拡張、float64 は元値)。
  [-1,1] 正規化を I/O 境界で強制しない。NaN/Inf 等の IEEE 特性は
  保持しゼロへ置換しない。
- raw PTS/DTS・time base・原点・推定値/明示値の区別・欠落を
  I/O 境界から失わない。欠測は欠測のまま保持し、公称 FPS 等から
  時刻を捏造しない。
- AudioLens の解析パイプラインは全体として非 RT。Ruviform の
  RT 契約を緩めない。f64 API の一部が RT-suitable でも、
  AudioLens は非 RT 利用として扱う。

### Consequences

- Good: 汎用基盤の重複保守が解消され、改善が Ruviform 側に集約
  される。AudioLens 側の差分は解析意味論に限定される。
- Good: f64・元 PCM/PTS の契約が API 境界で明文化され、f32
  縮退や正規化による情報損失を契約違反として検出できる。
- Good: GUI と CLI が同一の非 UI 処理層を共有し、旧 `al-viz`
  系の静的生成と並行する二重表示系が最終的に一つになる。
- Bad: 移行期間は旧実装と Ruviform 呼び出しが同居し、暫定
  adapter・比較フラグの管理コストが発生する。撤去条件は
  `../plans/ruviform-foundation-migration.md` で強制する。
- Bad: Ruviform 側の不足・不具合は AudioLens では直せず、
  Ruviform Issue への依存がクリティカルパスに入る。

### Confirmation

- 責務マップの全行が「現オーナー→新オーナー」「移す汎用計算 /
  残す固有処理」「API の意味・precision・RT 契約」「ライブラリ
  依存」「性能・検証方法」「切替順序」を持ち、曖昧な所有が
  残っていないこと。
- 依存 DAG の各ノードが両リポジトリの具体的 Issue とリンクし、
  クリティカルパスと各段階のゲートが一意に読み取れること。
- 下記「既存 ADR との整合」表の各行について、維持/改訂/撤去
  時 supersede の扱いが個別に記述されていること。

## 既存 ADR との整合

本 ADR が改訂・補足する既存決定と、変更しない決定を明示する。
「切替時 supersede」は当該切替 Issue の完了をもって旧 ADR を
`superseded` に更新する運用とし、本 ADR 作成時点では status を
変更しない(現行実装の正本であり続けるため)。

なお、本 ADR が `depends-on`/`refines` で参照する proposed ADR
の記述内容は、本 ADR の acceptance により不変条件として採用
され、対象 ADR が後に改訂・却下されても本 ADR の前提は変わら
ない。

| ADR | 扱い | 変更点 |
| --- | --- | --- |
| `implementation-stack.md` | refines(部分改訂) | 「Rust 採用」は維持。「コア処理を自前モジュールで実装」は、用途非依存のメディア I/O・数値・DSP・表示・実行基盤を Ruviform が所有する形に狭める。AudioLens 固有の解析意味論・Job・Dataset は引き続き自前実装。 |
| `media-decoder-backend.md` | refines(切替時 supersede) | `MediaDecoder` trait/native 優先 registry・fallback 合成は、#100 で Ruviform media-io/backend(#151/#154)呼び出しへ置き換わる。detect/probe/decode の責務分割・明示失敗・provenance 記録の契約は Ruviform 観測契約への要求として継承され、AL 側 registry 自体は撤去時に superseded。 |
| `mp4-demux-decode.md` | refines(切替時 supersede) | 「MP4 はコーデック decode しない」は「AudioLens 自前実装では decode しない」に読み替えられる。codec 対応は Ruviform backend capability(#154)に委譲され、AAC decode を含む既存 capability も同 backend capability へ移管される(能力は維持)。タイミング観測の独立価値・明示失敗ポリシーは維持。「外部依存を増やさない(決定性・再現性・配布容易性)」制約も継承する(Ruviform backend も同制約下)。 |
| `avi.md` | refines(切替時 supersede) | 「映像フレームデコードは非圧縮 AVI 限定」は AL 自前実装のスコープ決定であり、codec/container 対応範囲は Ruviform backend capability に移管。明示失敗・時刻推定禁止は維持。「外部依存を増やさない(決定性・再現性・配布容易性)」制約も継承する(Ruviform backend も同制約下)。 |
| `audio-fallback-pipeline-native-container-facts-symphonia-decode.md` | refines(切替時 supersede) | native container facts + symphonia 音声 fallback の AL 内合成は、Ruviform 側の backend 構成・観測契約へ移管。container facts 正本と部分失敗の区別は #151 の要求として継承。 |
| `simd-arithmetic-pulp.md` | refines(撤去時 supersede) | pulp は `al-math` 独自 SIMD 基盤としての選定。カーネルが Ruviform(fearless-simd 系、`simd-ulp`/`dsp-simd-backend-adoption`)へ移ると AL の pulp 使用は消滅し、#104 の撤去時に superseded。移行期間中は現行契約として有効。 |
| `derived-visualization-dataset.md` | refines(切替時 supersede) | 「可視化は解析意味を持たない」は維持。「Analysis Dataset から派生した可視化データセットを別成果物として生成する」は、GUI が永続 Analysis Dataset を range query し LOD/表示 cache を実行時に作る方式(#105)に置き換わる。Visualization Dataset の永続生成は #106 で廃止し、切替時点で記録された方向性を supersede する(対象文書が proposed のため廃止時に superseded 扱いとする)。 |
| `measurement-over-verdict.md`、`immutable-resumable-job.md`、`analysis-dataset-storage.md`、`artifact-*.md`、`stage-timing-manifest-persistence.md`、`spec-residual-contract-finalization.md`、`residual-issue-boundaries.md`、`multi-method-locate-consensus.md`、`locate-argmax-ambiguity-representation.md` | 維持(depends-on/related) | 観測主義・不変 Job・Dataset スキーマ・耐久性・合議/曖昧性・候補成果物の固有責務は AudioLens が所有し続ける。 |
| `observability-dedicated-crate.md`、`logging-sink-format-activation-contract.md` | 維持(related) | アプリレベルの logging init/format/activation 契約は AL に残る。`ruviform-observability` への委譲は本 ADR では決めず、#104 監査で別判定。 |
| `al-math-api-*.md`、`lag-al-math-plan.md`、`ncc-*.md`、`caller-side-optimizations-structure.md`、`bandfilter-inputcache.md`、`spectral-envelope-fft-size-assert.md`、`simd-reduction-lane-fma.md`、`locate-plan-run-pool.md` | 維持(related) | `al-math`/`al-locate` の API・plan/workspace・数値規約の決定群。対応カーネルの Ruviform 移行時に、Ruviform 側 API 契約へ引き継ぐ要件として責務マップから参照する。 |

### Ruviform 側 ADR との整合

調査済みの Ruviform 側 accepted ADR との関係:

- `realtime-contract-and-verification-policy.md`(RT 契約): AL の
  f64 オフライン利用は RT 契約を緩めない。矛盾なし。
- `media-io-backend-abstraction.md`(I/O 抽象境界): #151/#154 の
  観測情報拡張はこの抽象境界の内側で行う。矛盾なし。
- `offline-determinism-scope.md`(逐次参照等価): `ruviform-exec`
  の並列実行を stage 内に使う場合、チャンク独立の呼び出し側契約を
  AL 側 stage が満たす必要がある(#54 で評価)。矛盾なし。
- `dsp-simd-backend-adoption.md`・`simd-ulp.md`(fearless-simd):
  AL の `simd-arithmetic-pulp.md` と採用基盤が異なる。カーネル
  移行時は Ruviform 側の SIMD 契約に従う。ADL 側の pulp 撤去は
  #104。矛盾なし(共存期間の重複は許容、撤去条件で解消)。
- `ui-architecture-webview-hybrid.md`・`ui-tech-selection-poc.md`
  (WebView hybrid): #105 はこの選定を再審査せず利用する。
  矛盾なし。

今後、責務マップや切替実装中に Ruviform 側の設計ベースライン・
RT/モジュール分割 ADR と矛盾を発見した場合は、AudioLens 内だけで
例外化せず、AGENTS.md の規約に従い Ruviform 側に Issue を起票し
相互参照する。

## Implementation Plan

- Affected paths: `docs/adr/`(本 ADR と索引)、
  `../designs/ruviform-migration-responsibility-map.md`(責務
  マップ正本)、`../plans/ruviform-foundation-migration.md`
  (段階 DAG・rollback・撤去条件)、
  `../specs/ruviform-foundation-migration.md`(要求仕様)。
  コード変更は本 ADR のスコープ外で、#98〜#106 /
  Ruviform #150〜#157 が実装を担当する。
- Patterns to follow: 処理単位の所有判定は責務マップの行に従う。
  切替は #98 の baseline と同一 fixture/形状での Golden・差分・
  性能比較をゲートにする。暫定 adapter/比較フラグには目的・削除
  条件・関連 Issue を付記する。
- Patterns to avoid: Ruviform 側の責務を AL 側の恒久 adapter で
  覆うこと。f64→f32 の暗黙変換、元 PCM/PTS の正規化喪失、
  `unavailable` と 0/デフォルトの混同、AudioLens 固有型の
  Ruviform 公開 API への混入。
- Dependencies or configuration: Ruviform crate は検証済み同一
  Git SHA の git 依存(#99 で固定)。本 ADR 自体は新規依存を
  追加しない。

## Verification

- [ ] 本 ADR・責務マップ・段階 DAG がレビュー済み
      (review-protocol で CONFIRMED blocker なし)。
- [ ] 責務マップの各行が 6 項目(現→新オーナー/移す・残す/
      API 意味・precision・RT 契約/ライブラリ依存/性能・検証方法/
      切替順序)を満たし、棚卸し対象の全 crate・全境界を網羅する。
- [ ] f64・元 PCM/PTS・RT/非 RT・汎用/固有境界について、責務
      マップと本 ADR の記述が一致している。
- [ ] 依存 DAG の全ノードが Issue 番号に対応し、クリティカル
      パスと撤去条件が追跡可能である。
- [ ] 上記「既存 ADR との整合」表が関連 ADR 全件をカバーし、
      `relations.refines`/`related`/`depends-on` と矛盾しない。
- [ ] `mise run lint` が pass する(文書のみの変更で実行可能な
      検証が無い場合はその旨を記録)。

## Pros and Cons of the Options

### A: 自前モジュール継続

- Good: 移行コスト・リスクがなく、現行の f64 精度と観測契約を
  そのまま維持できる。
- Bad: 汎用基盤の重複保守が続き、Ruviform 側の改善(最適化・
  backend 拡充・GUI 基盤)を享受できない。#95 で棄却済み。

### B: 処理単位の責務分割(採用)

- Good: 汎用処理の再利用と固有意味論の保護を両立し、依存方向と
  精度契約を API 境界で強制できる。
- Bad: 移行期間の二重実装・比較ゲート・adapter 管理が必要で、
  Ruviform 側の進捗にクリティカルパスが依存する。

### C: パイプライン全体を Ruviform へ移す

- Good: AudioLens の実装量は最小になる。
- Bad: Ruviform に AudioLens 固有の Job・合議・Validity・画面
  構成を持ち込むことになり、依存方向と再利用性を破壊する。
  #95 の採用方針 3・4 に反する。

### D: 所有権を決めない併用運用

- Good: 局所的には最も早く部分利用を始められる。
- Bad: 二重実装が恒常化し、どちらが正本か判断不能な分岐が
  残る。「段階移行と実測比較後の撤去」という #95 の前提に反する。

## More Information

- 追跡 Issue: AudioLens
  [#95](https://github.com/scarecrow173/audio-lens/issues/95)・
  本設計 Issue
  [#97](https://github.com/scarecrow173/audio-lens/issues/97)。
- 子 Issue: AudioLens #98〜#106。継続利用 Issue #54/#60/#71/#76。
- Ruviform Issue: #150(f64 基盤)・#151(観測契約)・#152(相関)・
  #153(帯域/STFT)・#154(codec backend)・#155(CLI/headless)・
  #156(GUI host)・#157(表示 primitive)。設計ベースラインは
  Ruviform [#1](https://github.com/scarecrow173/ruviform/issues/1)。
- 責務マップ正本: `../designs/ruviform-migration-responsibility-map.md`。
- 段階 DAG・rollback・撤去条件正本:
  `../plans/ruviform-foundation-migration.md`。
