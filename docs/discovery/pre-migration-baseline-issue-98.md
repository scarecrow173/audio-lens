---
id: "DISC-034P3FNUUdDoTj34qYS35D"
type: "discovery"
status: "active"
title: "pre-migration baseline: Issue #98 の現行固定対象と環境調査"
created: "2026-10-30"
updated: "2026-10-31"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/98"
    - "https://github.com/scarecrow173/audio-lens/issues/95"
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
    - "../specs/validation-strategy.md"
    - "validation-fixture-gaps.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/pre-migration-baseline-golden.md"
    - "../adr/pre-migration-baseline.md"
  verifies: []
  verified-by: []
  references:
    - "../../samples/README.md"
    - "../../crates/al-cli/src/main.rs"
    - "../../crates/al-job/src/manifest.rs"
  defers: []
  deferred-by: []
---

# pre-migration baseline: Issue #98 の現行固定対象と環境調査

## Summary

Issue #98 は Ruviform 移行前の比較基準として、現行実装の
golden・実メディア挙動・性能を固定することを求める。既存の
golden テスト行列・bench・manifest 計時は相当部分を既に
カバーしており、本件の新規部分は (a) 再現情報(commit/環境/
入力 hash/設定/手順)の固定、(b) bit-exact vs 許容誤差の
契約区分、(c) 性能 baseline の計測・保存、(d) 欠けた
guard ケースの最小増補に絞られる(confirmed)。

## Exploration Goal

