---
id: "ADR-034EdkLsGVFb43OUttkjdR"
type: "adr"
status: "accepted"
title: "ログ sink・format・activation の契約"
created: "2026-10-11"
updated: "2026-10-11"
owners:
  - "scarecrow173"
relations:
  source:
    - "../discovery/observability-foundation-logging-and-profiling.md"
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
    - "../specs/observability-foundation.md"
    - "observability-dedicated-crate.md"
    - "stage-timing-manifest-persistence.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/observability-foundation-logging-and-profiling.md"
  derived-by:
    - "../designs/observability-foundation.md"
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

# ログ sink・format・activation の契約

## Context and Problem Statement

tracing 基盤の導入にあたり、log をどこへ(sink)、どの形式で
(format)、いつ(activation)出力するかの契約を決める必要がある。
制約として、(1) `al inspect` は manifest 全体の JSON を stdout
へ出すため log が stdout を汚すと機械処理が壊れる実害がある、
(2) file sink は job 横断の俯瞰分析に使うため集約先の選定が
目的適合性を左右する、(3) log 量を常時コストにしない activation
経路が要る(discovery D2/D3/D5/D6)。

## Decision Drivers

* **計測の目的は job 横断のシステム全体のボトルネック調査であり、
  全体俯瞰から詳細へドリルダウンする使い方を前提とする**
  (ユーザー明示)— 全 run の log が一箇所に集約されることが前提
* `inspect` の stdout JSON、`run`/`resume` の key=value 行を
  汚染しない(stdout への log は実害回避の規約として禁止)
* 既定 OFF で OFF 時の overhead を実用 zero に抑える
* 「全体=info・特定 crate=debug」のドリルダウンを1経路で実現する
  (EnvFilter directive 型)
* file 集約 dir の肥大を retention 管理で抑える
* AudioLens に credential は存在しないが、raw PCM・巨大 field を
  log に載せない metadata-only の予防規約を置く(discovery D11、
  cheaper SEC-B と同型)

## Considered Options

* stderr + file(`<base_dir>/logs/`)の dual routing、各 sink 独立に
  text/JSON 切替・同時併用可、既定 OFF、`AL_LOG`(EnvFilter
  directive)+ `--log` flag(flag 優先)
* stderr のみ既定有効 + file sink は opt-in(縮小構成)
* file sink を `<job_dir>/logs/` に置く job 単位集約
* format 固定(file=JSON-lines、stderr=text、切替機構なし)

## Decision Outcome

Chosen option: "stderr + file(`<base_dir>/logs/`)の dual
routing、各 sink 独立に text/JSON 切替・同時併用可、既定 OFF、
`AL_LOG` + `--log` flag", because 全体俯瞰(file=JSON-lines の
機械処理)と実行中の人間向け確認(stderr=text)を同時に成立
させられ、job 横断の集約先が目的に合致し、既定 OFF が overhead
を除去するため。

契約の確定内容:

* **sink**: stderr + file の dual routing。file は
  `<base_dir>/logs/`(output root 直下)に集約し、全 job の全
  run が同一 dir に書く。stdout には一切流さない。sink 語彙は
  `{stderr, file}` の閉集合で、`stdout` を含む未知の sink 名は
  `fail()` で fail-fast して拒否する — `al inspect` の stdout
  JSON を tracing 行で汚すと機械処理が壊れる実害があるため、
  stdout sink は誤指定の受理も含めて認めない
* **format**: 各 sink が独立に `text`/`json` を切替可能で同時
  併用できる。既定は file=JSON-lines、stderr=text。未知の
  format 値や `<sink>=<format>` 形でない壊れた directive も
  `fail()` で fail-fast。`stderr=json` の指定自体は許容するが、
  既存 `fail()` の `error:` diagnostics 行は plaintext のまま
  残るため stderr を JSON-lines として parse できることは保証
  しない — JSON の機械処理は file sink が責務を持つ
