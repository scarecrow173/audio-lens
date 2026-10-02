---
id: "TSPEC-034z0Ht3SwLfLVjMLB1TKy"
type: "test-spec"
status: "proposed"
title: "WAV インジェストとメタデータ記録"
created: "2026-10-02"
updated: "2026-10-02"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ingest-av.md"
  derived-by: []
  verifies:
    - "docs/specs/media-ingest.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# WAV インジェストとメタデータ記録

## Purpose

メディアインジェストの最初のデコーダ(WAV/PCM)が「メディアがどう
デコードされたかを完全に説明できる」ことを合成 fixture で検証する。
新しいフォーマット対応を追加するたびに同じ規範を再利用する。

## Feature

WAV 入力のデコード・メタデータ記録・ハッシュ provenance。

## Rules

- 正規化前のデコード済み PCM が保存される(正規化 / リサンプリングを
  暗黙に行わない)
- コーデック / コンテナ / サンプルレート / チャンネル数 /
  ビット深度 / ストリーム長がメタデータに記録される
- 元ファイルの SHA-256 ハッシュが成果物と一緒に記録される
- 同一入力は同一のメタデータとハッシュを再現する(決定性)
- 未対応形式は `decode/input unavailable` の明示的失敗になる

## Examples

- Given 合成 16bit ステレオ 48kHz WAV、When ingest を実行、
  Then サンプル数・レイアウト・ハッシュが記録される
- Given 同じ WAV を二度実行、Then メタデータ・ハッシュが一致する
- Given WAV でも既知形式でもないファイル、Then 明示的エラーで失敗する
- Given 破損した RIFF ヘッダ、Then 暗黙スキップせず明示的失敗する

## Guarantees

- PCM 値・タイムライン・メタデータがデコード経路から再構成できる
- ハッシュで入力ファイルと成果物の対応を一意に特定できる
- 失敗は常に明示的で、部分的な成果物を成功と誤記録しない

## Non-goals

- MKV / MP4 など実コンテナのパース(別プラン)
- 音声内容の解析(オンセット・レベル等は解析ステージ側)
- 非可逆コーデック(AAC/Opus 等)のデコード

## Risk

- ここを落とすと後段の全計測が「どうデコードされたか分からない
  信号」に対して行われ、原因切り分けの前提が崩れる
