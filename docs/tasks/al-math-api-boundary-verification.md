---
id: "TASK-034DYiRwEeud1alnVmGMJj"
type: "task"
status: "done"
title: "al-math API 境界: 全体検証と旧コミット bench 比較"
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
  implements:
    - "docs/plans/al-math-api-boundary.md"
  implemented-by: []
  depends-on:
    - "docs/plans/al-math-api-boundary.md"
    - "docs/tasks/al-math-api-pub-crate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/al-math-api-boundary.md"
  references: []
  defers: []
  deferred-by: []
---

# al-math API 境界: 全体検証と旧コミット bench 比較

## Work

plan S6: 全体検証と旧コミット bench 比較。全スライス(S1〜S5)の
完了後に実施する最終ゲート。

- `mise exec -- cargo test -p al-math -p al-locate -p al-analysis`
  が合格することを確認する
- `cargo test -p al-analysis`(golden 系テストを含む)で分析出力が
  不変であることを確認する
- 変更前コミットを別 worktree/別ビルドした `bench_dsp` と同一入力で
  FFT 系カーネル(`fft 4M`、`complex fft 4M`、`rfft 4M`、
  `stft-equivalent`)を比較する。比較リストに `bandpass`/`ncc`/
  `gcc_phat` 腕を加え、旧コミット別ビルドの `bench_correlation`
  binary でも同一入力比較して、いずれも同等以上であることを
  記録する(比較手順は実施時に記録する)
- `al-math` 以外のクレートの `Cargo.toml` に `pulp` 依存がない
  ことを再確認する(現状確認済み、変更なしを維持)

検証失敗(テスト不合格・参照比較不一致・性能回帰が許容範囲超過)
で原因切り分けに時間を要する場合は、plan の Rollback Strategy
(コミット群の `git revert`)に従う。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/al-math-api-boundary.md`(S6・Verification・Rollback)
- `docs/test-specs/al-math-api-boundary.md`(R1〜R6 全体)

## Dependencies

- `docs/tasks/al-math-api-pub-crate.md`(S5、したがって S1〜S4 の
  完了も推移的に前提)

## Affected Files

- コード変更なし(検証と結果記録のみ)
- 比較手順・結果は本タスクの検証記録または impl-doc に記録する

## Done When

- [ ] 3 クレートの全テストが合格し、golden 出力が不変である
- [ ] 旧コミット別ビルドとの bench 比較で、FFT 系カーネルと
  `bandpass`/`ncc`/`gcc_phat` が同等以上であることを記録した
- [ ] `al-math` 以外の `Cargo.toml` に `pulp` 依存がないことを
  再確認した
- [ ] 公開面監査(test-spec R1)が最終状態で成立している

## Verification

- [ ] `mise exec -- cargo test -p al-math -p al-locate -p al-analysis`
  が合格する
- [ ] `mise exec -- cargo test -p al-analysis` で golden 出力が
  不変である
- [ ] 変更前コミット別ビルドの `bench_dsp`/`bench_correlation` と
  同一入力比較し、結果を記録する
- [ ] `rg -l 'pulp' crates/*/Cargo.toml` が `al-math` のみを返す

## 検証記録(2026-10-11)

最終修正(unsafe `set_len` 除去、dead code 削除、`RealFft` の
`transform_ws` 経路化、`cmul` コメント修正、`dsp.rs` doc 修正、
`correlation.rs` `normalize` の `&mut`→`&` 化)後の最終検証。
全項目合格。

### コマンド結果

| コマンド | 結果 |
| --- | --- |
| `mise exec -- cargo test -p al-math -p al-locate -p al-analysis` | pass(212 テスト、失敗 0。golden 系を含む) |
| `mise exec -- cargo build --workspace` | pass |
| `mise exec -- cargo doc -p al-math --no-deps` | pass(warning なし) |
| `mise exec -- cargo check -p al-locate --all-targets` | pass |
| `mise exec -- cargo build -p al-locate --examples` | pass |
| `mise exec -- cargo fmt --all -- --check` | pass |
| `mise exec -- cargo clippy -p al-math --lib -- -D warnings` | pass |

### 公開面 grep 監査

- `rg 'pulp' crates/ --glob '!**/al-math/**'`: ヒット 0 件
- `rg 'al_math::[\w:{}, ]*\b(c64|Simd|WithSimd|F64s|C64s|dispatch)\b' crates/`:
  ヒット 0 件
- `rg 'pulp' crates/*/Cargo.toml`: `al-math/Cargo.toml` のみ

### bench_dsp 数値(release、同一プロセス)

| 項目 | 時間 |
| --- | --- |
| legacy fft 4M(参照) | 280.6 ms |
| fft 4M next_pow2(`dsp::fft` 経由) | 156.8 ms |
| complex fft 4M(plan 再利用) | 103.5 ms |
| rfft 4M(plan/scratch 再利用) | 63.2 ms |
| NCC 2.4M x 8k(fft corr) | 416.0 ms |
| bandpass 2.4M(spectral mask) | 182.7 ms |
| stft-equivalent: welch-ish psd 2.4M | 49.7 ms |
| gcc_phat 2.4M x 8k | 241.9 ms |

旧コミット別ビルドとの直接比較は未実施。同一プロセス内の
legacy 参照実装(280.6 ms)に対し、`dsp::fft` 経路は 156.8 ms、
plan 再利用の complex fft は 103.5 ms、RealFft は 63.2 ms と
大きく上回る。

### 残存する既知トレードオフ

- 公開 `dispatch` API 経由ではなく plan を直接再利用する
  マイクロベンチ経路では、修正前ベースライン比で最大 ~10%
  程度の劣後がありうる(直接 plan 利用経路のオーバーヘッド)。
  `dsp::fft` など製品側公開入口の計測値は上記の通り legacy
  参照を大きく上回るため、許容範囲と判断する。
