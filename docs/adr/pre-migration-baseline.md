---
id: "ADR-034P3HuDRtKNYNujsHyJMz"
type: "adr"
status: "proposed"
title: "pre-migration baseline の保存形式・正規化・比較契約"
created: "2026-10-30"
updated: "2026-10-31"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/98"
    - "../discovery/pre-migration-baseline-issue-98.md"
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
    - "../specs/pre-migration-baseline-golden.md"
    - "../specs/validation-strategy.md"
    - "stage-timing-manifest-persistence.md"
    - "measurement-over-verdict.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/pre-migration-baseline-issue-98.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers:
      - "scarecrow173"
    consulted: []
    informed: []
---

# pre-migration baseline の保存形式・正規化・比較契約

## Context and Problem Statement

Issue #95/#98 のため、現行実装の挙動・精度・性能を「移行後に
同じ手順で再計測し、意味が変わったかを機械的に判定できる」
形で固定しなければならない。レビューで確認した実装上の制約:

- job 成果物の JSON は run 毎に変わる値を含む。ID
  (`JOB-`/`ART-`/`IN-`/`ALN-`)は乱数採番で、JSON 値だけでなく
  **ファイル名・文字列内の部分文字列としても埋め込まれる**
  (`locate/normalized/<ART>.f64le`、`residual_<IN>_<ART>.wav`、
  `residual:{ALN}`、`EVT-*-IN-*`、params.files 等)。
- 絶対パスは `source_path_abs` だけでなく `inputs[].path`・
  `artifacts[].params.source_path`・`metadata.json`/
  `video_features.json` の `source_path`・dataset manifest の
  `inputs[].source_path` に現れる(`JobDefinition::create` が
  無条件で絶対化する)。
- 許容誤差と分類する面の多くは `.bin`(chunked f64 tensor /
  columnar table)・`.f64le`・`.wav` のバイナリ成果物であり、
  JSON 比較だけでは比較面が存在しない。tensor `.bin` は
  little-endian f64 row-major + `array.json` メタ、table
  `.bin` は NULL bitmap + 非 null 値で、外部からデコード可能。
- `serde_json` は非有限 f64(NaN/±Inf)を `null` に潰す。
  JSON 面では非有限値の区別がつかない。
- `al-math` は pulp による実行時 SIMD dispatch で、f64 演算
  順序が CPU 機能レベルに依存する。A/A(同一バイナリ再実行)
  では検出できず、異環境比較では bit-exact は成立しない。
- ffmpeg は repo toolchain 外(実測では global mise 9.0.2)。
  samples の MP4 は H.264+AAC で al-media が直接デコード
  できないため、AVI 派生経路が必要。AVI コンテナ bytes は
  ffmpeg build で変わり得るが、デコードされた PCM/時刻の
  意味は不変。

## Decision Drivers

* 比較は移行後の再計測結果に対して行われるため、保存形式は
  「機械可読・正規化可能・差分許容値つき」でなければならない
* golden の量は限定的でなければならない(実測 1 job 約 5.5 GB
  の生成果物を全て commit するのは非現実的)
* 「意味が変わらない」判定は観測値の記録を優先する
  (`docs/adr/measurement-over-verdict.md`) — baseline は
  判定器ではなく観測記録と比較補助である
* 再現可能性: 入力は LFS commit 済み MP4/WAV の hash で固定
  し、派生入力は生成手順・ffmpeg 版数・生成物 hash を記録
  する。派生 AVI のコンテナ bytes 自体は比較契約に含めない
  (比較面はデコード後の観測値)
* 実行は既存 CLI + 計測 script(Node.js、mise 内)のみで
  再現できること。product code に計測を持ち込まない

## Considered Options

* `baseline/`(repo 直下)に正規化済み JSON + 成果物索引 +
  計測 script を commit する
* golden を `target/bench-baseline/` 等の untracked 領域に
  保存し、再生成手順のみ文書化する
* baseline データを `docs/` 配下の canonical document に
  埋め込む
* 生成果物・派生 AVI を丸ごと LFS/外部ストレージに保存する

## Decision Outcome

Chosen option: "`baseline/`(repo 直下)に正規化済み JSON +
成果物索引 + 計測 script を commit する", because 機械可読で
diff 可能、clone で残り、docs の artifact graph を汚染せず、
外部依存なしに再現・比較できるため。

確定した契約:

