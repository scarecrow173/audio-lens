---
id: "TSPEC-034EeuZabJTmVBb077k5GE"
type: "test-spec"
status: "approved"
title: "observability foundation 検証仕様"
created: "2026-10-11"
updated: "2026-10-11"
owners: []
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
    - "docs/designs/observability-foundation.md"
  derived-by: []
  verifies:
    - "docs/specs/observability-foundation.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# observability foundation 検証仕様

## Purpose

`docs/specs/observability-foundation.md` の Acceptance Criteria を
検証可能な形に分解した検証仕様。logging 基盤(activation・sink・
format・degrade)、stage 計時の manifest 永続化、依存方向・
metadata-only の不変条件が、spec の契約が変わらない限り保持
されることを保証する。

## Feature

既定 OFF の dual-sink logging、stage 計時の manifest 永続化、
および init/emit 分離の依存構造を備えた observability
foundation。

## Rules

### Rule A: 既定 OFF と非回帰

- Rule: `AL_LOG`/`--log` 未指定では logging は有効化されず、
  実用上 zero overhead(subscriber 未設定時の no-op コストのみ)
  である
- Rule: log OFF で既存動作が変わらない — `run`/`resume` の
  stdout key=value 形式は変更前と一致し、`inspect` の stdout
  schema は `StageRecord.duration_ms` 追加のみの差分である
- Rule: `AL_LOG` が空文字の場合は未指定扱いで OFF になる
- Rule: `--log-format` 単独(activation 指定なし)では logging
  は有効化されない

### Rule B: activation 契約

- Rule: activation directive は `AL_LOG` env または `--log`
  flag の EnvFilter directive で与え、flag が env を per-concern
  の全体置換で上書きする(`--log`→`AL_LOG`、
  `--log-format`→`AL_LOG_FORMAT`、directive 単位の merge は
  しない)
- Rule: `--log`/`--log-format` は値必須。flag が存在して値が
  無い場合は `error: --log requires a value`(同様に
  `--log-format`)+ exit 2 で失敗する
- Rule: flag の空文字値(`--log ""`/`--log-format ""`)は
  malformed として `error:` 行 + exit 2 で fail-fast する
  (env の空文字=未指定とは扱いが異なる)
- Rule: EnvFilter directive の parse 失敗・未知の sink 名
  (`stdout` を含む)・未知の format 値・`<sink>=<format>` 形
  でない壊れた directive は `error:` 行 + exit 2 で fail-fast
  し panic しない
- Rule: flag はサブコマンド後の args でのみ解釈する。
  `al --log x run` は従来どおり unknown subcommand になる

### Rule C: sink と format

- Rule: `run`/`resume` で logging 有効時、stderr と
  `<base_dir>/logs/al.YYYY-MM-DD.log` の両 sink に stage 完了を
  含む event が出力される
- Rule: `inspect` は stderr sink のみが有効で、
  `<base_dir>/logs/` に file が作られない
- Rule: 各 sink の format は独立に `text`/`json` を切替可能で、
  既定は file=JSON-lines・stderr=text。同時併用できる
- Rule: file は daily rotation・保持 7 世代・file 名
  `al.YYYY-MM-DD.log` 形式である
- Rule: file sink init の失敗(read-only dir 等)は job を
  エラーにせず、stderr に warn 行を出して stderr-only に
  degrade する(「log 機構は job を壊さない」不変条件)
- Rule: log は stdout に一切流さない。logging 有効時でも
  `al inspect` の stdout は tracing 行を含まず有効な JSON と
  して parse できる

### Rule D: 計装と metadata-only

- Rule: span 名は `&'static str` で、stage 境界は
  `info_span!("stage", stage = "<serde snake_case kind>")`、
  stage 内部は `<crate>::<unit>`(例 `locate::ncc`)とする。
  EnvFilter の target は span 名ではなく module path である
- Rule: field には `job_id`・`stage`・`duration_ms`・`len`・
  `len_a`・`len_b`・`sha256`・`input_id`・`method` の語彙の
  メタデータのみを載せ、raw PCM・波形データ・巨大バッファは
  記録しない。`len_a`/`len_b` は locate の template/signal
  入力長のように `len` を用途別に分ける派生語彙とする。
  run 単位の span/event には `job_id` を必須とする
- Rule: 計装対象 crate(al-cli・al-job・al-locate・
  al-analysis・al-media・al-viz・al-dataset)それぞれで内部
  span が観測できる。al-core・al-math・al-testkit は計装
  対象外である

### Rule E: stage 計時の manifest 永続化

- Rule: 完了した `StageRecord` の `duration_ms` は log 有効性と
  無関係に常時 `Some` として記録される(1ms 未満は 0 になり
  得る)。意味は「その stage の最新実行の経過時間(ミリ秒)」
- Rule: resume で再実行された stage は新計測値で上書きし
  (合算しない)、Completed/Skipped で実行されなかった stage
  は既存値を保持する。未実行・旧 manifest 由来の record は
  `None`
- Rule: `stage.run` の `Err` 戻りでも計時・記録される
  (panic による unwind は適用範囲外)
- Rule: `duration_ms` field を持たない旧形式 manifest が
  `Manifest::load` で読み込める
- Rule: runner は stage 完了 event を成功(`info!`)・失敗
  (`error!`)の両分岐で `stage`・`job_id`・`duration_ms`
  field 付きで emit する

### Rule F: 依存方向

