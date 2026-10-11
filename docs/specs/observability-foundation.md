---
id: "SPEC-034EdkEKW7URB22O9btHFS"
type: "spec"
status: "approved"
title: "observability foundation: logging・計時基盤の導入スコープ"
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
  related:
    - "../adr/observability-dedicated-crate.md"
    - "../adr/logging-sink-format-activation-contract.md"
    - "../adr/stage-timing-manifest-persistence.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/observability-foundation-logging-and-profiling.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# observability foundation: logging・計時基盤の導入スコープ

## What Should Be Built

計装対象 crate(Scope で列挙: al-cli・al-job・al-locate・
al-analysis・al-media・al-viz・al-dataset)を対象とする logging +
計時基盤(observability foundation)を導入する。専用 crate
`al-observability` を新設して sink init/format/routing/activation
解釈を集約し、leaf crate は `tracing` 標準 API で emit のみを行う
(leaf crate は `al-observability` に依存しない)。log は
stderr + file(`<base_dir>/logs/`、run/resume のみ)の dual
sink、各 sink 独立に text/JSON を切替可能、既定 OFF で
`AL_LOG`/`--log` の EnvFilter directive により有効化する。
あわせて stage 計時を `StageRecord.duration_ms` として manifest
に常時永続化し、L1 計測に criterion(al-math dev-dep)、L3 計測に
samply(外部ツール随時、文書化のみ)を導入する。

## Why Now

`al-math-simd-reduction` の vec プリミティブ集約を進めるにあたり、
「先に Logging と Profile の方針を決めて全体に導入する」という順序
変更がユーザーにより確定した。現状 AudioLens の観測可能性は
manifest・feature records 等の構造化 artifacts による「結果」の
記録のみで、(1) 進行・診断のログ経路が production 経路に存在せず、
(2) ステージ単位の時間配分を計測する証拠チャネルがない。最適化
施策の優先度付けと効果検証に使う計測基盤を先行整備する。

計測の目的は job 単位デバッグではなくシステム全体のボトルネック
調査であり、job 横断で俯瞰してから詳細にドリルダウンする使い方を
前提とする(ユーザー明示)。この目的が file sink の集約先・
EnvFilter 粒度・既定 OFF の各決定を方向づけた。

## Users and Value

- 最適化・ボトルネック調査の担当者: stage 単位の時間配分が
  manifest に構造化して残り、job 横断の俯瞰分析が可能になる。
  `AL_LOG`/`--log` で全体=info・特定 crate=debug のドリルダウンが
  1経路で行える
- 実行中の進行を確認したい利用者: stderr の text log でリアルタイム
  に stage 遷移と所要時間を追える
- ログを機械処理する利用者・ツール: file sink の JSON-lines を
  そのまま集計・可視化の入力にできる
- CLI 出力を機械処理する既存利用者: log は stdout に流さないため
  `inspect` の JSON 出力が汚染されない

## Scope

### In Scope

- `crates/al-observability` の新設 — sink init、format 配線、
  routing、activation 解釈を所有(詳細は
  `docs/adr/observability-dedicated-crate.md`)
- stderr + file(`<base_dir>/logs/`)の dual sink routing。file は
  tracing-appender の rolling + 世代数管理。既定値: daily
  rotation、保持 7 世代、file 名 `al.YYYY-MM-DD.log`。file sink は
  `run`/`resume` のみで、読み取り専用の `inspect` は stderr sink
  のみとし file への書込みを行わない
- per-sink format 切替: 各 sink が独立に text/JSON を設定可能で
  同時併用できる。既定は file=JSON-lines、stderr=text
- activation 契約: 既定 OFF。`AL_LOG`(EnvFilter directive、unset
  または空文字は未指定扱いで OFF)+ `--log <directive>` flag
  (値必須)で有効化。優先順位は per-concern で、`--log` は
  `AL_LOG` を、`--log-format` は `AL_LOG_FORMAT` をそれぞれ
  全体置換で上書きする。`--log-format` 単独(activation 指定
  なし)では有効化されない。format 指定は `AL_LOG_FORMAT` /
  `--log-format` で `<sink>=<format>` のカンマ区切り directive
  (例: `stderr=text,file=json`)。sink 語彙は `{stderr, file}`
  の閉集合で、未知の sink/format 値・壊れた directive は
  `fail()` で fail-fast。詳細は
  `docs/adr/logging-sink-format-activation-contract.md`
