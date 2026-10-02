---
id: "PLAN-034z0yzgmf43IGEQwzG9gE"
type: "plan"
status: "approved"
title: "インジェストとAVタイミングの実装"
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
  implements:
    - "docs/specs/media-ingest.md"
    - "docs/specs/video-av-timing.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/media-ingest.md"
    - "docs/specs/video-av-timing.md"
    - "docs/designs/ingest-av.md"
    - "docs/adr/implementation-stack.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ingest-wav-decode.md"
    - "docs/test-specs/av-timing-observation.md"
  references: []
  defers: []
  deferred-by: []
---

# インジェストとAVタイミングの実装 Implementation Plan

## Goal

`al-media` クレートに自前のコンテナ検出・WAV デコード・メタデータ記録と、
CFR 非依存の A/V タイミング観測、バージョン付きキャプチャ
アーティファクト辞書を実装し、`Ingest` ステージとして runner に
接続する。

## Scope

本プランは 1 つの実装ストリーム。対象はインジェスト層の骨格と
最初のデコーダ(WAV/PCM)まで。

- In scope: コンテナ / フォーマット検出、WAV(RIFF/PCM)デコード、
  元ファイルハッシュ、音声メタデータ記録、PTS 系列ベースの
  タイミング観測、キャプチャアーティファクト辞書の読み込みと
  注釈クエリ、`Ingest` ステージ実装
- Out of scope: MKV / MP4 等のコンテナパーサ、実映像デコーダ、
  音声の内容解析(locate / 解析ステージ側)。映像タイミングは
  汎用タイムスタンプ源 trait と観測ロジックまでとし、実コンテナ
  からの PTS 抽出は別プランに defer する

## Upstream Documents

- `docs/specs/media-ingest.md`
- `docs/specs/video-av-timing.md`
- `docs/adr/implementation-stack.md`

## Design Inputs

- `docs/designs/ingest-av.md`
- `docs/designs/overview.md`

## File Map

- `crates/al-media/Cargo.toml` - メディア層クレート(al-core 依存のみ)
- `crates/al-media/src/format.rs` - コンテナ / フォーマット検出と
  未対応形式の明示的失敗(`decode/input unavailable`)
- `crates/al-media/src/wav.rs` - RIFF/WAV デマックスと PCM デコード
  (正規化前 PCM、チャンネル・サンプルレート・フォーマット記録)
- `crates/al-media/src/hash.rs` - 自前 SHA-256(依存追加を避け、
  元ファイルハッシュの provenance に使用)
- `crates/al-media/src/metadata.rs` - 音声メタデータ / デコード履歴 /
  不連続観測の型
- `crates/al-media/src/timing.rs` - タイムスタンプ源 trait と
  フレーム間隔・ドリフト・不連続の観測ロジック(CFR 非依存)
- `crates/al-media/src/dictionary.rs` - バージョン付きキャプチャ
  アーティファクト辞書の読み込みと注釈クエリ
- `crates/al-media/src/stage.rs` - `IngestStage`(Stage trait 実装、
  成果物を manifest に登録)
- `dictionaries/capture-artifacts.v1.json` - 辞書の初期版
- `crates/al-cli/src/main.rs` - Ingest スタブを実装に差し替え
- `crates/al-media/tests/` - 合成 WAV / 合成 VFR 系列の結合テスト

## Implementation Sequence

- [ ] `al-media` クレート骨格とフォーマット検出・明示的失敗を実装
- [ ] WAV/RIFF デマックス + PCM デコード + メタデータ記録を実装
- [ ] SHA-256 ハッシュを自前実装(既知ベクトルで検証)
- [ ] タイムスタンプ源 trait と間隔・ドリフト・不連続観測を実装
- [ ] キャプチャアーティファクト辞書(JSON + バージョン)を実装
- [ ] `IngestStage` を実装して成果物を manifest 経由で出力
- [ ] `al-cli` で Ingest を実装に差し替え、resume 互換を確認
- [ ] 合成 fixture で run → resume → inspect の結合テストを追加

## Dependencies and Risks

- 依存: `al-core` / `al-job`(既存)。外部クレートは追加しない方針
  (SHA-256・WAV とも自前実装。依存追加が必要になった場合は事前確認)
- リスク: 実コンテナパーサの不在で実キャプチャを読めない → 本プラン
  では WAV のみ対応とし、他形式は `decode/input unavailable` で
  明示的失敗にする(設計の方針どおり)
- リスク: 映像 PTS 抽出ができないと timing 観測の実証が弱い →
  タイムスタンプ源 trait + 合成系列テストで観測ロジックを独立検証する

## Task Breakdown

- `al-media` 骨格・フォーマット検出・明示的失敗
- WAV デコード・メタデータ・ハッシュ
- タイミング観測モジュール
- キャプチャアーティファクト辞書
- `IngestStage` 実装と CLI 接続
- 合成 fixture の結合テスト

## Verification

- [ ] `mise run test` が全 crate で通る
- [ ] 同一 WAV 入力で同一メタデータ / PCM ハッシュが得られる
- [ ] 合成 VFR タイムスタンプ系列から間隔・ドリフト・不連続が観測される
- [ ] 未対応形式が `decode/input unavailable` で明示的失敗する
- [ ] 辞書照合が注釈のみを付与し生計測を変更しない
- [ ] `al run --stages ingest` で実際の WAV が ingest できる

## Open Questions / Gaps

- MP4/MKV 等の実コンテナパーサは別プラン(範囲が大きいため)
- 映像イベント検出器は spec どおり本コアに入れない
- 辞書の初期収録内容は最小限とし、実測で育てる

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(単一ストリーム、依存順が明確)
- Independent work streams: なし
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same dependency
  graph and verification matrix.

## Review Handoff

- WAV のみを初期対応形式とする境界設定が妥当か確認
- 映像タイミングを trait + 合成系列に留める範囲判断を確認
- 外部依存を増やさない方針(SHA-256 自前実装)を確認