* **activation**: 既定 OFF。`AL_LOG` env var(unset または
  空文字は未指定扱いで OFF)に EnvFilter directive(例:
  `AL_LOG=al_locate=debug,info`)を指定するか、
  `--log <directive>` flag で有効化する。al-cli は手書き引数
  解析のため flag parse は既存の `parse_flag`/`flag_present`
  パターンで自前実装する
* **優先順位**: per-concern の全体置換。`--log` は `AL_LOG` を、
  `--log-format` は `AL_LOG_FORMAT` をそれぞれ丸ごと上書きする
  (directive 単位の merge はしない)。`--log-format` 単独
  (activation 指定なし)では logging は有効化されず、既定 OFF
  のままとする
* **format 指定**: `AL_LOG_FORMAT` / `--log-format` で
  `<sink>=<format>` のカンマ区切り directive(例:
  `--log-format stderr=text,file=json`)。未指定 sink は既定値
* **flag 契約**: flag はサブコマンド後の args でのみ解釈する
  (既存 parse パターン)。`al --log x run` は従来どおり
  unknown subcommand になる。`--log` は値必須で、末尾で値が
  無い場合は `--input-ids` と同じ
  `flag_present && parse_flag.is_none()` チェックで
  `error: --log requires a value` + exit 2 とする。`al run --log
  --stages ...` のように次 token が flag の場合は、その token
  が値として飲み込まれたうえで directive parse に失敗し、
  同じく `error:` 行 + exit 2 で検出される
* **directive parse**: EnvFilter directive は
  `EnvFilter::try_new`/`try_from` 系で parse し、失敗は panic
  ではなく `fail()` の `error:` 行 + exit 2 にする(既存エラー
  規約と一致)。panic する `EnvFilter::new` は使用禁止
* **init 位置**: subscriber init は各 `cmd_*` 内で `--base-dir`
  検証後に呼ぶ。file sink は `run`/`resume` のみ
  (`<base_dir>/logs/`)。`inspect` は読み取り専用のため stderr
  sink のみとし、file への書込みは行わない。file sink init の
  失敗(read-only dir 等)は job をエラーにせず、stderr に
  warn 行を出して stderr-only に degrade する — 「log 機構は
  job を壊さない」不変条件
* **retention**: `tracing_appender::rolling::Builder` で
  `filename_prefix("al")` + `filename_suffix("log")` +
  `max_log_files(7)` の daily rotation とする。素の
  `rolling::daily` では suffix が付かず `al.YYYY-MM-DD.log`
  形式にならないため Builder を使う
* **stderr 共存**: tracing の stderr 出力と既存 `fail()` の
  `error: {message}` 行は行単位で interleave して共存する。
  `--quiet` 相当の抑制 flag は新設しない
* **metadata-only 規約**: field には len・件数・hash・ID・
  stage 名・method 名等のメタデータのみを載せ、raw PCM・波形
  データ・巨大バッファは記録しない。run 単位の span/event には
  `job_id` field を必須とし、集約 file での event→job 帰属に
  使う(field 語彙の詳細は spec の Requirements を参照)

### Consequences

* Good, because `<base_dir>/logs/` への集約で job 横断の全 run が
  一箇所に揃い、「全体俯瞰→詳細」の分析フローに合致する
* Good, because file=JSON-lines と stderr=text を同時に使え、
  機械集計とリアルタイム確認が競合しない
* Good, because EnvFilter directive が env・flag の同一形式で
  受け付けられ、ドリルダウンが1経路で完結する
* Good, because 既定 OFF で instrumentation の常時コストが
  subscriber 未設定時の no-op コストに留まる。stage 計時は
  manifest に常時記録されるため OFF でも俯瞰データは残る
* Bad, because `<base_dir>/logs/` が肥大するため retention
  管理(rolling + 世代数)が必須になる
* Bad, because 同一 `<base_dir>` では複数 `al` プロセスが同日同
  log file に append し得る既知制約がある。行書込みの原子性は
  tracing-appender に依存し、retention による古 file の削除は
  他プロセスが open 中だと Windows で失敗し得る