- Scope で列挙した計装対象 crate の網羅的 instrumentation(各
  crate の主要境界 + stage 内部)。対象と計装点:
  - al-cli: subcommand dispatch
  - al-job: runner・stage 境界(`StageRunner::execute` が
    `stage.run` を stage span で囲む)
  - al-locate: method span(例 `locate::ncc`)
  - al-analysis: family span(例 `analysis::spectral_envelope`)
  - al-media: ingest 経路
  - al-viz: visualization / export 経路
  - al-dataset: dataset stage 経路
  対象外: al-core(純粋型のみで emit 点なし)、al-math(ホット
  カーネル — カーネル性能は criterion の責務で tracing は入れ
  ない)、al-testkit(テストヘルパで観測経路なし)。
  span 命名・field 規約は本 spec の Requirements で統一
- `StageRecord.duration_ms` の manifest 常時記録(log 有効性と
  無関係)+ runner が stage 完了を tracing event でも emit。
  旧 manifest の読込は維持する(`Option<u64>` は serde で
  missing→None が既定のため `#[serde(default)]` は厳密には
  冗長だが、互換意図の明示として付ける)。詳細は
  `docs/adr/stage-timing-manifest-persistence.md`
- criterion の al-math dev-dependency としての導入(add 時点で
  crates.io 公開から ≥7 日経過したバージョンを version pin)
- samply による CPU sampling の手順を `docs/` に文書化(計装不要の
  外部ツールとして随時使用)
- metadata-only 規約(len/hash/id を記録し、raw PCM・巨大 field は
  log に載せない)の文書化と、capture subscriber で規約違反を
  検出する regression test(検出機構は Acceptance Criteria を
  参照)
- 既存 examples ベンチ(bench_dsp/bench_ncc/bench_correlation/
  bench_locate_prepared)の当面維持 — 粗い E2E 計測として残す

### Out of Scope

- raw payload capture の opt-in 機構(PCM 等の生データを log に
  記録する経路は作らない)
- 常時計装プロファイラ(puffin/tracy 等)の導入
- CI 上でのベンチ実行・回帰ゲート(CI 基盤自体が未整備)
- CPU sampling プロファイラのバイナリ内蔵(samply は外部ツール
  として随時使う)
- 既存 examples ベンチの criterion への移行
- `--quiet` 相当の stderr 抑制 flag(stderr では tracing 行と
  `fail()` error 行が行単位で interleave する共存規約とする)
- log を stdout へ出力する選択肢(`inspect` の JSON 契約保護のため
  恒久的に非対象)

## Requirements

- 依存方向は2系統とも一方向: `al-cli` → `al-observability` →
  `tracing-subscriber`/`tracing-appender`(init 系統)、および
  leaf crate → `tracing`(emit 系統)。leaf crate は `tracing`
  のみに直接依存し、`al-observability` や subscriber の存在を
  知らない。init は `al-cli` のみが呼び、各 `cmd_*`
  (`run`/`resume`/`inspect`)内で `--base-dir` 検証後に行う。
  file sink(`<base_dir>/logs/`)は `run`/`resume` のみで、
  読み取り専用の `inspect` は stderr sink のみとし file への
  書込みを行わない
- 依存追加は `[workspace.dependencies]` に `tracing`・
  `tracing-subscriber`(feature: `fmt`, `env-filter`, `json` —
  `json` feature は file sink の JSON-lines に必須)・
  `tracing-appender` を定義し、leaf crate は
  `tracing.workspace = true` で参照する
- logging は既定 OFF とし、OFF 時は実用上 zero overhead(subscriber
  未設定時の tracing の no-op コストのみ)であること
- log は stdout に一切流さない。stderr sink は既存 `fail()` の
  `error: {message}` 行と行単位で interleave して共存する
- activation の解釈: `--log`/`--log-format` はサブコマンド後の
  args でのみ解釈する(既存 `parse_flag`/`flag_present`
  パターン)。`al --log x run` は従来どおり unknown subcommand。
  値なしの `--log` は `--input-ids` と同じ
  `flag_present && parse_flag.is_none()` チェックで
  `error: --log requires a value` + exit 2。EnvFilter directive
  の parse 失敗、未知の sink 名(`stdout` を含む)/format 値、
  `<sink>=<format>` 形でない壊れた directive も `fail()` の
  `error:` 行 + exit 2 で fail-fast する