- Rule: `tracing-subscriber`/`tracing-appender` は
  `al-observability` 経由のみで依存グラフに現れ、leaf crate
  (al-cli 除く)は `tracing` のみに直接依存する。
  `al-observability` に依存するのは al-cli のみで、al-cli の
  `tracing-subscriber` は dev-dep のみである
- Rule: subscriber init は `crates/al-cli/src/main.rs` の各
  `cmd_*` 内(`--base-dir` 検証後)のみから行われる
- Rule: al-cli の統合 test は `AL_LOG`/`AL_LOG_FORMAT` を
  `Command::env_remove` で子プロセス側から除去(env scrub)
  して実行し、環境変数の有無で結果が変わらない

### Rule G: 計測基盤

- Rule: criterion は al-math の dev-dependency とし、crates.io
  公開から ≥7 日経過した版を pin する。`cargo bench -p
  al-math` で smoke ベンチが実行できる
- Rule: samply による CPU sampling の手順が `docs/` に文書化
  されている(外部ツール随時使用、計装不要)
- Rule: logging OFF 時の overhead が計測・記録される(examples
  ベンチの前後比較、または criterion による dispatch 経路の
  空転 cost)

## Examples

- Example(OFF/非回帰): `AL_LOG` 未指定で `mise exec -- cargo
  test --workspace` を実行すると全件 pass し、`al run` の
  stdout が変更前の key=value 形式と一致する。
  `<base_dir>/logs/` は作られない
- Example(activation): `al run --log info --base-dir <dir> ...`
  を実行すると stderr に text、`<base_dir>/logs/al.YYYY-MM-
  DD.log` に JSON-lines で stage 完了 event が出力される
- Example(inspect): `al inspect --log info` を実行すると
  stdout が serde_json で parse できる JSON のままで、
  `<base_dir>/logs/` に file が作られない
- Example(fail-fast): `al run --log`(末尾・値なし)は
  `error: --log requires a value` + exit 2。
  `AL_LOG="=info"`(target が空の壊れた directive)、
  `AL_LOG="info=bogus"`(未知 level)、`--log-format
  stdout=text`、`--log-format file=bogus`、`--log-format
  stderrtext`、`--log ""` も同様に `error:` 行 + exit 2 で
  失敗し panic しない。EnvFilter の parse は lenient で、
  `bogus!!!` のような文字列は valid target directive として
  受理されるため parse 失敗例には使えない。`al --log x run`
  は unknown subcommand
- Example(優先順位): `AL_LOG=warn` + `--log info` では info が
  効き、`AL_LOG_FORMAT=file=text` + `--log-format file=json`
  では file=json が効く(per-concern 全体置換)
- Example(format 切替): `--log-format stderr=text,file=json`
  で stderr=text・file=JSON-lines の同時併用が動作する
- Example(ドリルダウン): `AL_LOG=al_locate=debug` で file
  JSON-lines に al-locate の内部 span event(例 `locate::ncc`)
  が含まれ、他 crate は既定 level に留まる
- Example(degrade): file sink init が失敗する環境
  (`<base_dir>/logs/` が作れない read-only dir 等)で `al run
  --log info` が成功し、stderr に warn 行が出て stderr-only
  に degrade する
- Example(retention): 8 世代超の run で `<base_dir>/logs/` 配下
  が 7 世代を超えず、file 名が `al.YYYY-MM-DD.log` 形式である
- Example(timing): `al run` 完了後の `manifest.json` で完了
  stage の `duration_ms` が `Some`(0 含む)、未実行 stage が
  `None` である。`duration_ms` を持たない旧 manifest が
  `Manifest::load` で読める
- Example(resume): Failed 後の resume で再実行 stage の
  `duration_ms` が新計測値に上書きされ、Completed 維持の
  stage の値が変わらない
- Example(metadata-only): `tracing::subscriber::with_default`
  + 自前 Layer で event の field を収集し、field 値サイズが
  1 KiB を超えるものや既知の巨大データ field が無いことを
  assert する regression test が pass する
- Example(依存方向): `mise exec -- cargo tree --workspace
  --edges normal` で `tracing-subscriber`/`tracing-appender`
  が `al-observability` 経由のみに現れる

## Guarantees

- log OFF が既存の全動作(stdout 契約・golden 出力・分析意味)
  を保つこと
- activation/sink/format の不正入力が panic せず `error:` 行 +
  exit 2 の既存エラー規約で失敗すること
- log 機構の失敗が job を壊さないこと(degrade 不変条件)
- `duration_ms` が log 有効性と無関係に manifest に残り、旧
  manifest 読込を壊さないこと
- leaf crate が `tracing` 標準 API だけで emit でき、subscriber
  の存在を知らないこと
- al-cli 統合 test が環境変数の有無に依らず決定的であること

## Non-goals

- raw payload capture(PCM 等の生データを log に記録する経路)
- 常時計装プロファイラ(puffin/tracy)の検証
- CI 上でのベンチ実行・回帰ゲート
- 既存 examples ベンチの criterion 移行
- `stderr=json` 指定時の stderr 全体の機械 parse 可能性
  (`fail()` の plaintext 行との混在は既知)
- 複数 `al` プロセスの同一日 log file への append の行原子性
  (tracing-appender 依存の既知制約)
- stage 計時の Layer ベース close-time 計測への置換

## Risk

- この保証を失うと、既定 OFF が守られず instrumentation が
  常時コスト・stdout 汚染(`inspect` JSON 破壊)を招き、
  ボトルネック調査の証拠チャネル自体が信用できなくなる。
  `duration_ms` の読込互換喪失は既存 job の resume/inspect を
  壊す