* Bad, because sink ごとの format 配線と env/flag 両経路の
  directive 解釈で、設定面の実装量が単純 level 指定より増える

### Confirmation

* `AL_LOG`/`--log` 未指定時に stderr・file とも log が出力され
  ないこと。`AL_LOG` が空文字の場合も未指定扱いで OFF になる
  こと
* 値なしの `--log` が `error: --log requires a value` + exit 2
  で失敗すること。未知の sink 名(`stdout` を含む)・未知の
  format 値・壊れた directive が `error:` 行 + exit 2 で
  fail-fast し panic しないこと
* `AL_LOG=al_locate=debug,info` で file sink の JSON-lines に
  対象 crate の debug event が含まれること
* `--log-format` で file=text に切替えられること、および flag が
  env 指定をそれぞれ全体置換で上書きすること。`--log-format`
  単独では logging が有効化されないこと
* logging 有効時に `al inspect` の stdout が有効 JSON のままで、
  `inspect` 実行で `<base_dir>/logs/` に file が作られないこと
* file sink init が失敗する環境で job が成功し、stderr に warn
  行が出て stderr-only に degrade すること
* `<base_dir>/logs/` 配下が世代数上限(7)を超えず、file 名が
  `al.YYYY-MM-DD.log` 形式であること

## Implementation Plan

* Affected paths: `crates/al-observability/src/lib.rs`(公開
  API — flag/env の生値 `Option<String>` を引数で受けて優先順位・
  既定値・directive 解釈を行う `resolve` 系関数と、解釈済み
  config を受けて subscriber を構築し file writer の guard を
  返す `init` 関数。API はこの程度の薄さに留める)、
  `crates/al-cli/src/main.rs`(`--log`/`--log-format` の
  `parse_flag` による値取得、値なし `--log` の
  `flag_present && parse_flag.is_none()` チェック、env の生値
  読取りと `al-observability` への受け渡し、各 `cmd_*` で
  `--base-dir` 検証後の init 呼出しと guard 保持、`USAGE`
  への追記)
* Patterns to follow: activation 解釈(env 変数名・flag 優先・
  既定値・directive parse)は `al-observability` 内の単一関数に
  集約し、al-cli は argv と env から取った生値を渡すだけに
  する。`resolve` 系関数は env を内部で読まず env 値も引数と
  して受ける純粋関数にし、unit test 可能にする。EnvFilter は
  `tracing_subscriber::EnvFilter` を直接使用し、directive
  parse は `EnvFilter::try_new`/`try_from` 系で失敗を `Result`
  として扱う。file writer は `tracing_appender::rolling::Builder`
  (`filename_prefix("al")` + `filename_suffix("log")` +
  `max_log_files(7)`)+ `WorkerGuard` で、guard を `cmd_*`
  のスコープ終了まで保持して flush を保証する。format 層は
  `tracing_subscriber::fmt` の per-layer writer/format 構成で
  stderr layer と file layer を別々に組み、`registry().with(layer)`
  で重畳する。file sink init の失敗は warn 行を stderr に出して
  stderr-only に degrade する
* Patterns to avoid: stdout への log 出力・`stdout` sink 指定の
  受理(`inspect` JSON 保護)、env 変数を al-observability 以外で
  読む二重解釈、`resolve` 関数の内部で env を読む設計(unit
  test 不能化)、panic する `EnvFilter::new` の使用、sink ごとの
  format を単一 fmt layer で兼ねる設計(per-sink format が組め
  ない)、file sink init 失敗を `fail()` にする設計(log 機構が
  job を壊す)、素の `rolling::daily`(`al.YYYY-MM-DD.log`
  形式にならない)、log 有効化を既定 ON にする変更
* Dependencies or configuration: `[workspace.dependencies]` に
  `tracing`、`tracing-subscriber`(feature: `fmt`, `env-filter`,
  `json` — `json` は JSON-lines に必須)、`tracing-appender`
  (non-blocking writer + rolling)を追加。env var: `AL_LOG`、
  `AL_LOG_FORMAT`。flag: `--log <directive>`(値必須)、
  `--log-format <sink>=<format>[,...]`