- file sink init の失敗(read-only dir 等)は job をエラーに
  せず、stderr に warn 行を出して stderr-only に degrade する —
  「log 機構は job を壊さない」不変条件
- `StageRecord.duration_ms` は log の有効性と無関係に常時記録する。
  意味は「その stage の最新実行の経過時間(ミリ秒)」で、resume で
  再実行された stage は新しい計測値で上書き(合算しない)、
  Completed/Skipped で実行されない stage は既存値を保持する。
  未実行・旧 manifest 由来の record は `None`。「成否に関わらず」
  の適用範囲は `stage.run` の `Err` 戻りまでで、`stage.run` が
  panic した場合は unwind により計時注入の適用範囲外となり
  記録されない
- span/field は metadata-only 規約に従う: サイズ・件数・
  hash・ID・stage 名・method 名等のメタデータのみを field に載せ、
  raw PCM・波形データ・巨大バッファは記録しない
- span 命名: span name は `&'static str` とし、実行時の動的生成は
  しない。stage 境界 span は固定名 + field の
  `info_span!("stage", stage = "<serde snake_case kind>")` 形とし、
  `stage::<kind>` のような動的名は取らない。stage 内部のサブ単位は
  `<crate>::<unit>`(例: `locate::ncc`、
  `analysis::spectral_envelope`)とする。EnvFilter の target は
  span 名ではなく module path(例 `al_locate`)であり、
  `AL_LOG=al_locate=debug` は crate 単位の絞込みになる。span 名
  単位で絞るのではなく、event の field(`stage` 等)を見て判別
  する使い方とする
- field 語彙: `job_id`・`stage`・`duration_ms`・`len`・
  `sha256`・`input_id`・`method` を共通利用する。run 単位の
  span/event には `job_id` を必須 field とする —
  `<base_dir>/logs/` の集約 file で event→job 帰属を取るために
  必要
- runner は `stage.run` を stage 境界 span で囲み、成功・失敗の
  両分岐で stage 完了 event を emit する(成功は `info!`、失敗は
  `error!`、両方 `stage`・`job_id`・`duration_ms` field 付き)
- criterion は `al-math` の dev-dependency とし、add 時点で
  crates.io 公開から ≥7 日経過したバージョンを version pin する。
  選定した版と公開日は Implementation Record に記録する
- `run`/`resume` の key=value 行と `inspect` の stdout JSON
  schema の形式を変更しない。ただし `inspect` の JSON には
  `StageRecord.duration_ms` の field 追加のみを差分として許容
  する
- al-cli の統合テストは子プロセス側の `AL_LOG`/`AL_LOG_FORMAT`
  を `Command::env_remove` で除去(env scrub)して実行する —
  edition 2024 で `std::env::set_var` は unsafe のため、親側では
  なく子プロセスの環境を scrub する形とする
- 既存の分析意味・manifest の既存 field・golden 出力を変更しない
  (`duration_ms` の追加を除く)

## Acceptance Criteria

- [ ] `AL_LOG`/`--log` 未指定(既定 OFF)で
  `mise exec -- cargo test --workspace` が全件 pass する。
  `run`/`resume` の stdout 出力形式は変更前と一致し、`inspect`
  の stdout は `StageRecord.duration_ms` field 追加のみの差分
  (それ以外の schema は一致)である
- [ ] `run`/`resume` で `--log` または `AL_LOG` 指定時、stderr と
  `<base_dir>/logs/al.YYYY-MM-DD.log` の両 sink に stage 完了を
  含む event が出力される。`inspect` では stderr sink のみが
  有効になり `<base_dir>/logs/` に file が作られない
- [ ] `AL_LOG` が空文字の場合は未指定扱いで OFF になり、
  `--log-format` 単独(activation 指定なし)では logging が
  有効化されない(既定 OFF のまま)
- [ ] 値なしの `--log` が `error: --log requires a value` +
  exit 2 で失敗する。未知の sink 名(`stdout` を含む)・未知の
  format 値・`<sink>=<format>` 形でない壊れた directive も
  `error:` 行 + exit 2 で fail-fast し panic しない
- [ ] `--log-format`/`AL_LOG_FORMAT` で各 sink の format が独立に
  切り替わり、file=JSON-lines と stderr=text の同時併用が動作する
