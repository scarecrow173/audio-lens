---
id: "TASK-034EO4z7oZadAFuMojWjLO"
type: "task"
status: "done"
title: "機械的 caller クリーンアップ: 事後計測・Implementation Record 記録・最終検証"
created: "2026-10-12"
updated: "2026-10-12"
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
  implements:
    - "docs/plans/mechanical-caller-cleanup.md"
  implemented-by:
    - "docs/impl/ir/mech-cleanup-measurement-ir-closeout.md"
  depends-on:
    - "docs/plans/mechanical-caller-cleanup.md"
    - "docs/tasks/mech-cleanup-a16-row-major-fast-paths.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/mechanical-caller-cleanup.md"
    - "docs/specs/mechanical-caller-cleanup.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/mechanical-caller-cleanup.md"
  references: []
  defers: []
  deferred-by: []
---

# 機械的 caller クリーンアップ: 事後計測・Implementation Record 記録・最終検証

## Work

plan T7(post-measurement + closeout)。T1–T6 が land した
状態で全ベンチを再実行し、T1 の baseline と比較して、
spec プロセス契約の「before/after 実測を Implementation
Record に記録」を完了させ、workspace 全体の最終検証を
行う。

- 全 criterion bench を `--baseline <T1 で保存した名>` で
  再実行: `benches/band_correlation.rs`(A6 delta)・
  `benches/csd_accum.rs`(A7/A8 delta)・
  `benches/quant_step.rs`(A12 delta)・
  `benches/spectral_pass.rs`(A9/A16a/A16b delta)。
  既存 `stft`/`conv`・`al-math` lstsq も再計測し非対象
  経路の回帰がないことを確認
- `cargo run --example bench_reference --release -p
  al-analysis`・`cargo run --example bench_dsp
  --release -p al-locate` を再実行し、
  `target/bench-baseline/` の T1 出力と比較(Instant
  ベンチは複数回中央値)
- al-dataset `tests/integration.rs` の Instant
  micro-timing(A16d)と al-media の採用計測方式
  (Instant 区間または code inspection + manifest
  `duration_ms`、T6 で確定)の after 値を記録
- stage 級計時証拠として manifest 永続化済み
  `duration_ms` の変動を併記(代表的な解析経路があれば)
- 回帰判定: ノイズ幅(目安 ±5%)を超える回帰がない
  こと。超過があれば原因を切り分け、正当な理由が
  なければ該当 opt の revert を検討(plan Rollback
  Strategy)
- `docs/impl/ir/` の Implementation Record を完成
  させる: baseline 実測値・各 opt 後の delta・fixture
  定義・baseline 名・使用した criterion/Instant
  コマンド・項目ごとの計測方式・回帰判定結果を記録
- git 履歴の最終確認: char コミットが A6-opt・
  A7A8-opt・A9A12-opt・A15-opt・A16-opt のすべてに
  先行していること
- 最終 workspace 検証: `cargo test --workspace`
  (golden 含む)・`cargo clippy`・`cargo fmt --check`・
  `cargo tree` 依存検査・`audit_docs`・IR
  `audit_impl_record` が clean

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/mechanical-caller-cleanup.md`(T7・
  Verification matrix)
- `docs/specs/mechanical-caller-cleanup.md`(Acceptance
  Criteria — bench delta 記録・±5% 回帰 bound・最終検証
  コマンド・audit)
- `docs/designs/mechanical-caller-cleanup.md`(テスト計画
  ゲート 7–8・Verification Notes)

## Dependencies

- `docs/tasks/mech-cleanup-a16-row-major-fast-paths.md`
  (T6 完了 = 全 opt 適用済みであること)

## Affected Files

- `docs/impl/ir/<record>.md`(before/after 実測の確定
  記録)
- `target/criterion/`・`target/bench-baseline/`(計測
  artifact、gitignore 済み)

## Done When

- [ ] 全 criterion bench(band_correlation・csd_accum・
  quant_step・spectral_pass、および既存 stft・conv・
  lstsq)が baseline 名付きで再実行され、T1 baseline
  との delta が IR に記録されている
- [ ] `bench_reference`・`bench_dsp` の再計測が
  `target/bench-baseline/` 出力と比較され delta が IR
  に記録されている
- [ ] al-dataset Instant micro-timing・al-media 採用
  計測方式の before/after が IR に記録され、al-media
  で fallback(code inspection + 回帰ゲート + manifest
  `duration_ms`)を採用した場合はその旨が明記されて
  いる
- [ ] 全計測点で ±5% を超える回帰がない、または超過分
  の原因・対応が IR に記録されている
- [ ] git 履歴で char コミットが全 opt コミットに先行
  していることが確認できる
- [ ] workspace 検証一式が green で golden テストが
  pass し、`audit_docs`・`audit_impl_record` が clean
- [ ] spec Acceptance Criteria の全項目が充足確認され、
  充足根拠(コミット・計測値・テスト名)が IR に記録
  されている

## Verification

- [ ] `mise exec -- cargo bench -p al-analysis --
  --baseline <baseline-name>` で 4 腕の比較が実行できる
- [ ] `mise exec -- cargo run --example bench_reference
  --release -p al-analysis`・`mise exec -- cargo run
  --example bench_dsp --release -p al-locate` が完走し
  baseline との差分を確認済み
- [ ] `mise exec -- cargo test --workspace` が green
  (golden 含む)
- [ ] `mise exec -- cargo clippy --workspace --all-targets
  -- -D warnings`・`mise exec -- cargo fmt --all --
  --check` が green
- [ ] `mise exec -- cargo tree -p al-analysis --depth 1`
  の直接依存に al-math が現れない
- [ ] `git log --oneline` で char コミットが各 opt
  コミットに先行している(目視)
- [ ] `docs/impl/ir/` のレコードに before/after 実測・
  回帰判定・baseline 名・計測方式が記録されている
  (目視)
- [ ] `audit_docs`・IR `audit_impl_record` が clean
  (spec AC の audit 要件)

## Out of Scope

- 新たな最適化・リファクタ(回帰が見つかった場合の
  修正は該当 opt タスクへ戻すか revert で対応)
- criterion ベンチの CI 回帰ゲート整備(別途、CI 基盤
  整備時)