## Verification

* [ ] `AL_LOG`/`--log` 未指定で実行し、stderr に tracing 行が
  出ず `<base_dir>/logs/` に file が生成されない(既定 OFF)ことを
  確認する。`AL_LOG` が空文字でも同様に OFF になることを確認する
* [ ] `--log info` で stderr に text、`<base_dir>/logs/` に
  JSON-lines の event が出力され、stage 完了 event が両 sink に
  現れることを確認する
* [ ] `al run --log`(値なし)が `error: --log requires a value`
  + exit 2 で失敗すること、および `al --log x run` が unknown
  subcommand のままであることを確認する
* [ ] `AL_LOG` に無効な directive、`--log-format` に `stdout`
  を含む未知 sink 名・未知 format 値・壊れた directive を指定
  した場合が `error:` 行 + exit 2 で fail-fast し panic しない
  ことを確認する
* [ ] `AL_LOG=al_locate=debug,info` で file JSON に `al_locate`
  の debug event が含まれ他 crate が info に留まることを確認する
* [ ] `--log-format file=text` で file 出力が text に切り替わり、
  flag 指定が `AL_LOG`/`AL_LOG_FORMAT` を上書きすること、
  `--log-format` 単独では logging が有効化されないことを確認する
* [ ] `--log info` 付きで `al inspect` を実行し、stdout が JSON
  として parse できること(`jq` が使えない環境では serde_json
  を通す統合 test や PowerShell の `ConvertFrom-Json` で確認)、
  および `inspect` 実行で `<base_dir>/logs/` に file が作られ
  ないことを確認する
* [ ] 世代数上限(7)を超える run で古い log file が削除され、
  file 名が `al.YYYY-MM-DD.log` 形式であることを確認する
* [ ] file sink init が失敗する環境(read-only dir 等)で job が
  成功し、stderr に warn 行が出て stderr-only に degrade する
  ことを確認する
* [ ] metadata-only 規約の regression test(capture subscriber
  で event の field を収集し巨大 field が無いことを assert
  するもの)が存在し pass する

## Pros and Cons of the Options

### dual routing + per-sink format + EnvFilter + 既定 OFF

* Good, because 俯瞰の機械処理(file=JSON)と実行中の人間向け確認
  (stderr=text)を同時に成立させ、集約先が job 横断目的に合致する
* Bad, because sink ごとの format 配線と env/flag 両経路の
  directive 解釈で実装量が増える

### stderr のみ既定 + file opt-in

* Good, because 実装が最小で file sink の retention 管理が不要
* Bad, because file 集約なしでは job 横断の俯瞰データが残らず、
  計測目的(全体俯瞰→ドリルダウン)と矛盾する

### `<job_dir>/logs/` の job 単位集約

* Good, because job 単位で証跡が完結する
* Bad, because job 横断の俯瞰ができず計測目的と直接矛盾する

### format 固定

* Good, because format 切替機構が不要で最小
* Bad, because 用途別の最適化(機械処理 vs 人間向け)が選べず、
  後から切替を足すと activation 経路の再設計になる

## More Information

* 確定経緯: `docs/discovery/observability-foundation-logging-and-profiling.md`
  の D2(file sink の置き場)・D3(sink ごとの format)・
  D5(activation と既定状態)・D6(フィルタ粒度)・D11(redaction)
* stdout 契約の根拠: `crates/al-cli/src/main.rs` の
  `inspect`/`run`/`resume` 出力形式(同 discovery の「確認済み」
  項目)
* crate topology: `docs/adr/observability-dedicated-crate.md`
* 実施スコープ: `docs/specs/observability-foundation.md`
* 参照実装(外部リポジトリ): `F:\repositry\cheaper` の
  `docs/adr/logging-output-and-activation-contract.md`
