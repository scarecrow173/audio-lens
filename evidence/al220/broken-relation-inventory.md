# Issue #220 broken-relation 棚卸しと修復分類

対象: `route_graph.js --json` が報告した `broken-relation:*` 24 件
(全件 `relations.source` が削除済みパスを指す)。
分類基準は synvelis#519 と同型:
(i) frontmatter 記述の削除 / (ii) 有効参照への置換 /
(iii) 意図的例外として残置(根拠明文化)。

## 分類方針

全 24 件を **(ii) 有効参照への置換** に一律分類。
置換先は削除直前の最終残存 revision
`1824fe169460a0e7314cb73686db638bf1ee9f5e`(`0a1bd24^`)の
GitHub blob URL:
`https://github.com/scarecrow173/audio-lens/blob/1824fe16.../<path>`。

根拠:

- `source` relation は「この文書が分析・根拠とした provenance」を
  記録する。削除済みコードへの provenance を失わないため、
  エントリ削除(i)は採用しない。
- ruviform-numeric 現行ファイルへの置換は、ファイル単位の対応が
  未確定(`correlation.rs`→`corr.rs`、`filter.rs`→`bands.rs` 等は
  API 対応表からの推測。tests/examples/`dsp.rs` は 1:1 対応なし)
  であり、誤った等価性を主張するリスクがあるため不採用。
- 移行記録 doc(`issue-101-ruviform-numeric-migration.md`)への
  retarget は provenance の方向が逆のため不採用。
- pinned commit の内容は不変であり、`scheme:` 始まりの URL は
  resolver が external ref として受理する(実測確認)。
- `1824fe16` は `0a1bd24` の親であり、全 stale path が存続する
  最後の tree state。削除 commit 群(`0a1bd24`・`9db1153`)以降に
  当該 path への変更がないことを `git log` で確認済み。

## 棚卸し表(24 件)

凡例: M = `crates/al-math` 配下、L = `crates/al-locate/src/dsp.rs`。
「置換後」は全件共通で
`https://github.com/scarecrow173/audio-lens/blob/1824fe169460a0e7314cb73686db638bf1ee9f5e/<旧path>`。

### docs/discovery/(20 件)

| # | 文書 | 旧 source path | 分類 |
| --- | --- | --- | --- |
| 1 | al-math-api-fft.md | `../../crates/al-math/src/lib.rs` | (ii) 置換 |
| 2 | al-math-api-fft.md | `../../crates/al-math/src/fft.rs` | (ii) 置換 |
| 3 | al-math-api-fft.md | `../../crates/al-locate/src/dsp.rs` | (ii) 置換 |
| 4 | al-math-api-fft.md | `../../crates/al-math/tests/fft.rs` | (ii) 置換 |
| 5 | al-math-api-fft.md | `../../crates/al-math/tests/correlation_filter.rs` | (ii) 置換 |
| 6 | al-math-api-fft.md | `../../crates/al-math/examples/bench_correlation.rs` | (ii) 置換 |
| 7 | al-math-simd-reduction.md | `../../crates/al-math/src/linalg.rs` | (ii) 置換 |
| 8 | al-math-simd-reduction.md | `../../crates/al-math/src/lib.rs` | (ii) 置換 |
| 9 | al-math-simd-reduction.md | `../../crates/al-locate/src/dsp.rs` | (ii) 置換 |
| 10 | caller-side-optimizations.md | `../../crates/al-locate/src/dsp.rs` | (ii) 置換 |
| 11 | caller-side-optimizations.md | `../../crates/al-math/src/linalg.rs` | (ii) 置換 |
| 12 | caller-side-optimizations.md | `../../crates/al-math/src/fft.rs` | (ii) 置換 |
| 13 | caller-side-optimizations.md | `../../crates/al-math/src/correlation.rs` | (ii) 置換 |
| 14 | caller-side-optimizations.md | `../../crates/al-math/src/filter.rs` | (ii) 置換 |
| 15 | issue-40-locate-pair-plan-cache.md | `../../crates/al-math/src/correlation.rs` | (ii) 置換 |
| 16 | mechanical-caller-cleanup.md | `../../crates/al-math/src/filter.rs` | (ii) 置換 |
| 17 | mechanical-caller-cleanup.md | `../../crates/al-math/src/vec.rs` | (ii) 置換 |
| 18 | observability-foundation-logging-and-profiling.md | `../../crates/al-math/examples/bench_correlation.rs` | (ii) 置換 |
| 19 | spectral-envelope-fft-size.md | `../../crates/al-math/src/filter.rs` | (ii) 置換 |
| 20 | spectral-envelope-fft-size.md | `../../crates/al-math/tests/correlation_filter.rs` | (ii) 置換 |

### docs/specs/(4 件)

| # | 文書 | 旧 source path | 分類 |
| --- | --- | --- | --- |
| 21 | locate-plan-cache.md | `../../crates/al-math/src/correlation.rs` | (ii) 置換 |
| 22 | mechanical-caller-cleanup.md | `../../crates/al-locate/src/dsp.rs` | (ii) 置換 |
| 23 | mechanical-caller-cleanup.md | `../../crates/al-math/src/filter.rs` | (ii) 置換 |
| 24 | mechanical-caller-cleanup.md | `../../crates/al-math/src/vec.rs` | (ii) 置換 |

## 件数内訳

- 合計: 24 件(全件 (ii) 有効参照への置換。(i) 削除 0 件、
  (iii) 意図的例外 0 件)
- `crates/al-math/**`: 20 件(src 15・tests 3・examples 2)
- `crates/al-locate/src/dsp.rs`: 4 件
- 文書別: discovery 7 文書 20 件 + specs 2 文書 4 件

## 消し過ぎ防止の確認

`../../crates/al-locate/` 配下で存続する path
(`examples/bench_dsp.rs`・`src/methods.rs`・`src/prepared.rs` 等)
は置換対象外とし、実際に `al-math-api-fft.md` の
`../../crates/al-locate/examples/bench_dsp.rs` は変更していない
(`git diff` で確認)。

## 検証

- `rg '\.\./\.\./crates/(al-math|al-locate/src/dsp)' docs/` → 0 件
- `route_graph.js --json` → `broken-relation:*` 0 件
- `doc_status.js lint` → `provenance-invalid-source` 0 件
  (discovery/specs とも)

詳細な before/after 実測は `verification.md` を参照。
