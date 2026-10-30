---
id: "DESIGN-034P4pSHzTNg4S497UokuT"
type: "design"
status: "approved"
title: "pre-migration baseline の実装設計"
created: "2026-10-30"
updated: "2026-10-31"
owners:
  - "scarecrow173"
relations:
  source:
    - "../adr/pre-migration-baseline.md"
    - "../specs/pre-migration-baseline-golden.md"
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
    - "../discovery/pre-migration-baseline-issue-98.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/pre-migration-baseline.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# pre-migration baseline の実装設計

## Context

`docs/adr/pre-migration-baseline.md` が定めた保存構成・正規化・
比較契約を実装する。実装は 2 系統:

1. `tools/baseline/` — Node.js(mise pin 26.12.1、外部依存
   なし、stdlib のみ)による capture / normalize / compare
   script。product code には計測コードを持ち込まない。
2. `baseline/` — capture が生成し commit する成果物群。

計測・比較の駆動源は既存 `al` CLI(release build)と job
manifest。`al` は base-dir 配下に全成果物を置くため、
baseline の記録単位は「1 run = 1 job ディレクトリ」。

## Scope

### In Scope

- `tools/baseline/capture.mjs`: run matrix の実行、環境・
  入力 hash・成果物索引・golden・性能値の収集
- `tools/baseline/lib/normalize.mjs`: ADR「正規化契約」の
  実装(ID 正規名 mapping、絶対パス placeholder、record-only
  除外、path セパレータ正規化)
- `tools/baseline/lib/store.mjs`: al-dataset の columnar
  table(`.bin`)と chunked tensor(`array.json` +
  `chunks/*.bin`)の読み取り。schema に従い I64/F64/Utf8/
  Null cell と f64 tensor を JS 値へ decode する
- `tools/baseline/compare.mjs`: `contracts.json` に従う
  差分判定 + 摂動自己検証モード
- alloc 計測用の counting-allocator example(既存
  `examples/bench_*` 規約に従い `target/bench-baseline/`
  相当の結果を `baseline/bench/` に転記)
- `mise.toml` の `baseline` task(capture の入口)
- `baseline/` 配下の成果物一式と README・coverage・
  contracts の作成
- coverage 台帳で「追加 test」と判定した最小 guard テスト

### Out of Scope

- 比較対象の自動修正・再生成 automation
- product code への計測機構の組み込み(tracing counter 等)
- GUI/al-viz 内容の golden 化
- CI 組み込み

## Components and Boundaries

### `tools/baseline/capture.mjs`(オーケストレーション)

責務: 「実行」「収集」「記録」の 3 相を run ごとに駆動する。

- **実行相**: 入力 AVI が未生成なら ffmpeg で派生
  (`samples/_runs/decoded/`、gitignore 済み領域)。
  `target/release/al run --base-dir samples/_runs/<run名>
  --input ... --input-ids ... [--stages ...]` を spawn し、
  子プロセスを ~1s 間隔でポーリングし CPU(user+kernel)
  と RSS 標本列を収集する。e2e wall は外部
  `performance.now()`。
- **resume run**: `al resume` は manifest の plan と異なる
  stage 集合を拒否し `--stages` も持たないため、
  「全 stage plan の `al run`」を spawn → `manifest.json`
  をポーリングし `locate` が `Completed` になった stage
  境界で子プロセスを kill → `al resume --job-id <JOB>`
  で残りを完走する(Pending stage の半書き成果物が
  残る race は stage 完了直後の kill で回避する)。
- **収集相**: 完了後に job dir を walk し、
  `artifacts.index.json`・golden 抽出・`performance.json`・
  `manifest.json` コピー・`manifest.normalized.json`・
  `inspect.json` を生成する。
- **記録相**: `baseline/environment.json`・`inputs.json`・
  `config.json`・`bench/results.json`・
  `runs/<name>/determinism.json` を書き出す。

境界: capture は「何を実行するか」だけを知り、正規化・
比較の意味論は lib/compare に委譲する。

### `tools/baseline/lib/normalize.mjs`(正規化)

責務: run ディレクトリ全体に対し 1 回の mapping pass を
構成し、run グローバルな canonical 名を発行する。

- pass 1(収集): まず `manifest.json` を読み、
  `artifacts`/`stages` 配列の登録順(pipeline 決定順で
  run 不変)に現れる raw ID を第一基準の出現順として
  採番する。続けて残りの raw ID を「raw path 昇順の
  ファイル列 × ファイル内 DFS 出現順」で拾う(file
  path 文字列自体も走査対象)。正規表現は
  `(?<![0-9A-Za-z])(JOB|ART|IN|ALN)-[0-9A-Za-z]{22}(?![0-9A-Za-z])`
  — `\b` は `_` 挟み ID(`residual/IN-…_ART-….wav`)を
  取りこぼすため使わない。content-derived で決定的な
  `EVT-*` は正規化対象外(埋め込まれた `IN-…` は `-`
  境界で正規化される)。
- pass 2(写像): raw ID → `PREFIX-#n`(n = pass 1 の採番
  順)の mapping を構成。文字列置換は長さ降順に適用し
  前方一致の衝突を避ける。`--input-ids` には
  `IN-` + 22 base62 の literal を指定する(`IN-#n` 形は
  CLI が拒否する)。
- 絶対パス: capture が記録した worktree root・base-dir・
  入力ファイル絶対 path を `<WT>`/`<BASE>`/`<IN:role>`
  に置換する。判定は field 名ではなく「記録した abs
  prefix を含む文字列値」で行う。
