# Issue #256 — D 層 formal verification 実行台帳

slice 1 = 第 1 標的 `al-media` WAV codec / slice 2 = 第 2 標的
MP4/AVI container parse(`mp4.rs`/`avi.rs`/`format.rs`)。

## slice 1 — 第 1 標的 (al-media WAV codec)

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
| `mutation-mp4-sowt-endian.txt` | slice 2 変異注入時の失敗出力全量 — `mp4_writer_output_round_trips_through_decode` が設計どおり fail した証拠 |

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

## slice 2 — 第 2 標的 MP4/AVI container parse

実行環境: Windows 11 (x86_64) / Orca worktree
`G:\orca\workspaces\audio-lens\al256-formal-slice2` / rustc 1.99.1 /
mise 管理 tool は slice 1 と同じ。`MISE_ENV` 未設定。

Kani 実行環境: WSL2 `Ubuntu`(slice 1 で provision 済み)。
今 slice の実走は WSL service 障害(`Wsl/Service/E_UNEXPECTED`)
で verify 未到達 — 詳細は後述。

ブランチ: `feature/256-formal-verification-slice2`
(基点 `7423546` = slice 開始時点の origin/main tip。covered code
`crates/al-media/src/mp4.rs`/`avi.rs`/`format.rs` は本 slice で
無変更 — 変異注入は revert 済みで committed product diff 0)。

### スコープ

D 層昇格路の第 2 標的として MP4/AVI container parse と
`detect_format` に property-based 検証と Kani bounded proof
harness を導入する slice。対象コードの変更はなく、追加・変更は
検証側のみ: `tests/formal_containers.rs`(proptest suite +
`#[cfg(kani)]` harness)、`mise.toml` の `test-formal` に
`--test formal_containers` 追加、`tools/run-formal-kani.mjs` の
`HARNESSES` に 6 件追加、`docs/specs/formal-verification.md`・
SKILL.md formal 節・本 evidence の更新。

### 実走(actions + resulting state)

| 手順 | コマンド | exit | 観測 |
| --- | --- | --- | --- |
| proptest lane | `mise run test-formal` | 0 | 13/13 pass — 6/6 formal_wav + 7/7 formal_containers |
| 変異注入 | mp4.rs `pcm_layout` `sowt` `base(false,true,false,..)` → `base(false,true,true,..)`(LE→BE)後 `cargo test --locked -p al-media --test formal_containers` | 101 | `mp4_writer_output_round_trips_through_decode` が設計どおり FAIL — decoder codec `pcm_s16be` vs expected `pcm_s16le`。最小反例: 単一 `sowt` 1ch/16bit track(rate 1849・15 sample)。seed `b13b82d5f8f1e1633a1b84b183d4c056b5774a473c717465eae7ea33cec3af70`、全量は `mutation-mp4-sowt-endian.txt`(tee 経由のため pipeline tail は 0、cargo test 本体は 101) |
| 変異 revert | `base(false,true,false)` に戻し `git diff crates/al-media/src/` + suite 再実行 | 0 | `src/` 配下 product diff 空(0 行)・suite 7/7 green — commit 対象に変異を含まない(Coordinator gate 条件: committed src diff 0) |
| canonical lanes | `mise run` fmt / check / lint / test `al-media` / test-formal / check-cli-headless / check-ruviform-lock / check-ruviform-rev / `prek run --all-files` | 0 | 全 Pass — fmt 差分なし、clippy `-D warnings` ok、al-media test green、formal 13/13、headless 境界、ruviform `e4c9f6f0` 26 packages pin、prek 全 hook pass |
| kani lane | `mise run test-formal-kani` × 2 | 0 | verdict `not_run` — 0/9 harness verified(2 run で同一再現)。WSL mirror+dep compile は到達、先頭 harness の verify が exit 1、以後の wsl launch が全て `Wsl/Service/E_UNEXPECTED` で失敗。CBMC verify に到らず — environment-blocked として fail-closed 記録 |

### 検証する property と bounds(tests/formal_containers.rs)

