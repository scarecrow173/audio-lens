---
id: "TASK-034EJgPpxRSjjhOJVeBUup"
type: "task"
status: "todo"
title: "caller 側最適化: 事後計測・Implementation Record 記録・最終検証"
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
    - "docs/plans/caller-side-optimizations.md"
  implemented-by: []
  depends-on:
    - "docs/plans/caller-side-optimizations.md"
    - "docs/tasks/caller-opt-dsp-conv-overlap-save.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/caller-side-optimizations.md"
    - "docs/specs/caller-side-optimizations.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/caller-side-optimizations.md"
  references: []
  defers: []
  deferred-by: []
---

# caller 側最適化: 事後計測・Implementation Record 記録・最終検証

## Work

plan T5(post-measurement + closeout)。T1–T4 が land した状態で全
ベンチを再実行し、T1 の baseline と比較して、spec プロセス契約の
「before/after 実測を Implementation Record に記録」を完了させ、
workspace 全体の最終検証を行う。

- 全 criterion bench を `--baseline <T1 で保存した名>` で再実行:
  `benches/stft.rs`(A3 delta)・`benches/conv.rs`(`conv_direct`
  baseline vs `dsp::conv`、A1 delta)・`al-math benches/lstsq.rs`
  (`lstsq-rows` baseline vs 列 API、A4 delta)
- `cargo run --example bench_reference --release` を再実行し、
  `target/bench-baseline/` の T1 出力と比較(Instant ベンチは
  複数回中央値)。stft・analyze_pair(fir_model 経路)・
  `prepare_residual`(fit_mix 経路)の回帰有無を判定
- stage 級計時証拠として manifest 永続化済み `duration_ms` の
  変動を併記(代表的な解析経路があれば)
- 回帰判定: ノイズ幅(目安 ±5%)を超える回帰がないこと。超過が
  あれば原因を切り分け、正当な理由(例: `conv` の新規コード
  パスで既知の trade-off)がなければ該当 opt の revert を検討
  (plan Rollback Strategy)
- `docs/impl/ir/` の Implementation Record を完成させる:
  baseline 実測値・各 opt 後の delta・fixture 定義・baseline 名・
  使用した criterion/Instant コマンド・回帰判定結果を記録
- git 履歴の最終確認: 全 characterization コミットが対応する
  opt コミットに先行し、`subtract` zero-fill が A1 適用より先に
  land していること
- 最終 workspace 検証: `cargo test -p al-math -p al-locate -p
  al-analysis`・`cargo test --workspace`(golden 含む)・
  `cargo clippy`・`cargo fmt --check`・`cargo tree` 依存検査

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/caller-side-optimizations.md`(T5・Verification
  matrix)
- `docs/specs/caller-side-optimizations.md`(Acceptance Criteria —
  bench delta 記録・±5% 回帰 bound・最終検証コマンド)
- `docs/designs/caller-side-optimizations.md`(テスト計画 7–8・
  Verification Notes)

## Dependencies

- `docs/tasks/caller-opt-dsp-conv-overlap-save.md`(T4 完了 =
  全 opt 適用済みであること)

## Affected Files

- `docs/impl/ir/<record>.md`(before/after 実測の確定記録)
- `target/criterion/`・`target/bench-baseline/`(計測 artifact、
  gitignore 済み)

## Done When

- [ ] 全 criterion bench(stft・conv・lstsq)が baseline 名付きで
  再実行され、T1 baseline との delta が IR に記録されている
- [ ] `bench_reference` の再計測が `target/bench-baseline/` 出力と
  比較され、stft・analyze_pair・prepare_residual の delta が IR
  に記録されている
- [ ] 全計測点で ±5% を超える回帰がない、または超過分の原因・
  対応が IR に記録されている
- [ ] git 履歴で char → opt の順序と zero-fill の先行 land が
  確認できる
- [ ] workspace 検証一式が green で golden テストが pass する
- [ ] spec Acceptance Criteria の全項目が充足確認され、充足根拠
  (コミット・計測値・テスト名)が IR に記録されている

## Verification

- [ ] `mise exec -- cargo bench -p al-analysis -- --baseline
  <baseline-name>` で stft・conv の比較が実行できる
- [ ] `mise exec -- cargo bench -p al-math --bench lstsq --
  --baseline lstsq-rows` で lstsq の比較が実行できる
- [ ] `mise exec -- cargo run --example bench_reference --release
  -p al-analysis` が完走し baseline との差分を確認済み
- [ ] `mise exec -- cargo test -p al-math -p al-locate -p
  al-analysis` と `mise exec -- cargo test --workspace` が green
- [ ] `mise exec -- cargo clippy --workspace --all-targets -- -D
  warnings`・`mise exec -- cargo fmt --all -- --check` が green
- [ ] `mise exec -- cargo tree -p al-analysis --depth 1` の直接
  依存に al-math が現れない
- [ ] `git log --oneline` で char コミットが各 opt コミットに
  先行している(目視)
- [ ] `docs/impl/ir/` のレコードに before/after 実測・回帰判定・
  baseline 名が記録されている(目視)

## Out of Scope

- 新たな最適化・リファクタ(回帰が見つかった場合の修正は該当
  opt タスクへ戻すか revert で対応)
- criterion ベンチの CI 回帰ゲート整備(別途、CI 基盤整備時)