- セパレータ: `\` → `/`。
- record-only field(`duration_ms`、実行日時、`error` 文)は
  **値を残したまま**、`contracts.json` の
  `recordOnlyFields` path 指定で comparator が比較を
  skip する(置換・削除はしない — 観測記録として残す)。
- path 置換も同じ mapping を使うため、ファイル名中の
  `ART-*`/`IN-*` も canonical 名になる。

### `tools/baseline/lib/store.mjs`(payload 読み取り)

責務: 公開レイアウトのみを使って `.bin` を decode する。

- table: `schema.json`(列名 + `data_type`、serde は
  snake_case の `"i64"/"f64"/"utf8"`)と `index.json`
  (行数・chunk 行数)を読み、`<col>/<NNNNNN>.bin` を
  NULL bitmap + 値列として decode する。`i64` は BigInt
  で decode し 2^53 超えを検出する。行デコード結果は
  golden の `*.rows.json` に使い、compare 時の値比較に
  も使う。
- tensor: `array.json`(dtype/shape/chunk_shape/order)を
  読み、`chunks/<coords>.bin` を f64le row-major で
  decode する。edge chunk の実 extent を shape から算出。
- wav: RIFF ヘッダを parse し PCM サンプルを decode
  する。PCM int だけでなく IEEE float(f32/f64、
  `WavEncoding::Float64`/`Float32`)も扱う(bit-exact 面
  は sha256 比較、decode は差分表示用)。

### `tools/baseline/compare.mjs`(比較)

責務: `contracts.json` の意味論を 2 run 間に適用する。

- golden JSON 面: 構造一致。leaf が数値なら bit-exact/
  tolerance を `contracts.json` の path-glob 規則で選択。
  整数(Number.isSafeInteger 範囲確認つき)は厳密一致。
  `null` は「非有限または欠損」で null 同士は一致、
  非 null と null の不一致は差分。
- index 面: path 集合一致 + class 別規則(bit-exact は
  sha256、tolerance は sha256 差なら payload decode 比較、
  record-only は path のみ)。
- 差分は `{ surface, path, kind, expected, actual }` の列挙と
  して出力し、exit code で有無を返す。
- `--self-test`: baseline の一時コピーに摂動を加え、
  意図した差分が報告されることを確認する。

### alloc 計測 example

`crates/al-analysis/examples/bench_alloc.rs`(仮): 既存
`bench_*` example と同じく Instant 計測に加え、
`#[global_allocator]` に counting allocator を挟み、代表
解析(例: NCC パイプライン or reference analysis の小
fixture)の alloc 回数・bytes を JSON で出力する。
実行は `cargo run --release -p al-analysis --example
bench_alloc`(workspace のため `-p` 必須)。
product crate への変更は example 追加のみ。

## Data and Control Flow

1. `mise run baseline` → `capture.mjs` が `baseline/config.json`
   の run matrix を読み、各 run を実行。
2. 各 run 完了時に収集相が `baseline/runs/<name>/` を生成。
3. `aa` run 完了後、normalize + compare で
   `runs/aa/determinism.json` を生成(`canonical` との比較)。
4. `resume` run 完了後、`canonical` との normalized golden
   一致を `compare` で検証し結果を記録。
5. bench: `cargo run --release --example bench_*` の出力を
   `baseline/bench/results.json` に転記(criterion は既存の
   `target/criterion` サマリを転記)。
6. compare は移行後に「再計測 run vs baseline commit 済み
   run」でも同じコードで動く。

## Non-Functional Constraints

- 実行時間: run matrix 全体で実測 40 分程度(canonical ~6 分
  + variants 5 本 + small + resume + aa)。計測は release
  build 前提。
- 容量: job dir は合計 ~20 GB(生成物); baseline commit
  分は正規化 JSON + 索引で数十 MB 未満に抑える。
- 外部依存なし: script は Node stdlib のみ。ffmpeg は入力
  派生のみに使い、比較契約には含めない。

## Risks and Trade-offs

- run 全体の収集は IO 重い(5.5GB/run の walk + hash)。
  sha256 は stream で計算し、index 生成は run 完了直後の
  1 pass で済ませる。
- normalize の「run グローバル mapping」は run 内で一貫
  だが run 間で構造差があると正規名がずれる。これは差分
  として観測される設計であり、比較時は「差分あり」で
  報告すればよい(隠れない)。
- store.mjs は al-dataset の store レイアウトに依存する。
  レイアウト変更時は store.mjs も更新が必要(変更は同
  リポジトリ内で追跡可能)。
- `test.avi` の small run で `capture_bug`/`capture_reference`
  に同一ファイルを使うのは実運用形と異なるが、多入力・
  全 stage を短時間で通す smoke として位置づける。

## Deferred Design Concerns

- baseline の CI 定期再計測 | CI 未整備 | CI 整備時 |
  契約ドリフトを検知できない
- store.mjs の al-dataset レイアウト追従 | レイアウトが
  安定するまでは手動追従 | レイアウト変更時 | golden
  decode 失敗

## Verification Notes

- [ ] `mise run baseline` が run matrix 全 run を実行し
  `baseline/` 成果物を生成する
- [ ] `compare.mjs` で `canonical` vs `aa` の差分が
  bit-exact 面で空(または determinism.json に非決定面
  として記録)になる
- [ ] `compare.mjs` で `resume` vs `canonical` の
  normalized golden が一致する
- [ ] `--self-test` で意図差分が正しく報告される
- [ ] `mise run test`/`lint`/`fmt` が green
