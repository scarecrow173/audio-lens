---
id: "DESIGN-AL127OBSNONINTERFERENCE"
type: "design"
status: "proposed"
title: "Issue #127 OBS 非干渉 oracle 設計"
created: "2026-11-28"
updated: "2026-11-28"
owners: []
relations:
  source:
    - "../specs/issue-127-obs-noninterference.md"
  implements:
    - "../specs/issue-127-obs-noninterference.md"
  implemented-by:
    - "../impl/ir/issue-127-obs-noninterference.md"
  verified-by:
    - "../test-specs/issue-127-obs-noninterference.md"
---

# Issue #127 OBS 非干渉 oracle 設計

## 設計概要

`obs_noninterference.rs` は self-contained な integration test として、
fixture 生成、3 状態実行、canonical projection、export 比較、privacy
assertion を 1 つの正規入口に集約する。既存 observability harness は
read-only reference とし、製品コード・既存 test・依存関係は変更しない。

## Fixture と pipeline

- 48,000 Hz、mono、480,000 frame（10 秒）、PCM s16 WAV。
- `al_testkit::signal::noise(FRAMES, 0xA127, 0.62 * 32767.0)` の決定的
  noise を round/clamp して s16 化する。
- sample index 4096 から 8 sample を ASCII sentinel `AL127_OBS_SECRET` の
  little-endian s16 byte 列に置き換える。WAV として valid な PCM のまま
  sentinel を payload に埋め込む。
- `al_testkit::media::wav_s16_bytes` で WAV bytes を生成する。乱数の種、
  現在時刻、環境変数、ホスト path は fixture 値に入れない。
- pipeline は `ingest,cheap_analysis,dataset,export`、input role は
  `capture_bug`、input id は固定の `IN-0000000000000000000001`。
- fixture SHA-256 は
  `dcad8817615ce1b56c67247e9dbc72fca36e8ba4f2afddaf17a1b5b3dab3eefb`。

## 3 状態

各状態は独立した temp dir/base dir を使い、子プロセスから `AL_LOG` と
`AL_LOG_FORMAT` を除去する。

- OFF: `--log` を付けない。`<base_dir>/logs` が生成されないことを確認する。
- ON: `--log info`。stderr に stage event があり、`al.*.log` が 1 つ生成
  され、file sink に JSON stage event があることを確認する。
- File sink fault: 先に `<base_dir>/logs` を regular file として作る。
  `warn: cannot initialize file log sink` を含み、stderr stage event が
  継続し、job が成功することを確認する。

## Canonical projection

raw output には random な `JOB-*`/`ART-*`/`ALN-*` ID、host 固有 absolute
path、stage `duration_ms` が含まれる。これらは製品意味ではなく実行
provenance なので、JSON object を再帰的に canonical 化するときだけ次を
置換する。

- `duration_ms` の数値: `<DURATION_MS>`。
- absolute path 文字列: `<ABS_PATH>`。
- `JOB-`/`ART-`/`ALN-` + 22 文字の ID: 文書内出現順の安定 placeholder。
- object key: 明示的に sort して compact JSON へ直列化する。

配列順、f64 bit 値、feature id、validity、input id、hash、artifact inventory、
export 内容は置換しない。binary artifact は canonical 化せず byte exact
compare する。

`golden/snapshot.json` の hash だけは特別扱いが必要である。snapshot に
記録される `.json` artifact の raw SHA-256 は、その JSON 内に random
artifact/job provenance が埋め込まれると provenance 依存になる。一方で
hash 検査を抜くことはできないため、oracle は対象 `.json` artifact を上記
canonical projection した byte 列の SHA-256 を再計算し、snapshot 内の対応
entry をその canonical hash に投影する。`.bin` と入力 WAV の hash entry は
raw deterministic bytes の hash として残り、export 側 binary artifact は
exact compare でも検査する。これにより hash を除外せず、provenance 依存
hash と製品意味のある内容 hash を分離する。

## 比較対象

1. `al inspect --job-id` stdout: JSON として parse できることを確認し、
   canonical byte列を 3 状態で exact compare する。`run` stdout は JSON
   ではないため対象外だが、job id 抽出と成功確認には使う。
2. `analysis/features/cheap/<input>.json`: JSON 内の浮動小数点 number を
   path 付きで走査し、`f64::to_bits()` ベクトルを exact compare する。
   random lineage ID はこの抽出に含めない。
3. `export/analysis` 以下: 相対 path inventory と全 artifact を比較する。
   JSON は canonical byte列、binary/WAV は raw byte列で比較する。

## Privacy assertion

次の concrete pattern が、captured stderr と生成 log file に出ないことを
確認する。

- ASCII sentinel: `AL127_OBS_SECRET`。
- sentinel bytes hex: `414c3132375f4f42535f534543524554`。
- sentinel base64: `QUwxMjdfT0JTX1NFQ1JFVA==`。
- 入力 PCM 先頭 8 sample の decimal 表現と raw little-endian hex
  `77bc3ac4ebb942d93ec0d83df74c14e0`。
- sentinel sample の debug/compact decimal 表現。

OFF でも stderr に sentinel が出ないことを確認する。field size だけを見る
既存 test の写しではなく、payload に対応する具体パターンで検査する。

## 性能観測

wall time は `Instant` で観測して stderr に出すが、hard assert しない。
メモリ指標はこの harness では取得しないため `Unsupported` として evidence
に記録する。

## 失敗時の扱い

canonical 化しても解析値・CLI JSON・Export が状態間で変わる場合、または
privacy pattern が漏れる場合は製品不具合として Fail にする。test の成功
条件を弱めない。lease 外修正が必要なら Coordinator へ範囲拡張を求める。