### 保存構成

`baseline/` 以下に次を置く(すべて非メディア形式):

- `environment.json` — commit SHA・branch・worktree dirty
  flag、rustc/cargo・mise tools・ffmpeg(版数と検出パス)、
  OS/アーチ/CPU モデル・物理コア数・SIMD dispatch の注意書き、
  build profile(release, `debug=true`)、取得日時
- `inputs.json` — baseline で使う入力一式:`role`・repo 相対
  `path`・`size`・`sha256`・由来。由来は LFS 実サンプル
  (MP4/WAV)か「派生入力」(ffmpeg 変換 AVI)かを区別し、
  派生入力は生成コマンド・ffmpeg 版数・生成 sha256 を記録。
  samples/ 全実サンプルの sha256 一覧も併記する。
  **派生 AVI の sha256 は「観測値」であって比較契約ではない**
  (ffmpeg build 差でコンテナ bytes は変わり得る。契約は
  デコード後の観測値に課される)
- `config.json` — `al run --config` に渡した設定と、各 run の
  `--input`/`--input-ids`/`--stages` 指定の完全な記録
  (既定値利用時はその旨を記録)
- `coverage.json` — Issue #98 列挙項目ごとのカバレッジ
  台帳: 項目 → 手段(e2e run 名 / 既存 test 名 / 追加 test /
  実測未実施とその理由)
- `contracts.json` — 下記「数値契約・比較意味論」の機械可読
  定義(既定 tolerance、面ごとの上書き、比較対象外の指定)
- `runs/<name>/` — 後述 run matrix の各実行記録:
  - `manifest.json` — job manifest の生コピー(record-only
    証跡。絶対パスを含む)
  - `manifest.normalized.json` — 正規化済み manifest
    (golden 比較面)
  - `golden/` — 正規化済みの主要観測 JSON(locate 結果、
    cheap/reference/expensive feature record、candidate
    events 表の行デコード、dataset manifest、inputs
    metadata、時刻観測)
  - `artifacts.index.json` — job 配下全成果物の
    `{正規化 path → {size, sha256, class}}` 索引
  - `performance.json` — stage 別 wall、run e2e wall/CPU、
    peak RSS、I/O 推定量、成果物サイズ集計
  - `determinism.json` — A/A 再実行比較の結果
  - `inspect.json` — `al inspect` 出力(record-only)
- `bench/results.json` — 既存 bench(criterion サマリ・
  `examples/bench_*`)の記録値
- `README.md` — 目的・比較契約の人間向け説明・再現手順・
  実測未実施の項目一覧

`tools/baseline/` に計測 script(Node.js)を置き、capture
(再計測)と compare(差分判定)を提供する。

### run matrix(固定実行集合)

- `canonical`: `capture_bug=dropout.avi` +
  `capture_reference=normal.avi` + `source_asset=source-bgm.wav`、
  全 stage。samples/README.md の動作確認構成と同一。
- `variants`(`desync`/`clipping`/`reverb`/`filter`/`clicks`):
  各 `<variant>.avi` を `capture_bug`、normal.avi を
  `capture_reference`、source-bgm.wav を `source_asset`、
  stage は `ingest,locate,cheap_analysis,reference_analysis,
  expensive_analysis,dataset`(visualization/export を省き
  計測コストを抑える)。実メディアの検出観測を固定する。
- `small`: `test.avi` を `capture_bug`/`capture_reference` に
  同一ファイル、複数 `source_asset`(bgm + 2 素材)で
  全 stage。多入力経路を短時間で通す。
- `resume`: `canonical` と同一入力・全 stage plan の
  `al run` を spawn し、`manifest.json` ポーリングで
  `locate` が `Completed` になった時点で子プロセスを
  kill、続けて `al resume` で完走する(`al resume` は
  記録済み plan と異なる stage 集合を拒否し `--stages`
  も持たないため、部分 plan + resume という形は取れ
  ない — kill 時点は stage 境界とし、Pending stage の
  半書き成果物が残る race は stage 完了直後の kill で
  回避する)。normalized golden が fresh run と一致する
  ことを比較対象にする(resume 等価契約)。
- `aa`: `canonical` と同一入力の 2 回目実行(A/A 決定性確認)。
  `canonical` は 1 回目、`aa` が 2 回目。

### 正規化契約(golden 比較面)

