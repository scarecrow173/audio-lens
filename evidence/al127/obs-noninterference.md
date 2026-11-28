# AL-OBS-NONINTERFERENCE 検証記録（audio-lens#127 slice）

- 実施日: 2026-11-28
- 対象 Ref/Feature: `feature/127-obs-noninterference-oracle` / AL-OBS-NONINTERFERENCE
- AL base commit: `0e89c27690e7ab0e046d4ba48547969eeed3c213`（`origin/main`、最終確認でも同 SHA）
- AL implementation commit: `55242c6f724fa52d489a9653815e97220a940445`（oracle + 文書チェーン。本 evidence は後続 commit）
- ruviform resolved SHA: `eea9e2090a23e24dd5cd7a0799b25175c91cad9c`（25 lockfile sources、lockfile-primary）
- 環境: Windows 11（`Windows-11-10.0.26300-SP0`、MINGW64 x86_64）、rustc/cargo 1.99.1、mise 2026.11.2 windows-x64
- 判定: **Pass**（Windows local/headless OS-L1 の範囲。統合検証は merge 後の対象 SHA に対する CI 証拠）

## Fixture / pipeline

| 項目 | 値 |
| --- | --- |
| fixture | 48,000 Hz mono PCM s16 WAV、480,000 frame（10 秒） |
| 基底信号 | `al_testkit::signal::noise(480000, 0xA127, 0.62 * 32767.0)` を round/clamp して s16 化 |
| sentinel | sample index 4096 から ASCII `AL127_OBS_SECRET` の little-endian s16 byte列 |
| fixture SHA-256 | `dcad8817615ce1b56c67247e9dbc72fca36e8ba4f2afddaf17a1b5b3dab3eefb` |
| input id | `IN-0000000000000000000001` |
| pipeline | `ingest,cheap_analysis,dataset,export` |
| export inventory | `export/analysis` 以下 33 files（実測 prototype と同一構成。JSON/canonical・binary/raw で比較） |

## 受入条件ごとの verdict

| Ref | 対象 | 期待 | 観測 | verdict |
| --- | --- | --- | --- | --- |
| OBS-NI-1 | 新規 oracle target | `crates/al-cli/tests/obs_noninterference.rs` が自動検出され、既存 observability test・Cargo manifest/lock を変更しない | 同ファイルを新設。`Cargo.toml`/`Cargo.lock` 差分なし。既存 `observability.rs` 差分なし | Pass |
| OBS-NI-2 | Logging 3 状態 | OFF は log file 非生成、ON は stderr+file sink active、fault は warning 付き stderr 縮退で job 継続 | oracle の前提 assertion が全状態で成立 | Pass |
| OBS-NI-3 | 同一 fixture/pipeline | 3 状態で同一 WAV bytes・同一 stages/input id | fixture SHA-256 を固定値と照合し、temp dir ごとに同一 bytes を供給 | Pass |
| OBS-NI-4 | CLI stdout JSON | `inspect` stdout が JSON で canonical 3 状態一致 | `inspect` stdout parse 成功。provenance projection 後、OFF/ON/fault で exact equality | Pass |
| OBS-NI-5 | f64 解析値 | cheap feature JSON の浮動小数点値が状態間で一致 | `(path, f64::to_bits())` 抽出ベクトルが 3 状態で exact equality。抽出数 20 以上 | Pass |
| OBS-NI-6 | Export artifact | inventory・binary raw bytes・canonical JSON が一致 | 33 files の inventory equality。binary/WAV は raw bytes、JSON は canonical bytesで exact equality | Pass |
| OBS-NI-7 | Privacy | sentinel/PCM 系列が stderr/log file に出ない | ON/fault/OFF/`inspect` stderr と ON file log に ASCII sentinel、hex、base64、PCM/sentinel decimal 系列なし | Pass |
| OBS-NI-8 | 性能 | hard assert せず観測のみ | 補助実行 wall time: OFF 12,743 ms、ON 10,350 ms、fault 9,656 ms。assertion なし | Pass |
| OBS-NI-9 | メモリ指標 | 利用可能なら観測 | この harness にはメモリ取得経路なし | Unsupported |
| OBS-NI-10 | GUI-L2/EXT-L3・macOS/Linux・cross-consumer・pin/candidate tuple | 本 slice 範囲外として記録 | 未実施。統合・別 slice へ委譲 | NotRun |

## 正規検証 7 lane（canonical = mise、全て Windows local/headless）

| コマンド | exit | 結果 |
| --- | ---: | --- |
| `mise run fmt` | 0 | pass（初回は formatting diff を検出し、`cargo fmt --all` で修正後に再実行して pass） |
| `mise run check al-cli` | 0 | pass |
| `mise run lint al-cli` | 0 | pass（初回は `clippy::chunks_exact_to_as_chunks` を検出し、範囲内修正後に再実行して pass） |
| `mise run test al-cli --test obs_noninterference` | 0 | 2 passed / 0 failed。`logging_state_does_not_interfere_with_analysis_cli_or_export` pass、18.28s |
| `mise run test al-cli --test observability` | 0 | 13 passed / 0 failed |
| `mise run test al-analysis --test rf628_oracle` | 0 | 7 passed / 0 failed。`rf628_long_scenario_non_regression` 含む、86.20s |
| `mise run check-cli-headless` | 0 | `no GUI framework deps in 10 headless crates` |

補助証拠（canonical = mise）:

```text
cargo test --locked -p al-cli --test obs_noninterference -- --nocapture
exit code: 0
2 passed / 0 failed
wall_ms: off=12743 on=10350 fault=9656 (observation only)
```

非 fatal warning として、mbx が ruviform git checkout の cache mapping warning と
一部 build script の non-UTF-8 stdout による cache bypass warning を出したが、
いずれの lane の exit code にも影響しなかった。

## Canonical projection と privacy の要点

除外したのは random `JOB-*`/`ART-*`/`ALN-*` ID、absolute path、`duration_ms`
数値だけ。artifact inventory、feature id/value、validity、配列順、input id、
binary bytes、deterministic hash、export 内容は除外しなかった。
`golden/snapshot.json` の `.json` hash entry は raw JSON が random lineage ID
を含むため provenance 依存になり得るが、hash を省略せず canonical JSON の
SHA-256 を再計算して比較した。`.bin`/WAV/export manifest の deterministic
な内容は raw/canonical exact compare に残った。

Privacy は ASCII sentinel `AL127_OBS_SECRET`、hex
`414c3132375f4f42535f534543524554`、base64
`QUwxMjdfT0JTX1NFQ1JFVA==`、PCM 先頭 8 sample
`[-17289, -15302, -17941, -9918, -16322, 15832, 19703, -8172]`、先頭 bytes hex
`77bc3ac4ebb942d93ec0d83df74c14e0`、sentinel sample列
`[19521, 12849, 24375, 16975, 24403, 17747, 21059, 21573]`、comma-joined
decimal を concrete pattern として検査した。

## 判定境界

- ローカル検証は本変更範囲の Windows/headless 検証であり、統合受入の成功ではない。
- 統合検証は merge 後の対象 SHA に対する必須 CI 結果であり、Coordinator が確認する。
- manual CI dispatch は本 slice のゲートにしなかった。
- docs index（`docs/**/README.md`）は Coordinator 整備対象で、本 slice では未同期のまま。
- `scarecrow173/ruviform#281` への言及は補足参照に留め、close keyword は使わない。
