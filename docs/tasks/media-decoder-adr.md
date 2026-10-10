---
id: "TASK-036aQwErTyUiOpAsDfGhJk"
type: "task"
status: "done"
title: "MediaDecoder トレイト + レジストリ設計の ADR(#36 フェーズ0)"
created: "2026-10-09"
updated: "2026-10-10"
owners: []
relations:
  source:
    - "../designs/media-format-analysis-extensions.md"
    - "../specs/media-ingest.md"
  changes:
    added:
      - "../adr/media-decoder-backend.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/media-format-analysis-extensions.md"
  implemented-by: []
  depends-on:
    - "../plans/media-format-analysis-extensions.md"
  blocks:
    - "media-decoder-native.md"
  supersedes: []
  superseded-by: []
  related:
    - "../adr/media-decoder-backend.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/media-format-analysis-extensions.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/36"
  defers: []
  deferred-by: []
---

# MediaDecoder トレイト + レジストリ設計の ADR(#36 フェーズ0)

## Work

issue #36 のプラガブルデコーダ設計を ADR として決定する。

- `MediaDecoder` トレイトの責務境界(コンテナ demux と codec
  decode の分離/一体)
- backend 評価: `symphonia`(純 Rust、音声のみ)/ `ffmpeg`
  子プロセスフォールバック(feature-gated・承認制)/ 自前拡張
- 出力正規化契約(統一 PCM 表現 + provenance 記録)
- 既存 ADR `mp4-demux-decode.md`(codec decode をスコープ外と
  した判断)との関係 — supersede/amend の形を決める
- ADR ドキュメントを `docs/adr/` に追加し README 索引を更新

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/media-format-analysis-extensions.md`
- `docs/designs/media-format-analysis-extensions.md`(approved)
- `docs/specs/media-ingest.md`
- `docs/adr/mp4-demux-decode.md`
- issue #36

## Dependencies

なし(設計フェーズ。実装タスクの前提)

## Affected Files

- `docs/adr/`(新規 ADR)
- `docs/adr/README.md`(索引)

## Done When

- [x] ADR が `accepted` ステータスで追加される
- [x] backend 選定(native 優先 + フォールバック方針)が決定
- [x] #36 のフェーズ1(既存実装の移行)に必要な契約が定義済み

## Verification

- [x] `node .agents/skills/adr-doc/scripts/review_adr.js --file docs/adr/media-decoder-backend.md --json` が exit 0、`findings: []`。
- [x] `node .agents/skills/adr-doc/scripts/audit_adr.js --dir docs/adr --json` が exit 0、新規 ADR の warning/error なし。
- [x] `git diff --check` が exit 0。
- [x] ADR が `docs/adr/README.md` 索引に登録され、task 索引の status が done。

## Outcome

[ADR](../adr/media-decoder-backend.md)を accepted として決定した。
4候補を比較し、native 優先 registry と依存なしの既存能力移行を採用。
PCM は未スケーリング interleaved f64le、能力不足のみ将来 fallback の対象とし、
破損/I/O を fallback で隠さない。MP4 demux-only ADR は refines/amend で維持する。
2026-10-10 に coordinator が具体案と accepted 化を承認した。

codec 拡大時の Symphonia version/features と FFmpeg の配布・実行 budget・
stream/clock 対応は ADR の未決事項に記録した。native 移行の前提には含めない。
実装と runtime テストは TASK-036b に残り、本タスクでは実行していない。
ADR audit の既存 `residual-issue-boundaries.md` に対する関係リンク warning 3件は
対象外として保持した。全 ADR の「Pros and Cons」info は checker が見出しを
完全一致で検出するためで、新規 ADR は MADR の `Pros and Cons of the Options`
を含む。新規 ADR の warning/error はなく、review_adr は指摘なし。