正規化は **run ディレクトリ全体に対して 1 回**行い、同一
mapping を全ファイル・全ファイル path に適用する:

- **ID 正規化**: 乱数採番 ID(`JOB`/`ART`/`IN`/`ALN` +
  22 base62)を、文字列値・ファイル path のいずれでも
  **部分文字列レベル**で置換する。正規表現は base62 の
  みを近傍禁止とする
  (`(?<![0-9A-Za-z])(JOB|ART|IN|ALN)-[0-9A-Za-z]{22}(?![0-9A-Za-z])`)
  — `\b` は `_` 挟みの ID(`residual/IN-…_ART-….wav`)を
  取りこぼすため使わない。content-derived で決定的な
  `EVT-*` 等は対象外(その中に埋め込まれた `IN-…` は
  `-` 境界で正規化される)。写像先は run グローバルの
  正規名 `PREFIX-#n` で、採番順は **manifest.json の
  `artifacts`/`stages` 配列の登録順に出現した順を第一
  基準**(pipeline 決定順で run 不変)、残りは raw path
  昇順の DFS 出現順 + 決定的 tie-break とする(「正規化
  path 昇順」は mapping 構築前で循環するため使わない)。
  同一 raw ID は run 内の全出現(値・path・lineage
  参照)で同じ正規名になり、参照関係が保持される。
- **`--input-ids` を固定**する。これにより `IN-*` 由来の
  ディレクトリ名・`EVT-*` 内の input 参照は run 間で安定する。
  固定値は `IN-` + 22 base62 の literal(例:
  `IN-0000000000000000000000` 連番)を `config.json` に
  記録する(`IN-#n` 形は base62 外で CLI が拒否する)
- **絶対パス正規化**: 文字列値に記録環境の絶対パス
  (worktree root・base-dir・各入力ファイルの絶対 path)が
  現れた場合、該当 prefix を `<WT>`/`<BASE>`/`<IN:role>`
  placeholder に置換する。対象 field を列挙せず「記録した
  abs prefix を含む任意の文字列値」で判定し、未捕捉の field
  追加に強い形にする
- **計時・実行日時**: `duration_ms` 系 field・実行日時・
  `error` メッセージ文字列は比較対象から除外する。
  値は normalized golden にも残し、`contracts.json` の
  `recordOnlyFields` 指定で comparator が比較を skip
  する(観測記録としての価値を残す)
