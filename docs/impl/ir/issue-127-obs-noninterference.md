---
id: "IMPLIR-AL127OBSNONINTERFERENCE"
type: "implementation-record"
status: "implemented"
title: "Issue #127 OBS 非干渉 oracle implementation record"
created: "2026-11-28"
updated: "2026-11-28"
owners: []
relations:
  implements:
    - "../../specs/issue-127-obs-noninterference.md"
    - "../../designs/issue-127-obs-noninterference.md"
    - "../../tasks/issue-127-obs-noninterference.md"
  verified-by:
    - "../../test-specs/issue-127-obs-noninterference.md"
---

# Issue #127 OBS 非干渉 oracle implementation record

## 実装概要

`crates/al-cli/tests/obs_noninterference.rs` を新設し、製品コードと既存
`crates/al-cli/tests/observability.rs` を変更せずに AL-OBS-NONINTERFERENCE
oracle を実装した。依存追加はない。

主要構成:

- `fixture()`: 48kHz mono 10 秒の deterministic noise PCM s16 WAV を生成し、
  sample index 4096 から ASCII sentinel `AL127_OBS_SECRET` を埋め込む。
  SHA-256 を固定値と照合する。
- `run_state()`: OFF/ON/file-sink fault を独立 base dir で実行し、全 spawn
  で `AL_LOG`/`AL_LOG_FORMAT` を除去する。状態前提 assertion、privacy
  assertion、`inspect` JSON 検査、export projection を行う。
- `canonical_json_slice()`/`canonical_json_file()`: object key sort、
  random provenance ID 置換、absolute path 置換、`duration_ms` 数値置換を
  行う。`golden/snapshot.json` の `.json` hash entry は対象 JSON の
  canonical hash を再計算して投影する。
- `export_projection()`/`assert_projection_eq()`: export inventory と
  artifact 内容を比較する。binary/WAV は raw bytes exact compare、JSON は
  canonical bytes exact compare。
- `cheap_feature_f64_values()`: cheap feature JSON の浮動小数点 number を
  path 付き bit 表現で抽出し、3 状態で exact compare する。

## 設計判断

`run` stdout は `job_id=`/`job_dir=` のため JSON oracle には使わず、JSON を
返す `inspect` stdout を canonical compare 対象にした。job id や temp path、
wall-clock duration は製品意味を持たない provenance としてのみ投影した。
artifact 集合、feature 値、binary bytes、deterministic hash は投影対象外
または exact compare 対象に残した。

`golden/snapshot.json` の `.json` hash entry は、raw JSON が random lineage
ID を含むと provenance 依存になる。hash 検査を省略せず、canonical JSON の
SHA-256 を再計算して比較することで、provenance 依存と payload 差分を分離
した。`.bin`、WAV、export manifest など deterministic な artifact は raw
または canonical 後の exact compare で検査する。

## 検証記録

補助証拠（canonical = mise）として実装直後に実行:

```text
cargo test --locked -p al-cli --test obs_noninterference -- --nocapture
exit code: 0
2 passed; 0 failed
```

観測 wall time（assertion ではない）: OFF 12,743 ms、ON 10,350 ms、
file-sink fault 9,656 ms。正規 7 lane の結果は
`evidence/al127/obs-noninterference.md` と PR 本文に記録する。

## 未対応・範囲外

- docs index（`docs/**/README.md`）は Coordinator が統合時に整備する。
- GUI-L2/EXT-L3、macOS/Linux、cross-consumer、pin/candidate tuple、
  `scarecrow173/ruviform#294` 形式への正式引き渡しは本 slice の範囲外。
- メモリ指標はこの harness では取得しないため `Unsupported`。
