# verification — AVI strh declaration bounded exact proof

## 環境と正本

- Host: Windows 11 / Git Bash / mise-managed Rust toolchain。
- Kani lane: runner 経由の WSL Ubuntu。`verdict.json` の `env` はすべて `Ubuntu`。
- `cargo` 直接実行は canonical 扱いしない。Rust verification は mise task、runner syntax check は `mise exec` 経由。
- 本 slice では legacy proof を含む全件 Kani lane は、既存 timeout が確定しているため必須検証にしない。selective group を canonical regression として使う。

## canonical verification

| コマンド | OS/環境 | exit | verdict / 結果 | 証跡 |
| --- | --- | ---: | --- | --- |
| `mise run fmt` | Windows host | 0 | pass | actions.jsonl |
| `mise run check al-media` | Windows host | 0 | pass | actions.jsonl |
| `mise run lint al-media` | Windows host | 0 | pass | actions.jsonl |
| `mise run test al-media` | Windows host | 0 | pass。unit/integration/golden を含む al-media 全件 pass | actions.jsonl |
| `mise run test-formal` | Windows host | 0 | pass。`formal_containers` 7/7、`formal_wav` 7/7 | actions.jsonl |
| `AL_KANI_HARNESS_GROUP="avi-strh-declaration" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-strh-declaration-proof/kani mise run test-formal-kani` | WSL Ubuntu | 0 | lane verdict `pass`。`avi_strh_declaration_bounded_exact` が `VERIFICATION:- SUCCESSFUL` | `kani/verdict.json`, `kani/kani-avi_strh_declaration_bounded_exact.txt` |
| `AL_KANI_HARNESS_GROUP="avi-stream-kind" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-strh-declaration-proof/kani-avi-stream-kind-regression mise run test-formal-kani` | WSL Ubuntu | 0 | lane verdict `pass`。1/1 proof successful | `kani-avi-stream-kind-regression/` |
| `AL_KANI_HARNESS_GROUP="avi-movi-scan" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-strh-declaration-proof/kani-avi-movi-scan-regression mise run test-formal-kani` | WSL Ubuntu | 0 | lane verdict `pass`。1/1 proof successful | `kani-avi-movi-scan-regression/` |
| `AL_KANI_HARNESS_GROUP="avi-movi-composition" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-strh-declaration-proof/kani-avi-movi-composition-regression mise run test-formal-kani` | WSL Ubuntu | 0 | lane verdict `pass`。1/1 proof successful | `kani-avi-movi-composition-regression/` |
| `AL_KANI_HARNESS_GROUP="avi-movi-tag" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-strh-declaration-proof/kani-avi-movi-tag-regression mise run test-formal-kani` | WSL Ubuntu | 0 | lane verdict `pass`。1/1 proof successful | `kani-avi-movi-tag-regression/` |
| `AL_KANI_HARNESS_GROUP="mp4-avi-boundary" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-strh-declaration-proof/kani-mp4-avi-boundary-regression mise run test-formal-kani` | WSL Ubuntu | 0 | lane verdict `pass`。2/2 proof successful | `kani-mp4-avi-boundary-regression/` |
| `AL_KANI_HARNESS_GROUP="mp4-scan" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-strh-declaration-proof/kani-mp4-scan-regression mise run test-formal-kani` | WSL Ubuntu | 0 | lane verdict `pass`。1/1 proof successful | `kani-mp4-scan-regression/` |
| `AL_KANI_HARNESS_GROUP="wav-chunk-scan" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-strh-declaration-proof/kani-wav-chunk-scan-regression mise run test-formal-kani` | WSL Ubuntu | 0 | lane verdict `pass`。2/2 proof successful | `kani-wav-chunk-scan-regression/` |
| `AL_KANI_HARNESS_GROUP="wav-range" AL_KANI_TIMEOUT_MS=300000 AL_KANI_LOG_DIR=evidence/al274-avi-strh-declaration-proof/kani-wav-range-regression mise run test-formal-kani` | WSL Ubuntu | 0 | lane verdict `pass`。2/2 proof successful | `kani-wav-range-regression/` |
| `mise exec -- prek run --all-files` | Windows host | 0 | pass。auto-fix による lease 外差分なし | actions.jsonl |

## Able to Fail / mutation evidence

一時 mutation として `scale` decode の input byte を正しい offset 20..24 から offset 21..25 へ 1 byte ずらした。`AL_KANI_LOG_DIR=evidence/al274-avi-strh-declaration-proof/mutation-strh-declaration-kani` の avi-strh-declaration canonical lane は exit 1 / lane verdict `fail` で、`avi_strh_declaration_bounded_exact` の `fields.scale` assertion が `VERIFICATION:- FAILED` になった。mutation は検出後に完全 revert し、上表の final lane で再 Pass を確認した。mutation lane は意図的失敗であり final verification ではない。

