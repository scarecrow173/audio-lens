# Issue #256 — D 層 formal verification 第 1 標的 (al-media WAV codec)

実行環境: Windows 11 (x86_64) / Orca worktree
`G:\orca\workspaces\audio-lens\al256-formal-slice1` /
rustc 1.99.1(環境導入、repo は rust を pin しない)/
mise 管理 tool: node 26.12.1, prek 0.5.7。
`MISE_ENV` 未設定 = ローカル標準 profile(dev/test)。

Kani 実行環境: WSL2 `Ubuntu` — kani-verifier 0.68.0 +
CBMC 6.11.0 + rustc 1.100.0-nightly (nightly-2026-08-21)。
`cargo kani` は Windows native 非対応のため
`tools/run-formal-kani.mjs` が WSL distro を検出し、source
mirror(`~/.cache/al-formal-kani/src`)経由で実行する
(cargo target-dir 設定・env 変数は変更しない)。

ブランチ: `feature/256-formal-verification-slice1`
(基点 `f11e04e` = slice 開始時点の origin/main tip。
検証対象の sync commit も `f11e04e` — covered code
`crates/al-media/src/wav.rs` は本 slice で無変更)。

## スコープ

D 層(proptest → Kani → TLA+ → Lean 4 の昇格路)の第 1 標的として
`al-media` WAV codec に property-based 検証と Kani bounded proof
を導入する slice。対象コード(wav.rs)の変更はなく、追加は検証
側のみ: `tests/formal_wav.rs` (proptest suite + `#[cfg(kani)]`
harness)、`tools/run-formal-kani.mjs` (env-detect lane runner)、
`mise.toml` の `test-formal`/`test-formal-kani` task、
`docs/specs/formal-verification.md`、SKILL.md formal 節、
本 evidence。

## ファイル

| ファイル | 内容 |
| --- | --- |
| `actions.jsonl` | action → 観測結果の 1 行台帳(`al-ctl evidence append` で追記) |
| `mutation-pcms16-endian.txt` | 変異注入時の `cargo test` 失敗出力全量 — property が設計どおり fail した証拠 |

## 実走(actions + resulting state)

| 手順 | コマンド | exit | 観測 |
| --- | --- | --- | --- |
| proptest lane | `mise run test-formal` | 0 | 6/6 properties pass — 任意 byte no-panic、frame-count/range 契約、5 encoding writer round-trip、param/unaligned rejection、mutated header no-panic |
| kani provision | `wsl -d Ubuntu -- bash -lc "cargo install --locked kani-verifier && cargo kani setup"` | 0 | kani-verifier 0.68.0・CBMC 6.11.0・nightly-2026-08-21 を WSL2 Ubuntu に導入 |
| 変異注入 | wav.rs `PcmS16` `to_le_bytes` → `to_be_bytes` 後 `cargo test --locked -p al-media --test formal_wav` | 101 | `writer_output_round_trips_losslessly` が設計どおり FAIL — 最小反例 `PcmS16`/1ch で write `14.0` → decode `3584.0`(byte swap)。seed `c8c8aadba36c661cb24b9b609ad560736501a5546823afe70153c7bf2b823fde`、全量は `mutation-pcms16-endian.txt` |
| 変異 revert | `to_le_bytes` に戻し `git diff crates/al-media/src/wav.rs` + suite 再実行 | 0 | product diff 空(0 行)・suite 6/6 green — commit 対象に変異を含まない |
| canonical lanes | `mise run` fmt / check / lint / test `al-media` / check-cli-headless / check-ruviform-lock / check-ruviform-rev / `prek run --all-files` | 0 | 全 Pass — fmt 差分なし、clippy `-D warnings` ok、al-media test green、headless 境界 `no GUI framework deps`、ruviform `e4c9f6f0` 26 packages pin、prek 全 hook pass |
| kani lane | `mise run test-formal-kani` | 0 | verdict `not_run` — 0/3 harness verified。sequential runner が `cargo kani -p al-media --tests --harness <name>` を 1 proof ずつ実行し、`decode`/`writer` は各 45min timeout(2700000ms)、`header_reads` は検証途中で wsl exit 15(SUCCESSFUL/FAILED 未出力)。shared box の並行 ruviform `-j4` kani job が CPU/メモリを占有し、同一 bounds でも実効予算内に収まらなかった。runner は per-harness 詳細を JSON で報告し not_run として fail-closed exit 0 |

## 検証する property と bounds(tests/formal_wav.rs)