- **path セパレータ**: `\` → `/` に正規化
- 比較は JSON を parse した構造比較であり、JSON ファイルの
  byte/size 一致は要求しない(下記 index 契約を参照)

### artifacts.index.json の比較意味論

索引 entry は `{size, sha256, class}`。`class` で比較を分ける:

- `class: "bit-exact"`: path 集合・size・sha256 が一致必須
  (decoded_pcm `.f64le`、normalized_pcm、residual `.wav` 等の
  PCM ペイロード、schema/index JSON の正規化後構造)
- `class: "tolerance"`: path 集合・size が一致必須、sha256 は
  参考値。tolerance payload の比較材料は baseline 側に
  `golden/payloads/` として保存する — 要素数が
  `SMALL_PAYLOAD_MAX`(既定 65536)以下の payload は
  **全要素値を JSON 化**して要素ごとに tolerance 比較、
  それより大きい payload は **統計 fingerprint**
  `{count, min, max, sum}`(f64 加算順差は tolerance 内に
  収まる)を保存し fingerprint 同士を tolerance 比較する。
  decode には table/tensor の公開レイアウト仕様を使う。
  非有限値(NaN/±Inf)は JSON 化時に
  `{"$nonfinite": "NaN"|"Infinity"|"-Infinity"}` の sentinel
  で表現し、JSON の null 潰れを回避する
- `class: "record-only"`: path 集合のみ比較(size/sha256 は
  記録)。該当: JSON 成果物全般(機械依存パス・ID の byte
  長を含むため size も record-only)、`video_thumbs`(b64
  ペイロード)、`al-viz` 系成果物(廃止予定)、log、`error`
  メッセージ文字列(OS メッセージ文言は環境依存)

同一 file role に複数 class が混ざる場合は cell/column 単位で
分ける(table の列型ごと: I64/Utf8 列は bit-exact、F64 列は
tolerance)。

### 数値契約の区分

- **bit-exact 必須(意味が変わる差として扱う)**:
  整数・離散値全般 — sample/frame 数・index、PTS/timebase
  等の時刻観測値、enum/文字列値(validity、confidence、
  method、kind、role)、受理/拒否等の判定、event の有無・
  件数、provenance の入力 sha256・role、PCM サンプル値、
  artifact 構成(索引の path 集合)、table の I64/文字列列、
  argmax の「選択位置」等連続値から導出される離散結果
- **許容誤差あり**: f64 連続値 — NCC/GCC-PHAT スコア、
  STFT/CSD/coherence 複素スペクトル、band gain、lag/
  offset 推定値、residual 数値、時刻推定(秒)等の解析
  観測値、table の F64 列、tensor `.bin` ペイロード
- **記録のみ・比較対象外**: 計時値、絶対パス、乱数 ID
  そのもの、`error` メッセージ、log 出力、派生 AVI の
  コンテナ sha256、`al-viz` 系成果物の内容(索引記録のみ)、
  `inspect.json`・生 `manifest.json`
- **意味が変わらない条件**: bit-exact 面が全て一致し、
  許容誤差面が契約 tolerance 内に収まること

### 比較(comparator)意味論

- f64 ペア `(a, b)` の一致判定: `|a-b| <= max(abs_tol,
  rel_tol * max(|a|, |b|))`。既定 `rel=1e-9, abs=1e-12`。
  `contracts.json` で normalized path の glob 単位に上書き
  可能(例: `intermediate/**` の tolerance 緩和)
- 配列/テンソルは要素ごとの要素一致で、shape・要素数は
  bit-exact
- 整数値(JSON 上の整数・I64 列・u64 index)は厳密一致。
  Node.js comparator は `>2^53` の整数を検出したら契約違反
  として報告する(現状の値域では未到達のはず)
- `null`: JSON 面では非有限 f64(NaN/±Inf)も欠損も `null`
  に潰れる。`null == null` は一致とするが、非有限の区別が
  意味を持つ面は `.bin` payload 比較で担保し、JSON null は
  「非有限または欠損」として記録する
- bit-exact 面の不一致・許容誤差超過は「移行差分」として
  列挙され、最終判定は人が行う

### 環境条件

- bit-exact 契約の保証は「記録した environment と同一の
  toolchain・OS/ABI・CPU SIMD 機能レベル」に限定する。
  別環境での比較は tolerance 面のみを有効とし、bit-exact
  面の差は「環境差の可能性あり」として別枠で報告する。
  `environment.json` に CPU モデル・コア数を記録し、SIMD
  dispatch 依存である旨を注意書きする
- A/A(同一バイナリ 2 回実行)で bit-exact 面が一致しない
  場合: その面は「現行実装でも非決定的」という観測事実として
  `determinism.json` に記録し、根拠を添えて record-only や
  tolerance に再分類する(検出自体が baseline の価値)。
  直列化順序の正規化等、解析アルゴリズムに触れない修正は
  別 Issue で扱い、本件では実施しない

### 計測方法(実測できる範囲の確定)

- **stage 別**: wall 時間のみ(manifest `duration_ms`)。
  stage 別の CPU/RSS/IO はポーリングでは stage 境界に帰属
  できないため **実測未実施** と明記する
- **run レベル**: e2e wall(外部タイマー)、CPU 時間
  (user+kernel 合計、子プロセスの OS 情報から取得:
  Windows は `Get-Process`、POSIX は `/proc`/`ps`)、
  peak RSS(定期ポーリングの標本最大値であり、真の peak を
  下回り得る推定下限として記録する)
- **I/O**: OS の IO counter が取得できる環境ではその値、
  取れない環境では「読み=入力総サイズ・書き=成果物総
  サイズ」の近似値を記録する(中間成果物の stage 間再読込は
  近似に含まれない点を明記)
- **alloc**: 代表処理を in-process で計測する counting
  allocator 付き bench(example)で alloc 回数・bytes を
  記録する
- **成果物サイズ**: ファイルシステム walk で role/領域別に
  集計する
- **決定性(A/A)**: 同一入力 2 回実行し、正規化面と
  `.bin`/`.f64le`/`.wav` の sha256 を比較し、一致した面と
  しなかった面を `determinism.json` に記録する
- **comparator 自己検証**: baseline に摂動を加えた仮コピーを
  compare に通し、bit-exact 差・tolerance 超過・構造差が
  正しく報告されることを確認する

### al-viz 系成果物の扱い

Visualization Dataset・export HTML は #105/#106 で廃止予定の
ため golden 比較面に含めない。`canonical` run の索引に
存在・サイズ・sha256 を記録し、移行前比較の材料とするのみ。

### Consequences

* Good, because baseline が clone 済み repo + ffmpeg のみで
  再現・比較でき、外部サービスや大容量ストレージを要しない
* Good, because bit-exact/tolerance/record-only の区分と
  comparator 意味論を先に固定するため、移行差分の報告を
  機械化できる
* Good, because run matrix + coverage 台帳で、Issue #98
  列挙項目の担保先が曖昧さなく追跡できる
* Bad, because 正規化・比較 script の維持対象が増え、
  comparator 自体の正しさを保つ必要がある(自己検証を
  組み込む)
* Bad, because 性能値・bit-exact 面は実行環境に結び付き、
  別環境では golden 契約の適用範囲が狭まる
* Bad, because 派生 AVI のコンテナ bytes は環境依存のまま
  (契約から外す形で扱う)

### Confirmation

* `tools/baseline` の script で同一手順の再計測が成功し、
  保存済み `runs/` と同じ構造の結果を生成できること
* A/A 比較が記録され、bit-exact 契約面が同一環境で一致
  している(または非決定面が観測・記録されている)こと
* `resume` run の normalized golden が `canonical` と一致
  すること
* comparator の摂動自己検証が pass すること
* spec の Acceptance Criteria が全て満たされること

## Implementation Plan

* Affected paths: `baseline/`(新規)、`tools/baseline/`(新規)、
  `mise.toml`(baseline 計測 task)、欠損 guard が見つかった
  crate の tests(`crates/al-*/tests/`)
* Patterns to follow: ID/path の正規化は正規表現置換ではなく
  「run 全体で収集した mapping を値・文字列・ファイル名に
  一貫適用」。計測は既存 `al` CLI と manifest を駆動源にし、
  product code に計測コードを持ち込まない(alloc 計測は
  dev-only の example で行う)
* Patterns to avoid: baseline 値の手編集・捏造、`docs/` に
  大きな生成 JSON を混在させること、product 側への互換
  レイヤ追加(Pre-release 方針)、派生 AVI のコンテナ bytes
  を比較契約に含めること
* Dependencies or configuration: 新規外部依存なし。
  Node.js は mise.toml 既存 pin(26.12.1)を使用。
  ffmpeg は再現用に global mise 等の外部ツールとして
  扱い、版数を記録する(repo toolchain には追加しない)

## Verification

* [ ] `baseline/` に environment/inputs/config/coverage/
  contracts/runs/bench が保存されている
* [ ] 計測 script を実行し、同等構造の baseline が再生成
  される
* [ ] A/A 比較結果が記録され、bit-exact 契約面が同一環境で
  一致している
* [ ] resume run の normalized golden が canonical と一致する
* [ ] `mise run test` / `mise run lint` / `mise run fmt` が
  green

## Pros and Cons of the Options

### baseline/ に正規化 JSON + script(commit)

* Good, because clone で残り・diff 可能・docs を汚染しない
* Bad, because リポジトリに生成データが増える(量は
  正規化+索引に絞れば数十 MB 未満に収まる)

### untracked 領域(target/bench-baseline 等)に保存

* Good, because repo を汚染しない
* Bad, because clone で失われ「固定した比較基準」にならない

### docs/ 配下に埋め込む

* Good, because 文脈と同居する
* Bad, because canonical document 群に機械生成 JSON が
  混在し artifact graph 監査・index 生成を汚染し得る

### 生成果物・派生 AVI を LFS/外部保存

* Good, because 完全な後検証材料が残る
* Bad, because 5.5 GB 級のバイナリ・約 220 MB の派生 AVI を
  全員が持つ必要があり、比較に必要なのは正規化面と索引
  だけで保存コストに見合わない

## More Information

* spec: `docs/specs/pre-migration-baseline-golden.md`
* 調査: `docs/discovery/pre-migration-baseline-issue-98.md`
* 関連 ADR: `docs/adr/stage-timing-manifest-persistence.md`
  (stage 計時の記録面)、`docs/adr/measurement-over-verdict.md`
  (判定ではなく観測を残す方針)
* Issue: <https://github.com/scarecrow173/audio-lens/issues/98>
