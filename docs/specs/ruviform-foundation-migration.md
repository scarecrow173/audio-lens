---
id: "SPEC-034P3nkrA1BoZbYjM3OrQ3"
type: "spec"
status: "approved"
title: "Ruviform 基盤移行の要求仕様"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/implementation-stack.md"
    - "../adr/media-decoder-backend.md"
    - "../adr/mp4-demux-decode.md"
    - "../adr/avi.md"
    - "../adr/audio-fallback-pipeline-native-container-facts-symphonia-decode.md"
    - "../adr/simd-arithmetic-pulp.md"
    - "../adr/derived-visualization-dataset.md"
    - "../adr/measurement-over-verdict.md"
    - "../adr/immutable-resumable-job.md"
    - "../adr/analysis-dataset-storage.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Ruviform 基盤移行の要求仕様

## What Should Be Built

AudioLens の共通マルチメディア基盤を Ruviform
(`scarecrow173/ruviform`) に移すための設計成果物を確定する。
具体的には次の 3 点を canonical 文書として作成する。

1. 移行方針を記録する正式 ADR(`docs/adr/`)。現行
   `implementation-stack.md` の「コア処理を自前モジュールで実装」
   方針との関係・supersede / 改訂箇所を明示する。
2. モジュール / 処理ごとの責務マップ。「現オーナー→新オーナー」
   「移す汎用計算 / 残す固有処理」「API の意味・precision・RT 契約」
   「ライブラリ依存」「性能・検証方法」「切替順序」を処理単位で
   定義する。
3. 移行の依存 DAG。各段階の実施順序、rollback / 比較手順、
   二重実装撤去条件を整理する。