Issue #98 の完了条件を、既存資産で充足できる部分と新規に
作る部分に分解し、baseline の保存形式・計測方法・比較契約を
確定させる。後続の移行 Issue(#100/#101/#103)が
before/after を客観比較できる状態を作ることが目的。

## Key Issues and Assumptions

- golden 比較は「run 毎に変わる値」と「変わらない値」を
  区別しないと成立しない。`JobId`/`ArtifactId`/`InputId`
  (既定)/`AlignmentId` は実行毎に乱数採番される
  (`crates/al-core/src/ids.rs`、時刻+PID シードの
  xorshift)。したがって生 manifest/結果 JSON の byte 比較は
  再現不能で、ID・時刻・絶対パスを正規化した比較面を定義する
  必要がある(confirmed)。
- `--input-ids` で InputId を固定すれば `analysis/` 配下の
  ディレクトリ名(`IN-*`)は run 間で安定する
  (`crates/al-cli/src/main.rs` の USAGE、実測でも確認)。
- 実メディア 1 job(50 s 本編 + dropout + bgm 素材、全 stage)
  の出力は約 5.5 GB、wall 約 358 s(locate 約 267 s が支配的)
  — golden に生成果物を全コピーするのは非現実的で、
  「小さな JSON/構造 golden + 全成果物の path/size/sha256
  索引」を保存する形が妥当(confirmed、実測)。
- `al-viz`/Visualization Dataset/HTML export は #105/#106 で
  廃止予定。維持 golden の対象外だが、移行前比較材料として
  生成物の存在・サイズ・hash 索引のみ記録する。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| baseline を `baseline/`(repo 直下)に JSON で commit | 機械処理しやすい、docs の artifact graph と独立、移行後も比較 script が参照できる | 新しい top-level dir が増える | yes |
| baseline を `docs/baseline/` に置く | 文書と同居 | canonical docs 以外の .md/大きな JSON が artifact graph 監査に混入し得る | no |
| golden を `target/bench-baseline/` 等 untracked に置く | 既存 bench 慣習と一致 | clone で失われ「固定」にならない | no |
| 計測を in-process(bench example)に限定 | OS 非依存 | stage 別/e2e/IO が取れない | maybe |
| 計測を外部ポーリング(OS プロセス情報)で取得 | e2e CPU/RSS/IO が取れる | OS 依存コード、ポーリング精度に限界 | yes |

## Tentative Conclusions and Hypotheses

- (confirmed) 既存 golden テストは issue 対象の大半を既に
  網羅: al-locate/tests/golden.rs(厳密挿入・gain/EQ・無音
  境界・周期・loop・複数出現・長尺 offset/drift・skip+再開・
  多 ch downmix・BGM 残差・3 失敗モード)、
  al-analysis/tests/golden.rs(scalar gain・band EQ・位相・
  遅延・jitter・SRC・極性・ch swap・ノイズ・clip・comb・
  echo・alias・drop/repeat/stuck・PSD lift・低コヒーレンス)、
  al-media/tests/golden.rs(CFR/VFR/AV offset/drift/dup/drop/
  ctts reorder/PCM roundtrip/AAC-only ingest)。
- (confirmed) `Manifest` は input の `source_sha256`・stage
  別 `duration_ms`・artifact lineage を既に記録する
  (`crates/al-job/src/manifest.rs`)。run manifest の保存は
  既存 manifest のコピー+正規化で足りる。
- (confirmed) bench 基盤は criterion(al-math/al-analysis,
  pin `=0.8.2`)と `examples/bench_*.rs`(target/bench-baseline
  出力慣習)が存在し、再利用できる。
- (hypothesis→確認済) メモリ/alloc・IO・busy-time の既存
  計測経路はない。al-observability は logging のみ。外部
  ポーリング + 成果物サイズ集計で代替する。
- (hypothesis) al-math の NaN/Inf・無音・ゼロ長・短長入力の
  明示 test は部分的(empty 系はある)で、guard 増補余地が
  ある。実装側で何を増補するかは test-spec で確定する。

## Open Questions

- なし(issue 本文・AGENTS.md・制約で判断可能な範囲に収まる)

## Promotion Candidates

- [x] spec-doc needed: baseline の構成・golden/性能の
  受け入れ条件・対象外範囲 → `docs/specs/pre-migration-baseline.md`
- [x] adr-doc needed: baseline の保存形式・正規化契約・
  bit-exact/tolerance 区分・計測方法 → `docs/adr/pre-migration-baseline.md`

## Sources and Relations

- Issue #98(本体): 対象列挙・作業項目・完了条件
- Issue #95(親): 採用済み方針 5「現行のアルゴリズム・実
  サンプル・Golden・性能測定を比較基準として保全」、
  GUI/al-viz 廃止方針
- `docs/specs/validation-strategy.md`: golden fixture 全体方針
- `samples/README.md`: 実メディア fixture の構成と
  ffmpeg AVI 派生手順(本 baseline の e2e 入力経路)
- `crates/al-cli/src/main.rs`: run/resume/inspect、
  `--input-ids`、`--config`、stage 一覧
- `crates/al-job/src/manifest.rs`: manifest スキーマ
  (inputs/config/stages/artifacts、duration_ms)

## Evidence

### 2026-10-31 環境・実測

- worktree: `al98-baseline`, branch `scarecrow173/geoduck`,
  base commit `f6a8037`(Merge PR #108)。mise toolchain:
  rust 1.99.0 / node 26.12.1 / biome 2.6.2 / prek 0.5.7 /
  git-lfs 3.8.0(いずれも mise.toml pin)。
- `cargo build --workspace --release`: 成功(1m03s)。
- ffmpeg: グローバル mise の 9.0.2 を再 install 後に利用可
  (初回 install は zlib.dll 解決不良で silent exit する
  壊れた状態だった。`mise install ffmpeg@9.0.2 --force` で
  復旧)。H.264+AAC の samples MP4 は al-media で直接
  デコードできないため(ADR: demux+timing 観測のみ)、
  `samples/README.md` 記載の ffmpeg→rawvideo AVI 派生経路を
  baseline e2e 入力に使う。
- e2e 試行(全 stage、3 入力): `JOB-OPNZOQvl2fj706ZclHk8K8`。
  stage duration_ms: ingest 1196 / locate 266870 /
  cheap_analysis 4778 / reference_analysis 56409 /
  expensive_analysis 5672 / dataset 220 / visualization 3784 /
  export 23413(合計約 358 s)。成果物計 約 5.5 GB
  (analysis 2.6G・export 2.6G・inputs 117M・locate 184M;
  analysis 内訳: intermediate 1.3G・residual 734M・
  visualization 525M)。
- ID 採番: `crates/al-core/src/ids.rs` —
  `JOB-`/`ART-`/`IN-`/`ALN-` + 22 char base62、
  SystemTime+PID シード。run 間で非決定的。
- golden 既存 coverage(test 名)は上記「Tentative
  Conclusions」に集約。multitrack・欠落/不連続の素材は
  AVI dup/drop・MP4 stts/ctts・多 track MP4 writer で
  部分的にfixture 化済み(al-testkit)。
- `al inspect` は manifest を JSON 表示するのみで集計機能
  なし(今回は計測 script 側で manifest から stage/成果物
  サイズを集計する)。