## Kani cost summary

| lane / harness | verdict | elapsed ms | max RSS KiB |
| --- | --- | ---: | ---: |
| avi-strh-declaration / `avi_strh_declaration_bounded_exact` | pass | 6379 | 249424 |
| mutation / same harness | fail（検出成功） | 6357 | 249824 |
| avi-stream-kind / `avi_stream_kind_classification_bounded_exact` | pass | 14554 | 499888 |
| avi-movi-scan / `avi_movi_scan_collection_bounded_exact` | pass | 8357 | 313780 |
| avi-movi-composition / `avi_movi_entry_decision_exact` | pass | 5075 | 304768 |
| avi-movi-tag / `avi_movi_packet_tag_exact` | pass | 5045 | 297792 |
| mp4-avi-boundary / `mp4_box_header_step_exact` | pass | 9725 | 306632 |
| mp4-avi-boundary / `avi_chunk_step_exact` | pass | 6210 | 304492 |
| mp4-scan / `mp4_box_scan_progress_bounded_exact` | pass | 19950 | 309120 |
| wav-chunk-scan / `wav_chunk_boundary_step_exact` | pass | 6703 | 302340 |
| wav-chunk-scan / `wav_chunk_scan_progress_bounded_exact` | pass | 7666 | 314852 |
| wav-range / `wav_range_plan_valid_ranges_exact` | pass | 10801 | 298688 |
| wav-range / `wav_range_plan_invalid_ranges_error` | pass | 4708 | 295984 |

すべて runner の `/usr/bin/time -v` 計測であり、max RSS は measured。strh declaration final lane は 10 分以内・4GiB 未満の設計予算内である。

## 限定的な旧新 cost 比較

read-only 引用した旧 whole-call/container proof の cost は次の通り。`avi_writer_output_decodes` は 1200s cap trial で elapsed 1267s / peak 3.9GiB、canonical 1500s では timeout。`mutated_avi_never_panics` は 1200s cap trial で elapsed 1266s / peak 2.9GiB、canonical 1500s では timeout。`truncated_avi_never_panics` は 900s cap trial で elapsed 948s / peak 2.9GiB、canonical 1500s では timeout。これらは writer fixture、container decode、allocation、formatting、parser-wide branch を含み、formal pass を出していない。

新しい strh declaration proof は fixed-prefix extraction/validation の narrow local proof であり、6379ms / 249424KiB で Pass した。既存 local AVI proof regression は stream kind 14554ms / 499888KiB、movi scan 8357ms / 313780KiB、movi composition 5075ms / 304768KiB、movi tag 5045ms / 297792KiB。低コストは保証範囲の狭さの結果であり、whole-call proof と同等の assurance と読み替えない。完全な旧新 cost matrix は後続 slice に残す。

## 保証範囲と未証明領域

証明したのは、full `usize` length scalar と fixed symbolic 48 byte prefix に対する strh declaration field extraction/validation の local total correctness である。length gate の exact complement、`fcc_type` byte 順序保持、offset 20/24/28/32/44 の LE u32 decode、field view の写像、局所算術安全性を production helper 接続で検査した。

この proof は任意長 body slice composition、実 `Vec<StreamHead>`、`strl` ordering/missing `strh` declaration correspondence、`strf` parsing、WAVEFORMATEX/BITMAPINFOHEADER validation、timing accumulation、PCM/video decode、partial ingest、公開 API 全体 no-panic、I/O、allocation、diagnostic formatting、MP4、writer round-trip を証明しない。proof Pass を AVI-STREAM-002 全体または AVI-TIME-004 の機械証明と読み替えない。

## 補助証拠（canonical = mise）

- `mise exec -- node --check tools/run-formal-kani.mjs`: exit 0。
- peak memory: runner の `/usr/bin/time -v` で測定済み。未測定 proof はない。
- 最終 `git fetch origin main` 時点の `origin/main` は `a5770e48cec99b09da9765df0a8d1aa2f6af614d` のままで、branch base から新規差分はなかった。
- lease 照合: `git diff --name-only` は `.agents/skills/verify-audio-lens/SKILL.md`, `crates/al-media/src/avi.rs`, `docs/designs/al274-formal-verification-boundary.md`, `docs/specs/formal-verification.md`, `mise.toml`, `tools/run-formal-kani.mjs` に限定され、新規 evidence は `evidence/al274-avi-strh-declaration-proof/` のみ。docs index、過去 evidence、Cargo 関係、`crates/al-media/src/wav.rs`、`crates/al-media/src/mp4.rs`、`crates/al-media/src/header.rs`、`crates/al-media/tests/formal_containers.rs` に diff はない。