- [ ] `--log` flag の directive が `AL_LOG` env より、
  `--log-format` が `AL_LOG_FORMAT` よりそれぞれ優先される
  (per-concern の全体置換)
- [ ] `AL_LOG=al_locate=debug` で al-locate の stage 内部 span
  (例 `locate::ncc`)の event が観測できるなど、Scope で列挙
  した計装対象 crate それぞれで内部 span が観測できる
- [ ] logging 有効時でも `al inspect` の stdout が tracing 行を
  含まず有効な JSON として parse できる
- [ ] file sink init が失敗する環境(read-only dir 等)でも job
  が成功し、stderr に warn 行が出て stderr-only に degrade する
- [ ] metadata-only 規約(raw PCM・巨大 field を emit しない)を
  検証する regression test が存在し pass する。test は
  `tracing::subscriber::with_default` と自前 Layer で event の
  field を収集し、field 値のサイズ(目安: 1 KiB 超)や既知の
  巨大データ field が無いことを assert する
- [ ] `manifest.json` の完了した各 `StageRecord` の
  `duration_ms` に値が存在する(`Some`。1ms 未満の stage では
  0 になり得るため正の値は要求しない)かつ log 無効時にも値が
  残る。`duration_ms` を持たない旧形式 manifest の
  `Manifest::load` が成功する
- [ ] resume で stage を再実行した場合 `duration_ms` が新計測値で
  上書きされ、実行されなかった stage の値が保持されることを
  test で確認する
- [ ] logging OFF 時の overhead を既存 examples ベンチ(例
  `bench_dsp`)の導入前後比較で計測して差を Implementation
  Record に記録する、または criterion で dispatch 経路の空転
  cost を測定して記録する
- [ ] `mise exec -- cargo tree --workspace` の出力を検査し、
  `tracing-subscriber`/`tracing-appender` が `al-observability`
  経由のみで依存グラフに現れ、leaf crate が直接依存しない
  ことを確認する(出力を目視確認するか `findstr` 等で抽出)
- [ ] al-cli の統合テストが `AL_LOG`/`AL_LOG_FORMAT` を
  `Command::env_remove` で子プロセス側から除去(env scrub)
  しており、環境変数の有無で結果が変わらない
- [ ] `mise exec -- cargo test --workspace`、
  `mise exec -- cargo clippy`、`mise exec -- cargo fmt --check`
  が green である
- [ ] `mise exec -- cargo bench -p al-math` で criterion ベンチが
  実行できる。採用した criterion の版と crates.io 公開日が
  Implementation Record に記録されている
- [ ] samply を使ったプロファイリング手順が `docs/` に存在する

## Deferred Design Concerns

- raw payload capture の opt-in — 現時点では metadata-only で
  十分と判断。log で原因追跡に raw データが必要な事案が出た時点で
  再検討する。無視した場合のリスク: 再現に別手段(手動再実行・
  artifact 検査)が要る
- CI ベンチ・回帰ゲート — CI 基盤自体が未整備のため見送り。
  CI 整備時に criterion の baseline 比較を載せるか再検討する
- 常時計装プロファイラ(puffin/tracy) — sampling で分解能が
  不足する調査が出た時点で再検討する
- export package への stage duration 同梱 — 現状 package は job
  manifest を含まず analysis dataset のみをコピーする境界を維持
  する。package 側で実行時間を表示したい要求が出た時点で job
  manifest コピー追加を検討する

## Sources and Relations

- 調査: `docs/discovery/observability-foundation-logging-and-profiling.md`
  (D1–D11 の確定決定、cheaper との9軸比較、計時注入点の特定)
- 決定: `docs/adr/observability-dedicated-crate.md`(crate
  topology)、`docs/adr/logging-sink-format-activation-contract.md`
  (sink/format/activation 契約)、
  `docs/adr/stage-timing-manifest-persistence.md`(stage 計時の
  manifest 永続化)
- 参照実装(外部リポジトリ): `F:\repositry\cheaper` の
  `docs/adr/logging-output-and-activation-contract.md`、
  `docs/adr/consolidate-observability-behind-a-dedicated-crate.md`
- 後続チェーン: `docs/discovery/al-math-simd-reduction.md`
  (本基盤の criterion 導入を前提とする)
