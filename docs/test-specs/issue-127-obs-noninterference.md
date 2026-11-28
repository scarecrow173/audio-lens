---
id: "TESTSPEC-AL127OBSNONINTERFERENCE"
type: "test-spec"
status: "proposed"
title: "Issue #127 OBS 非干渉 oracle test specification"
created: "2026-11-28"
updated: "2026-11-28"
owners: []
relations:
  verifies:
    - "../specs/issue-127-obs-noninterference.md"
  source:
    - "../specs/issue-127-obs-noninterference.md"
    - "../designs/issue-127-obs-noninterference.md"
  implemented-by:
    - "../impl/ir/issue-127-obs-noninterference.md"
---

# Issue #127 OBS 非干渉 oracle test specification

## 正規入口

```text
mise run test al-cli --test obs_noninterference
```

対象ファイルは `crates/al-cli/tests/obs_noninterference.rs`。Cargo integration
test 自動検出を使い、`Cargo.toml` と `Cargo.lock` は変更しない。

## Fixture

| 項目 | 値 |
| --- | --- |
| 形式 | PCM s16 WAV |
| sample rate | 48,000 Hz |
| channel | mono |
| frame 数 | 480,000（10 秒） |
| 基底信号 | `al_testkit::signal::noise(480000, 0xA127, 0.62 * 32767.0)` を round/clamp して s16 化 |
| sentinel | sample index 4096 から 8 sample に ASCII `AL127_OBS_SECRET` の little-endian s16 byte 列 |
| WAV SHA-256 | `dcad8817615ce1b56c67247e9dbc72fca36e8ba4f2afddaf17a1b5b3dab3eefb` |

fixture は現在時刻・乱数器の状態・環境変数・host path に依存しない。
temp dir 名には実行固有値が入るが、fixture bytes には入らない。

## Pipeline

すべての状態で同一コマンド形状を使う。

```text
al run --base-dir <state jobs dir> \
  --input capture_bug:<state cap.wav> \
  --input-ids IN-0000000000000000000001 \
  --stages ingest,cheap_analysis,dataset,export
```

ON/fault では `--log info` を追加する。`AL_LOG` と `AL_LOG_FORMAT` は全 spawn
で除去し、親環境による汚染を防ぐ。

## 状態前提 assertion

| 状態 | assertion |
| --- | --- |
| OFF | `<base_dir>/logs` が存在しない。stderr/log に sentinel が出ない。job が成功する。 |
| ON | stderr に `stage completed` がある。`al.*.log` が 1 つあり、file sink に JSON stage event がある。job が成功する。 |
| file sink fault | `<base_dir>/logs` を regular file として占有する。stderr に `warn: cannot initialize file log sink` と `stage completed` がある。job が成功する。 |

## 非干渉比較

### CLI stdout JSON

`inspect --job-id <job>` stdout を JSON parse し、canonical byte列を 3 状態で
exact compare する。`run` stdout は `job_id=`/`job_dir=` 形式で JSON では
ないため、CLI stdout JSON oracle には使わない。

### f64 解析値

`export/analysis/analysis/features/cheap/IN-0000000000000000000001.json` を
走査し、浮動小数点 number を path 付きで `(path, f64::to_bits())` として抽出
する。抽出数が 20 以上であることを確認し、3 状態で exact compare する。
random lineage artifact ID はこの抽出に含めない。

### Export artifact

`export/analysis` 以下の相対 path inventory が一致することを確認し、各
artifact を比較する。

- `.bin` と WAV: canonical 化せず raw bytes exact compare。
- `.json`: object key sort、provenance ID 置換、absolute path 置換、
  `duration_ms` 数値置換を行った canonical bytes exact compare。
- `export_manifest.json` は portable で raw bytes が一致することを期待するが、
  oracle は他 JSON と同じ canonical bytes でも検査する。

### Canonical projection の除外項目と根拠

除外するのは次だけである。

- random `JOB-*`/`ART-*`/`ALN-*` ID: 実行ごとの provenance であり解析意味を持たない。
- absolute path: host/temp dir 固有であり portable artifact 契約の対象外。
- `duration_ms`: wall-clock 実行時間であり性能閾値を assertion にしない。
- `.json` artifact の raw hash が random provenance を含む場合に限り、
  `golden/snapshot.json` の該当 entry を canonical hash へ投影する。

除外しないもの: artifact 集合、feature id、feature 値、validity、配列順、
input id、binary bytes、input/export の deterministic hash、canonical 化後の
JSON 構造。

## Privacy assertion

ON/file-sink fault の captured stderr、ON の生成 log file、OFF の stderr、
`inspect` stderr に対し、次の pattern が含まれないことを assert する。

- `AL127_OBS_SECRET`
- `414c3132375f4f42535f534543524554`
- `QUwxMjdfT0JTX1NFQ1JFVA==`
- PCM 先頭 8 sample `[-17289, -15302, -17941, -9918, -16322, 15832, 19703, -8172]`
- PCM 先頭 bytes hex `77bc3ac4ebb942d93ec0d83df74c14e0`
- sentinel sample列 `[19521, 12849, 24375, 16975, 24403, 17747, 21059, 21573]`
- 上記系列の comma-joined decimal 表現

## 性能・メモリ

wall time は観測のみで hard assert しない。メモリ指標はこの test harness で
取得しないため `Unsupported` とする。性能値は evidence に記録する。

## 判定

- 上記 assertion が全て成立: `Pass`。
- 比較差分または privacy leak を検出: 製品不具合として `Fail`。test を弱めない。
- lane が環境理由で完走しない: `InfrastructureFailure`。
- 3OS、GUI-L2/EXT-L3、cross-consumer、pin/candidate tuple は本 oracle の
  判定範囲外であり `NotApplicable` または `NotRun` として記録する。