これは追跡 Issue [#95](https://github.com/scarecrow173/audio-lens/issues/95)
の子 Issue [#97](https://github.com/scarecrow173/audio-lens/issues/97)
の要求を canonical 文書へ反映するものである。

## Why Now

- AudioLens はメディア I/O・DSP・数値演算を自前モジュールで実装して
  きたが、同じ基盤を再利用可能な形で所有する別リポジトリ
  (Ruviform) が存在し、処理単位で共通化する方針が #95 で採用済み。
- 子 Issue (#98〜#106、Ruviform #150〜#157) が起票済みで、各 Issue が
  依存する「どの処理を誰が所有するか」の正本がないと実装を始められ
  ない。
- GUI 化(#105)と旧可視化廃止(#106)の方針も確定しており、
  GUI/CLI/headless サービス/表示 cache/永続 Dataset/export の境界を
  同じ責務マップで確定する必要がある。

## Users and Value

- 移行を実装するエージェント / 開発者: 処理単位の所有境界・精度・
  RT 契約・切替順序を判断材料なしに参照できる。
- レビュアー: 汎用/固有の境界・依存方向・段階ゲートに対する変更の
  合否を客観基準で判定できる。
- 両リポジトリの Issue 追跡: 実装順序と完了条件が相互リンクで
  追跡可能になる。

## Scope

### In Scope

- 決定済み前提(Issue #97「決定済みの前提」5 項目)を ADR として記録:
  f64 高精度オフライン API の Ruviform 追加、Media I/O の Ruviform
  経由、処理単位の汎用化基準、旧 `al-viz` / Visualization Dataset /
  静的 HTML レポートの最終廃止と GUI 化、CLI/headless との両立。
- 棚卸し対象の責務マップ: `al-math`、`al-media`、`al-locate`、
  `al-analysis`、`al-job` vs `ruviform-exec`、`al-core`、`al-dataset`、
  `al-viz`(最終廃止・可搬 export 分離)、`al-observability`、
  `al-cli`(継続)、新 GUI アプリ。
- GUI/CLI 入口・内部 headless サービス、解析データ query/data port、
  UI 表示 cache と永続 Analysis Dataset、可搬な解析データ export の
  境界。
- 各段階の依存 DAG、rollback/比較手順、二重実装撤去条件、実施順序と
  完了条件を両リポジトリの Issue と相互参照で追跡可能にすること。
- 既存 accepted ADR との整合・変更点の明示(supersede / refines /
  related の関係付けを含む)。

### Out of Scope

- 解析アルゴリズム・コードの実装変更(本 spec の成果物は設計文書
  のみ。実装は子 Issue #98〜#106 / Ruviform #150〜#157 が担当)。
- Issue #97「決定済みの前提」5 項目の再投票。
- Ruviform リポジトリ側のファイル変更(矛盾・改善案は Ruviform 側
  Issue として起票し、AudioLens 内だけで例外化しない)。
- CLI 出力と永続スキーマの互換層(Pre-release 規約上要求しない。
  Golden 比較は #98 / 各切替 Issue が担当)。
- Golden・性能ベースラインの実測そのもの(#98 が担当。本 spec は
  比較手順の定義のみ)。
- 保存スキーマ変更(Parquet/Zarr 化等)・実行基盤の刷新(#103 の
  追加の位置づけ参照)。

## Requirements

- R1: ADR は MADR 形式で `docs/adr/` に配置し、採用済みの移行方針を
  `accepted` として記録する。`implementation-stack.md` を含む既存
  ADR との supersede / 改訂 / 維持箇所を明示する。
- R2: 責務マップは crate 名ではなく処理単位を正とし、各行に
  「現オーナー→新オーナー」「移す汎用計算 / 残す固有処理」
  「API の意味・precision(f64/f32・元 PCM 尺度)・RT/非 RT 契約」
  「ライブラリ依存」「性能・検証方法」「切替順序」を持つ。
- R3: f64 解析精度・元 PCM/PTS の非破壊取得、RT/非 RT 境界、
  汎用処理と AudioLens 固有処理の境界が曖昧なまま残らない。
- R4: Ruviform → AudioLens の依存方向を維持し、Ruviform に
  AudioLens 固有の Job・Locate・Validity・画面構成を持ち込まない。
- R5: 依存 DAG は両リポジトリの具体的 Issue(AL #98〜#106、
  Ruviform #150〜#157、および継続利用 Issue #54/#60/#71/#76)と
  リンクし、実装順序・各段階のゲート・移行完了条件が追跡可能
  である。
- R6: GUI と CLI が同一の UI 非依存 headless サービス層に依存し、
  CLI が WebView/GPU/ウィンドウに依存しない境界を責務表に含める。
- R7: 旧 `al-viz` / Visualization Stage / 静的 report の撤去と可搬
  解析データ export の分離を責務表・DAG に含め、撤去条件は #106 と
  連携する。
- R8: Ruviform 側の既存 ADR(RT 契約・media-io 抽象境界・SIMD・
  オフライン決定性等)と矛盾する点を発見した場合は AudioLens 側で
  例外化せず、Ruviform 側 Issue への起票を手順として明記する。

## Acceptance Criteria

- [ ] `docs/adr/` に移行方針の正式 ADR が存在し、status が
      `accepted` で、`implementation-stack.md` との関係と改訂箇所が
      本文・relations の双方で明示されている。
- [ ] 責務マップに `al-math` / `al-media` / `al-locate` /
      `al-analysis` / `al-job` vs `ruviform-exec` / `al-core` /
      `al-dataset` / `al-viz` / `al-observability` / `al-cli` /
      新 GUI アプリの全行があり、各行が R2 の 6 項目を満たす。
- [ ] GUI/CLI 入口・内部 headless サービス・解析データ
      query/data port・UI 表示 cache・永続 Analysis Dataset・
      可搬 export の各行が責務表に存在する。
- [ ] 依存 DAG が段階ゲート・rollback/比較手順・二重実装撤去条件を
      含み、各ノードが具体的 Issue 番号に対応する。
- [ ] f64・元 PCM/PTS・RT/非 RT・汎用/固有境界について、責務マップ
      と ADR の記述が一致し矛盾がない。
- [ ] review-protocol によるレビューで CONFIRMED blocker が残って
      いない。

## Deferred Design Concerns

- `al-gui` アプリの具体的構成・IPC adapter・配布形態 | GUI 画面
  設計は #105 の詳細設計が担う | #105 着手時 | 本 spec で確定しない
  ことによる責務漏れリスクは責務マップで吸収する。
- `ruviform-exec` の OfflineDriver を stage 内並列化に適用するか |
  性能課題であり移行の前提ではない | AL #54 の実測評価 | 早期採用は
  検証基盤不足でリスクが高い。
- `al-observability` の `ruviform-observability` への委譲 | アプリ
  レベルの logging 契約は現行 ADR で確定済みで移行目的ではない |
  #104 cleanup 監査 | 未評価のまま委譲すると sink/format 契約が
  揺れる。

## Sources and Relations

- 一次出典: Issue [#95](https://github.com/scarecrow173/audio-lens/issues/95)
  (採用済み移行方針・子 Issue 一覧・概略 DAG)、Issue
  [#97](https://github.com/scarecrow173/audio-lens/issues/97)
  (棚卸し対象・成果物・完了条件)。
- 関連 ADR: `../adr/implementation-stack.md`(自前モジュール方針、
  改訂対象)、`../adr/media-decoder-backend.md`・
  `../adr/mp4-demux-decode.md`・`../adr/avi.md`・
  `../adr/audio-fallback-pipeline-native-container-facts-symphonia-decode.md`
  (メディア I/O 関連)、`../adr/simd-arithmetic-pulp.md`(SIMD 基盤)、
  `../adr/derived-visualization-dataset.md`(派生可視化方針、
  改訂対象)、`../adr/measurement-over-verdict.md`・
  `../adr/immutable-resumable-job.md`・
  `../adr/analysis-dataset-storage.md`(維持する固有責務)。
- 対応する Ruviform 側 Issue: #150〜#157(前記 source 参照)。