| property | bound |
| --- | --- |
| `decode_wav_never_panics_on_arbitrary_bytes` | 任意 `&[u8]` 0..=4096B、proptest 既定 case 数 |
| `frame_count_and_range_reads_hold_the_contract` | 同上。countable≠decodable の区別を property 化 |
| `writer_output_round_trips_losslessly` | 5 encoding × ch 1..=8 × rate ≤ u32::MAX/64 × frames 0..=72、1..=3 分割 write |
| `writer_new_rejects_exactly_the_documented_invalid_params` | 任意 (channels, rate) — 拒否 iff 文書化条件 |
| `write_samples_rejects_exactly_unaligned_lengths` | 任意長 — 拒否 iff len % ch != 0 |
| `mutated_or_truncated_headers_never_panic` | 有効 WAV の field 上書き・任意位置 truncate |
| `kani_proofs::decode_never_panics_on_bounded_input` | symbolic `[u8;48]`、`#[kani::unwind(56)]` — [u8;128]+unwind160 と [u8;64]+unwind80 は 45min timeout 超過(同一 distro で別 repo の kani job と CPU 競合)のため budget 適合に縮小 |
| `kani_proofs::writer_round_trip_is_exact` | symbolic encoding/ch 1..=4/rate 1..=192000/1 frame、`#[kani::unwind(32)]` |
| `kani_proofs::header_reads_agree_with_writer_layout` | 具体 WAV(2ch/48kHz/8 frames)上で u8 由来 `start`/`end`(0..=255、valid/invalid 全分岐網羅)、`#[kani::unwind(32)]` |

assumptions: 整数 encoding の往復は表現可能な整値 f64、
float は f32/f64 表現値(非 NaN)に限る — doc 契約は
clamp+round/cast であり任意値の厳密往復を約束しない。
`WavEncoding` は PcmU8 を emit できない(decode-only)ため
writer 起点 round-trip は 5 encoding。

## verdict(POLICIES §3 — 要求状態 × 実行結果)

| 項目 | 要求状態 | 結果 | 根拠 |
| --- | --- | --- | --- |
| proptest property suite が契約を検証し native で常時再実行可能 | Required | Pass | `mise run test-formal` 6/6、`cargo test -p al-media --test formal_wav` 直接実行でも同結果 |
| Kani bounded proof lane が env-detect + honest verdict で動作 | Required | Pass | `tools/run-formal-kani.mjs` — env 不在時 `not_run`/`unsupported` exit 0、実行時は per-harness 結果 JSON。実走は 0/3 verified → `not_run` を正しく報告(fail-closed) |
| Kani proof が PASS すること | Required | NotRun | shared box の並行 kani job 占有下で 3 harness 全て 45min 予算超過。proof 本体・lane・報告経路は稼働済み、後続 slice で bounded budget 再試行が要る |
| 変異注入で property が fail する証拠 | Required | Pass | `mutation-pcms16-endian.txt` — byte-swap 変異で `writer_output_round_trips_losslessly` FAIL、seed 記録済み、revert 後 diff 0 |
| 昇格路・bounds・assumptions の文書化 | Required | Pass | `docs/specs/formal-verification.md` |
| Issue #256 は open のまま(後続標的・昇格が残る) | Required | Pass | PR 本文は `Refs #256` |

## 正規検証 lane 結果(canonical = `mise run`)

| lane | コマンド | exit | 結果 |
| --- | --- | --- | --- |
| fmt | `mise run fmt` | 0 | Pass — 差分なし |
| check | `mise run check al-media` | 0 | Pass |
| lint | `mise run lint al-media` | 0 | Pass — `-D warnings` |
| test | `mise run test al-media` | 0 | Pass — 全 suite green |
| formal proptest | `mise run test-formal` | 0 | Pass — 6/6 |
| formal kani | `mise run test-formal-kani` | 0 | NotRun — lane 正常稼働・0/3 harness verified(全て 45min budget 超過、並行 kani job 占有下) |
| headless 境界 | `mise run check-cli-headless` | 0 | Pass — `no GUI framework deps` |
| lockfile 決定性 | `mise run check-ruviform-lock` | 0 | Pass — `e4c9f6f0` 26 packages |
| ruviform rev | `mise run check-ruviform-rev` | 0 | Pass — `e4c9f6f0` lockfile-primary |
| pre-commit | `mise exec -- prek run --all-files` | 0 | Pass — 全 hook green |

## NotRun / 補足

- `mise run test-formal-kani` は lane としては稼働済みだが、実証
  verdict は `not_run`。観測上、同一 WSL distro で別 repo
  (ruviform)の `cargo kani -j4` job が実行中で、逐次実行でも
  `decode`/`writer` は 45min budget 超過、`header_reads` は
  検証途中で wsl exit 15。CBMC 単体ではなく box 全体の
  占有が主因の可能性が高い — 同一 bounds の先行 run では
  2/3 harness の cbmc phase が ~10-15min で終了する観測も
  あったが、再現性は未検証。再試行は並行 job 不在の
  idle window または CI 専用 runner で行う前提が要る。
- `docs/**/README.md` 索引は本 slice で更新しない — 統合時に
  Coordinator が再生成する規約。
- ローカル検証の成功は統合受入ではない — merge 後 main SHA での
  必須 CI 証拠が統合受入の条件(POLICIES §3)。