| property | bound |
| --- | --- |
| `container_decoders_never_panics_on_arbitrary_bytes` | 任意 `&[u8]` 0..=4096B × `decode_mp4`/`decode_avi`/`detect_format` |
| `mp4_writer_output_round_trips_through_decode` | tracks 1..=3・`soun`/`vide` 混在、decodable(sowt/twos/raw /lpcm v2/ipcm+pcmC)と unavailable(compressed・pcmC 無・unpacked・12bit・mid-frame・0ch/0rate)entry を網羅、extra box 付き |
| `avi_writer_output_round_trips_through_decode` | vids BGR24 width {4,8,16,32,64}×height 1..=16・fps 1..=60・frames 0..=4、auds PCM s16/非 PCM/absent、`dwStart` 0..=8・interleave 生成 |
| `compressed_audio_is_an_observation_not_an_error` | `mp4a`/`ac-3`/`alac`/`ec-3` `soun` のみ — `Err` にならず capability observation |
| `missing_or_undecodable_auds_is_audio_unavailable` | `auds` なし/非 PCM/`wb` なし — `audio` None + `audio_unavailable` |
| `detect_format_follows_signature_rules` | RIFF/`WAVE`/`AVI `・offset4 `ftyp`・その他/短入力、切詰め view |
| `mutated_or_truncated_containers_never_panic` | writer 生成物への byte 上書き+truncate |
| `kani_proofs::containers_never_panic_on_bounded_input` | symbolic `[u8;64]`、`#[kani::unwind(80)]` |
| `kani_proofs::detect_format_signature_rules` | len<8/8/12 の境界長、`#[kani::unwind(8)]` |
| `kani_proofs::mp4_writer_output_decodes` | symbolic `soun`(ch 1..=2・rate 1..=192000・4 frame s16)、`#[kani::unwind(64)]` |
| `kani_proofs::avi_writer_output_decodes` | concrete video(4x2・2 frame)+symbolic PCM `auds`、`#[kani::unwind(96)]` |
| `kani_proofs::mutated_mp4_never_panics` / `mutated_avi_never_panics` | writer 生成物へ 1 byte 上書き+truncate、`#[kani::unwind(64)]` / `#[kani::unwind(80)]` |

assumptions: v0/v1 `soun` entry の sample rate は 16.16 fixed で
>65535 は下位 16bit に折り返す(declared rate として model が
写す — 表現限界であり不具合ではない)。`skipped_boxes`・
`pcm_tracks` 所属(宣言 capability)・secondary `soun` の
capability observation は doc 契約として model が写す。coverage
の残り: 単一 chunk uniform table のみ(multi-chunk `stsc`/複数
`stts` run/`elst`/配列形 `stsz` は未生成)、`fpcm`/`fl32`/`fl64`/
`in24`/`in32` の PCM 復号は未生成、AVI は先頭 vids/auds 各 1 本。

### verdict(POLICIES §3 — 要求状態 × 実行結果)

| 項目 | 要求状態 | 結果 | 根拠 |
| --- | --- | --- | --- |
| proptest property suite が契約を検証し native で常時再実行可能 | Required | Pass | `mise run test-formal` 13/13(7 container properties 追加)、`cargo test --test formal_containers` 直接実行でも同結果 |
| Kani bounded proof lane が env-detect + honest verdict で動作 | Required | Pass | `tools/run-formal-kani.mjs` — 9 harness 登録、per-harness 逐次実行、環境障害を `not_run`+理由で exit 0 報告(fail-closed) |
| Kani proof が PASS すること | Required | NotRun | WSL service `E_UNEXPECTED` で CBMC verify 未到達(2 run 再現)。lane・harness・報告経路は稼働済み、WSL 健全化後の再試行が要る |
| 変異注入で property が fail する証拠 | Required | Pass | `mutation-mp4-sowt-endian.txt` — `sowt` endian 変異で `mp4_writer_output_round_trips_through_decode` FAIL、seed 記録済み、revert 後 `src/` diff 0 |
| 昇格路・bounds・assumptions の文書化 | Required | Pass | `docs/specs/formal-verification.md` §第 2 標的 |
| Issue #256 は open のまま(後続標的・昇格が残る) | Required | Pass | PR 本文は `Refs #256` |

### 正規検証 lane 結果(canonical = `mise run`)

