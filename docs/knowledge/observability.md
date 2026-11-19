---
id: "KNOW-wA8yNmQrTxVbHkLpGsDf"
type: "knowledge"
status: "approved"
title: "可観測性: logging・stage 計時・span 計装・プロファイリング"
created: "2026-11-19"
updated: "2026-11-19"
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
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 可観測性: logging・stage 計時・span 計装・プロファイリング

`al-observability` クレートは、logging sink 初期化・format 配線・routing・activation 解釈を集約する専用 crate で、leaf crate は `tracing` 標準 API のみで emit し subscriber の存在を知らない(依存方向: leaf → tracing のみ)。本書は logging 契約、stage 計時、span 計装、プロファイリング基盤をまとめる。

## Activation 解釈

`resolve_activation(log_env, log_flag, format_env, format_flag)` は純粋関数で、「per-concern 全体置換」— flag が env を丸ごと置換する。env 空文字は未指定、flag 空文字は malformed で fail-fast、activation 未指定時の format directive は無視(既定 OFF)。EnvFilter parse は panic する `new` ではなく `try_new` → `InvalidDirective`。`InitError` には `Build(LoggingBuildError)` variant があり `Required` stderr sink 失敗を上位伝播する。

## sink 構成と degrade

text/json で fmt layer 具象型が変わるため per-sink builder は `Box<dyn Layer>` を返し Vec で重畳し、末尾 global EnvFilter で registry を組む。file sink は `rolling::Builder`(DAILY・prefix `al`・suffix `log`・`max_log_files(7)`)+ `non_blocking` + `with_ansi(false)`。`Builder::build` 前に `fs::create_dir_all(dir)` を実行する —— `RollingFileAppender` が dir 作成前に `prune_old_logs` を走らせ fresh dir で stray stderr error を出す問題への対処。file sink 構築失敗時は stderr warn + stderr-only degrade で `Ok` を返す。

`LogGuard` は non_blocking `WorkerGuard` を保持し Drop で flush を保証する(早期 `fail()` でも cmd スコープ全体で guard を保持する契約)。dispatch span は init 成功後に生成する(init 前 span は inert で record が no-op)。`job_id` は job 確定後に `span.record` で後付けする。

## stage 計時の manifest 永続化

`StageRecord.duration_ms: Option<u64>` は `#[serde(default)]` で旧 manifest 互換を維持し、「その stage の最新実行の経過時間(ms)」の意味を持つ(ADR `stage-timing-manifest-persistence`)。resume 再実行は新計測値で上書き(合算しない)、未実行 stage は既存値保持、旧 manifest 由来は `None`。計時は log 有効性と無関係に常時 manifest へ永続化する。`execute` 内 `stage.run(&ctx)` を `info_span!("stage")` で囲み、成否両分岐で `duration_ms` を記録する。`Err` 戻りの失敗 stage も計時対象で、panic unwind のみ適用範囲外(既知の限界)。

## leaf crate の span 計装

`info_span!` の配置は design の span 配置マップどおりに al-locate/al-analysis/al-media/al-viz/al-dataset の 5 crate(al-core/al-math/al-testkit は対象外)。span name は `&'static str` リテラル必須のため family 呼出しは汎用ヘルパではなく個別 `info_span!` で包む。全同期 fn のため `let _enter = span.enter()` または `span.in_scope(..)`。`FmtSpan::CLOSE` は「内側 event を持たない leaf span が sink に残らない」不備として発覚・追加された —— metadata-only 語彙(job_id・stage・duration_ms・len・sha256・input_id・method)で raw PCM や巨大バッファを載せない。

## プロファイリング基盤

L1 計測は al-math criterion smoke bench(公開 dispatch 経路の `ncc`/`spectral_envelope`、固定サイズ)。L3 計測は `docs/guides/profiling-samply.md` の手順書で、root Cargo.toml に `[profile.release] debug = true` を追加(samply のシンボル解決に debug info 必須、codegen/最適化は不変)。criterion `=0.8.2` は「crates.io 公開から ≥7 日経過」ルールで pin した。

## 運用上の教訓

- init 前 span は inert なので、`init_logging` を dispatch span より先に呼ぶ順序規約を守る。
- EnvFilter は lenient で `bogus!!!` も valid target directive として受理する。fail-fast 検証には実際に失敗する `=info`(target 空)・`info=bogus`(未知 level)を使う。
- `stderr=json` 時に tracing 行と `fail()` plaintext が混在し stderr 全体は機械 parse 不可(既知制約、JSON 機械処理は file sink の責務)。
- 同日 log file への複数プロセス append の行原子性は tracing-appender 依存(Windows で retention 削除が失敗し得る既知制約)。

## ruviform 委譲移行(AL#138)

共通 logging 実装は `ruviform-observability::logging` へ委譲した。stderr sink は `build_logging_layers` へ委譲、global 登録は host 側 `registry().with(layers).try_init()` で 1 回のみ(ruviform adapter は `set_global_default` を呼ばない契約のため)。file sink は `al.YYYY-MM-DD.log` 命名契約のため暫定 local 構築を維持する —— ruviform `SinkDestination::RollingFile` が suffix を公開しない(ruviform#334)ためで、理由・解消条件・追跡 Issue を明記して隔離した。stderr span close は `span_events: SpanEvents::CLOSE` で復旧(ruviform#335 の消費者側修復、`SinkConfig` literal に必須 field を追加)。

関連 Issue/PR: AL#127、AL#138、ruviform#276、ruviform#289、ruviform#334、ruviform#335、ruviform PR #348。
