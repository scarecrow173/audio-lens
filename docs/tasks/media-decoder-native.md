---
id: "TASK-036bZxCvBnMqWeRtYuIo"
type: "task"
status: "done"
title: "既存 WAV/AVI/MP4 を MediaDecoder native バックエンドへ移行(#36 フェーズ1)"
created: "2026-10-09"
updated: "2026-10-10"
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
    - "docs/plans/media-format-analysis-extensions.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/media-decoder-adr.md"
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
    - "docs/test-specs/media-format-analysis-extensions.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/36"
  defers: []
  deferred-by: []
---

# 既存 WAV/AVI/MP4 を MediaDecoder native バックエンドへ移行(#36 フェーズ1)

## Work

ADR で決定した `MediaDecoder` トレイトに既存実装を移し、
呼び出し側をレジストリ経由に統一する。

- `wav.rs` / `avi.rs` / `mp4.rs` のデコードパスを
  `MediaDecoder` 実装(native backend)として再編
- `detect` / `stage` の分岐を拡張子ベースの direct dispatch から
  レジストリ照合へ変更
- provenance に decoder/backend 種別を記録
- コーデック追加(AAC/FLAC/MP3/H.264 デコード等)はこのタスクの
  範囲外 — 後続計画に分離

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/media-format-analysis-extensions.md`
- accepted `docs/adr/media-decoder-backend.md`

## Dependencies

- `docs/tasks/media-decoder-adr.md`

## Affected Files

- `crates/al-media/src/`(`decoder.rs` / `dib.rs`、WAV/AVI/MP4 adapter、stage)
- `crates/al-media/tests/registry.rs` / `ingest.rs`
- 本 task と `docs/tasks/README.md`、ADR の MP4 能力表訂正

## Done When

- [x] 既存の WAV/AVI/MP4 ingest テストが変わらずパス
- [x] decode 経路がレジストリ経由になる
- [x] provenance に decoder/backend が記録される

## Verification

- [x] `mise exec -- cargo build -p al-media -p al-core -p al-job -p al-testkit` が exit 0。
- [x] `mise exec -- cargo test -p al-media -p al-core -p al-job -p al-testkit` が exit 0。
- [x] `mise exec -- cargo fmt --all -- --check` が exit 0。
- [x] `mise exec -- cargo clippy -p al-media -p al-core -p al-job -p al-testkit --all-targets -- -D warnings` が exit 0。
- [x] `git diff --check` が exit 0。

## Implementation Notes

- `MediaDecoder` の signature detect / PCM を生成しない probe / 要求別 decode と、
  native を最初に登録する `DecoderRegistry` を実装した。同一 backend ID の重複を拒否し、
  フェーズ1では fallback を実行しない。WAV/AVI は構造解析を既存 parser と共有し、
  MP4 は sample 抽出を行わない probe mode を同じ demux に追加した。
- `IngestResult` に PCM、stream facts、timing、映像特徴、integrity を集約し、
  stage は pinned hash 検証、辞書注釈、部分 ingest ポリシー、成果物登録を担当する。
  AVI 音声・映像の unsupported / malformed は文字列化前の型付き区分で保持する。
- DIB pixel decode を `dib.rs` に移し、top-down luma / 元 clock / RGB preview を
  逐次処理する共通特徴抽出に渡す。全 frame の decoded plane は保持しない。
- metadata と生成 artifact params に hash、native/version、要求、選択 stream と
  結果/理由、試行順、変換なし、fallback=false を記録する。channel layout は unknown。
- registry テストは PCM 各対応幅、非有限 float、first-stream 選択、probe/input 一致、
  固定順/重複 ID、局所破損/能力不足、timing-only、MP4 既存 PCM 抽出を検証する。

## Non-blocking Follow-ups

- 実装前の ADR は MP4 に既存 PCM 抽出がないと誤認していた。coordinator の
  2026-10-10 指示により、その既存能力を維持し能力表に訂正注記を追加した。
  AAC/H.264 の新規 decode、MP4 の任意 stream 選択は追加していない。
- 後続 backend の能力不足に限った fallback と成功済み観測の合成、異なる demux の
  stream/clock 整合は、その backend の依存/設定承認後に実装する。registry の
  フェーズ1既定構成は native 一件で、破損/IO を別候補で隠さない。
- Symphonia 評価、FFmpeg process の配布/明示許可、codec 拡大は後続計画。
  今回は外部/内部 workspace 依存と Cargo manifest/lock を変更していない。
- probe と decode は同じ hash の bytes を使い、既存の構造解析を再利用する。
  フェーズ1では decode 時に構造を再走査する。解析済み状態のキャッシュは
  将来の計測で必要性が確認された場合のみ検討する。

- 補助 `audit_docs.js --type task --dir docs/tasks --json` は既存文書に 9 件の
  指摘を返した（8 件の ID 形式、`media-decoder-adr.md` の changes.added 型）。
  本 task の ID `TASK-036bZxCvBnMqWeRtYuIo` も変更前からの ID 形式指摘対象であり、
  関係参照を壊さないため今回は ID を変更しない。runtime 完了を妨げない文書整備課題。

## Verification Results

2026-10-10、上記 build/test/fmt/clippy と `git diff --check` はすべて exit 0。
対象 4 クレートで 171 tests が成功（al-media: 118、うち golden: 12、registry: 10）。
既存 ingest/golden の期待値は変更していない。ADR review は No findings。