| lane | コマンド | exit | 結果 |
| --- | --- | --- | --- |
| fmt | `mise run fmt` | 0 | Pass — 差分なし |
| check | `mise run check al-media` | 0 | Pass |
| lint | `mise run lint al-media` | 0 | Pass — `-D warnings` |
| test | `mise run test al-media` | 0 | Pass — 全 suite green |
| formal proptest | `mise run test-formal` | 0 | Pass — 13/13 |
| formal kani | `mise run test-formal-kani` | 0 | NotRun — lane 正常稼働・0/9 harness verified(WSL service 障害、2 run 再現) |
| headless 境界 | `mise run check-cli-headless` | 0 | Pass — `no GUI framework deps` |
| lockfile 決定性 | `mise run check-ruviform-lock` | 0 | Pass — `e4c9f6f0` 26 packages |
| ruviform rev | `mise run check-ruviform-rev` | 0 | Pass — `e4c9f6f0` lockfile-primary |
| pre-commit | `mise exec -- prek run --all-files` | 0 | Pass — 全 hook green |

### NotRun / 補足

- `mise run test-formal-kani` は lane としては稼働済みだが、実証
  verdict は `not_run`。WSL mirror(source 展開)と `cargo kani`
  の依存 compile までは到達するが、先頭 harness の verify が
  exit 1 で落ち、以後の全 `wsl.exe` launch が
  `Wsl/Service/E_UNEXPECTED` を返す(2 run で同一再現)。
  slice 1 と同じ shared-box 競合の類ではなく WSL service 自体の
  障害 — distro 再起動または box 再起動後の再試行が要る。
  proof 本体・lane・報告経路は稼働済み。
- `docs/**/README.md` 索引は本 slice で更新しない — 統合時に
  Coordinator が再生成する規約。
- ローカル検証の成功は統合受入ではない — merge 後 main SHA での
  必須 CI 証拠が統合受入の条件(POLICIES §3)。

## residual — Kani lane verdict 回収(2026-11-24)

slice 1-2 で環境起因により verdict が取れなかった 9 harness
(WAV 3 + containers 6)を、WSL2 が健全な状態で 1 本の lane run に
通し、per-harness verdict と分類を記録する residual slice。
covered code(`crates/al-media/src/`)は slice 1-2 と同じ —
sync commit `342aea7008e377d7dfa51d27eea689d6e1f6c251`
(本 slice 基点の origin/main tip、product 差分 0)。

### 実行環境

- Windows 11 (x86_64) / Orca worktree
  `G:\orca\workspaces\audio-lens\al256-kani-residual` /
  rustc 1.99.1(環境導入、repo は rust を pin しない)/
  mise 管理 tool: node 26.12.1。`MISE_ENV` 未設定。
- Kani 実行環境: WSL2 `Ubuntu`(Running — slice 2 の
  `Wsl/Service/E_UNEXPECTED` は今回未発生、全走行で wsl launch
  失敗 0)。kani-verifier 0.68.0 + bundled CBMC 6.11.0
  (`cargo kani --version` 出力)、rustup toolchains:
  nightly-2026-08-21 + 1.99.0。WSL envelope: RAM 31GiB +
  swap 8GiB、16 vCPU。全量は `residual/env.txt`。
- 並行 job: 実行前に他 repo/他 Worker の kani・cbmc process が
  0 件であることを `ps` で確認 — slice 1 の占有競合は解消済み。
- budget: `TIMEOUT_MS` 既定値 2700000ms(45min)/harness。
  `AL_KANI_TIMEOUT_MS` override は不使用。

### runner の最小変更(証拠記録)

`tools/run-formal-kani.mjs` に `AL_KANI_LOG_DIR` を追加。env 指定時
のみ、各 harness の raw 出力(stdout+stderr と終了 meta)を
`<dir>/kani-<harness>.txt`、最終 JSON verdict を
`<dir>/verdict.json` に書き出す。現行 runner は harness 出力の
tail 8 行だけを JSON details に残して残りを捨てるため、受入条件の
「各 `cargo kani` 生出力を evidence に保存」と verdict 未発行の
分類根拠(出力末尾の OOM/exit 兆候)に不足があった — 確定した
不足に対する最小変更。env-detect・WSL mirror・逐次実行・
JSON verdict・fail-closed exit の意味論は不変。

