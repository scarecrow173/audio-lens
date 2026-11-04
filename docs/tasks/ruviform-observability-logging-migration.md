---
id: "TASK-kcnk4zndetw2GNXXC8OfrL"
type: "task"
status: "in-progress"
title: "al-observability logging を ruviform-observability::logging へ委譲 (AL#138)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/138"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/ruviform-observability-logging-migration.md"
    - "../designs/ruviform-observability-logging-migration.md"
  implemented-by: []
  depends-on:
    - "../designs/ruviform-observability-logging-migration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "observability-cli-wiring.md"
    - "observability-crate.md"
    - "https://github.com/scarecrow173/ruviform/issues/334"
    - "https://github.com/scarecrow173/ruviform/issues/335"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/ruviform-observability-logging-migration.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ruviform-observability-logging-migration.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/276"
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/289"
  defers: []
  deferred-by: []
---

# al-observability logging を ruviform-observability::logging へ委譲 (AL#138)

## Work

Issue [#138](https://github.com/scarecrow173/audio-lens/issues/138)
(AL-OBS-01)を、
`docs/designs/ruviform-observability-logging-migration.md` の
設計どおり実施する。ruviform resolved SHA は `8fb665f…`
(追従済み)で同一 SHA から `ruviform-observability` を解決する。

1. **golden test 拡充**(実装順序 a): 現行実装で PASS を
   確認してから進める。追加ケース:
   - retention(`al.*.log` 9 file 事前配置 → 最大 7 件)
   - file init 失敗 degrade(`<base_dir>/logs` を通常 file
     で占拠 → warn + job 成功)
   - format E2E(`stderr=json` で stderr が JSON parse 可能、
     `file=text` で file が text 形式)
   - flag>env 全体置換の E2E(壊れた env + 有効 flag)、
     `AL_LOG` 空文字 OFF
   - file sink の span close event 観測(暫定 leaf span 担保)
   - `init` 2回目が `AlreadyInitialized`(unit test)
2. **依存追加**: `[workspace.dependencies]` に
   `ruviform-observability`(feature `logging`)。
   `al-observability` が参照。`cargo check` で lockfile 再解決
   (手編集しない)。
3. **`init` 委譲**(実装順序 b): stderr sink を
   `build_logging_layers`(`SinkConfig{Stderr, format,
   filter, Required}`)へ委譲。file sink は `al` + `log`
   suffix・daily・max 7・`non_blocking`・`FmtSpan::CLOSE`
   の local layer を暫定維持(ruviform #334 解消条件付き)。
   `registry().with(layers).try_init()` で単一登録、
   `LogGuard` が `WriterGuard` + `WorkerGuard` を保持。
   `InitError` に `Build` variant を追加。
4. **検証**(実装順序 d): verify-ci、golden parity、
   logging ON/OFF の解析結果同一性、`cargo tree` 依存境界、
   `check-ruviform-lock`/`check-ruviform-rev`、samply 手順の
   保持確認。Source→Snapshot(実装順序 c)は OBS-07 #289
   未マージかつ登録対象の Counter 実体が本 Repo に存在しない
   ため対象外であることを IR に記録する。

## Classification

- `normal-plan-task`

## Upstream Documents

- [docs/specs/ruviform-observability-logging-migration.md](../specs/ruviform-observability-logging-migration.md)(Issue #138)
- [docs/designs/ruviform-observability-logging-migration.md](../designs/ruviform-observability-logging-migration.md)(approved・gap/暫定方針)
- [docs/plans/ruviform-observability-logging-migration.md](../plans/ruviform-observability-logging-migration.md)(approved)
- [docs/test-specs/ruviform-observability-logging-migration.md](../test-specs/ruviform-observability-logging-migration.md)(検証基準)
- [docs/adr/logging-sink-format-activation-contract.md](../adr/logging-sink-format-activation-contract.md)(維持する互換契約の正本)
- [docs/specs/observability-foundation.md](../specs/observability-foundation.md)(leaf span 観測要件)

## Dependencies

- `ruviform-observability` @ ruviform main `8fb665f…`
  (feature `logging`。Ruviform #276 で導入済み)
- 暫定 file sink: 既存 `tracing-appender`/`tracing-subscriber`
  (新規外部 dep なし)
- 追跡: Ruviform #334(filename_suffix)/ #335(FmtSpan) —
  解消後に暫定 file sink と stderr CLOSE を復旧する

## Affected Files

- `Cargo.toml` / `Cargo.lock`
- `crates/al-observability/Cargo.toml` / `src/lib.rs`
- `crates/al-cli/src/main.rs` / `tests/observability.rs`
- `docs/specs|designs|plans|test-specs|tasks/ruviform-observability-logging-migration.md`
- `docs/impl/ir/ruviform-observability-logging-al-138.md`

## Done When

- [x] `al-observability` の stderr sink 構築が ruviform
      `build_logging_layers` へ委譲され、file sink が暫定
      local 構築(#334)と併せて単一 global subscriber に登録
      される
- [x] 拡充した golden test 群が現行実装・移行後実装の双方で
      PASS する(ただし stderr span close は既知 regression)
- [x] `resolve_activation`/`LogConfig` の公開契約(error
      文字列・exit code・有効化 semantics)が変わっていない。
      注: lint 抑制禁止規約に従い `#[allow(large_enum_
      variant)]` を撤去するため `Enabled` payload を
      `Box<LogConfig>` 化し、al-cli 側の呼出しは
      `init(*config)` に追随(観測される挙動は不変)
- [x] `cargo tree` で tracing-subscriber/tracing-appender が
      al-observability 経由のみで現れる
- [x] `mise run verify-ci` と `check-ruviform-lock`/
      `check-ruviform-rev` が green
- [x] 暫定回避策(file sink local・stderr CLOSE 欠落)が
      理由・解消条件・追跡 Issue 付きで設計書/コードに明記
      され、IR に検証結果と対象外判断(c)が記録される

## Verification

- [x] `mise run fmt` / `check` / `lint` / `test` /
      `verify-ci`
- [x] `mise run check-ruviform-lock` / `check-ruviform-rev` /
      `check-cli-headless` / `verify-toolchain`
- [x] `cargo tree` による依存境界の確認
- [x] `run` の manifest/artifact が logging ON/OFF で同一
      (生成 ID・timestamp を除き一致を実測。補助検証)