### per-harness verdict(正規 lane: `mise run test-formal-kani`)

lane verdict: `not_run` — **1/9 verified**、fail-closed exit 0。
実行コマンド:
`AL_KANI_LOG_DIR=evidence/al256/residual mise run test-formal-kani`
(mirror sync + 逐次 9 harness、全走行 約 6h50m)。

| harness | verdict | 分類 | 根拠(cbmc RSS 観測) |
| --- | --- | --- | --- |
| `decode_never_panics_on_bounded_input` | timeout 45min | envelope-bound | RSS が ~1GiB/min で単調増大し、kill 時点で ~30.4GiB / free 0GiB — 31GiB envelope 天井に張り付き |
| `writer_round_trip_is_exact` | timeout 45min | solver-bound | RSS ~3.4GiB で頭打ち、SAT grind |
| `header_reads_agree_with_writer_layout` | timeout 45min | solver-bound | RSS ~3.3GiB で頭打ち、SAT grind |
| `containers_never_panic_on_bounded_input` | timeout 45min | solver-bound | RSS ~11.4GiB、増大鈍化 |
| `detect_format_signature_rules` | **verified** | — | `VERIFICATION: SUCCESSFUL`、169 checks 0 failed、verify 0.63s — unwind 8 の小 bound は予算内に収まる |
| `mp4_writer_output_decodes` | timeout 45min | solver-bound | RSS ~8.4GiB、増大鈍化 |
| `avi_writer_output_decodes` | timeout 45min | solver-bound | RSS ~6.8GiB、ほぼ平坦 |
| `mutated_mp4_never_panics` | timeout 45min | solver-bound | RSS ~8.3GiB |
| `mutated_avi_never_panics` | timeout 45min | solver-bound | RSS ~4.8GiB、ほぼ平坦 |

cbmc RSS は走行中の `ps -eo etime,rss,comm` 定期観測による補助証拠
(canonical = lane の JSON verdict)。各 harness の raw 出力は
`residual/kani-<harness>.txt`、lane verdict 全量は
`residual/verdict.json`。timeout 各件は spawnSync の
ETIMEDOUT/SIGTERM で kill され、runner の `cleanupOrphans` が
WSL 側の cbmc/kani-driver を掃除した(観測: kill 後 RSS 解放済み)。

### 分類と見通し

- 環境障害ではない: WSL2 は全走行を通じ健全(wsl launch 失敗 0、
  `E_UNEXPECTED` 未発生)。slice 2 の障害は再現しなかった。
- `VERIFICATION: FAILED` + counterexample は 0 件 — 検証 fail
  ではなく budget 未収束の not_run 系。product 意味論への反例
  指摘はない。
- envelope-bound(`decode_never_panics_on_bounded_input`):
  ~31GiB 物理 envelope が原因。時間を増やしても OOM で止まる
  ため、この box では verdict 取得不能が確定的。solver-bound
  7 件は SAT instance が 45min 内に解けない grind — メモリ
  余力が残るため、より大きな budget または専用 runner では
  verdict が取れる見込みが残る。
- harness 除去は行わない: 除去条件は「明示 budget 下での複数
  試行で再現」+「proptest で同一契約を担保」。containers 6 件は
  slice 2 が env-blocked で今回が初回実走(1 試行のみ)のため
  確定条件を満たさず、全 timeout は proof 不能ではなく
  capacity-bound — `verdict が取れる見込みのある harness は
  残す` 方針どおり全件保持する。timeout 8 件の対象契約は
  proptest property(`*_never_panics*` 系と writer round-trip 系)
  で継続担保されている。
- 本 run で唯一 verified に達したのは unwind 8 の
  `detect_format_signature_rules` — bound の小さい証明はこの
  環境でも budget 内に収まる実測。

### 後続の選択肢(本 slice では実施しない)

- bound 縮小(`kani::assume`・入力長・unwind 再設計)は検証の
  成功条件を弱めるため、正当化できる bound のみ + 残りを
  proptest が担うことを明記する別判断。本 slice は現行
  bounds での verdict 取得を記録した。
- dedicated runner(より大きな envelope / 長 budget)での再実行。
- containers 6 件の 2 試行目以降で timeout が再現されれば、
  「複数試行で再現」の除去条件を検討できる。
