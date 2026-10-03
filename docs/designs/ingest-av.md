---
id: "DESIGN-034yYLa18yjs2eEhATeu20"
type: "design"
status: "approved"
title: "インジェストとAVタイミング設計"
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
    - "../specs/media-ingest.md"
    - "../specs/video-av-timing.md"
    - "../adr/implementation-stack.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# インジェストとAVタイミング設計

## Context

ゲームのキャプチャ映像と比較用ソース素材を入力として、デコード・
タイミング完全性・チャンネル / コーデック情報を記録する層を設計する。
この層の出力が後続の locate / 解析 / 可視化の全ての時間軸と
メタデータの基盤になる。設計は spec `../specs/media-ingest.md` と
`../specs/video-av-timing.md` に基づく。

## Scope

### In Scope

- メディアコンテナ / コーデック / 音声フォーマットの検出と記録
- 音声 / 映像ストリームのタイムスタンプ系列と不連続の観測
- 正規化 / リサンプリング履歴と元ファイルハッシュの保存
- キャプチャアーティファクト辞書の読み込みと照合可能な形式
- CFR を仮定しない A/V タイミング記録とイベント同期フック
- デコード済み PCM とメタデータを Analysis Dataset の inputs /
  intermediate に渡す境界

### Out of Scope

- 音声・映像の内容解析(解析ステージ側の責務)
- キャプチャアーティファクト辞書の具体的収録内容
- 実装言語 / デコーダライブラリの選定

## Components and Boundaries

- `ingest` コンポーネント: ファイル入力 → メタデータ抽出 →
  デコード済み PCM + 観測レコードを出力
- `av-timing` コンポーネント: 音声 / 映像の時刻情報を正規化し、
  フレーム PTS / 間隔 / ドリフト / 不連続候補を出力
- `artifact-dictionary` コンポーネント: バージョン管理された
  コーデック / コンテナ / SRC 知識の読み出し口
- 境界: デコード済み PCM / STFT / メタデータを dataset に保存し、
  downstream へは正規化済みの参照 id だけを渡す

## Data and Control Flow

- 入力ファイル → ハッシュ計算 → コンテナ / コーデックメタデータ抽出
- 音声ストリーム → デコード → 正規化前 PCM の保存 →
  正規化 / リサンプリング操作を履歴として記録
- 映像ストリーム → フレーム PTS / 間隔系列の抽出 → VFR / ドリフト /
  ドロップフレーム候補の観測
- コンテナが明示的な PTS を持たない場合(レガシー AVI)、mux 順と
  `strh` クロックから導出した per-packet タイムスタンプ列そのものを
  観測統計とともに保持し、導出値であることをフィールド名
  (`movi_order_timestamps` + `start_tick`)で明示する
- 各観測は artifact-dictionary と照合して注釈を付与するが、
  生計測は抑制しない
- 成果物は `analysis/inputs/` と `analysis/intermediate/` に出力し、
  後段は manifest 経由で参照する

## Non-Functional Constraints

- 再現性: 同じ入力と辞書バージョンで同じメタデータ / PCM を生成する
- 追跡性: すべての変換操作が履歴として残る
- 完全性: CFR 非依存、音声 / 映像クロックを個別に保持
- 副作用: 生計測を辞書照合で変更しない

## Risks and Trade-offs

- コーデック / コンテナの多様性により全入力を網羅できないリスク →
  未対応形式は `decode/input unavailable` と明示的に記録して失敗を
  曖昧にしない
- VFR / パケット欠落の扱いは複雑になる → タイムスタンプを一次情報とし、
  公称 FPS は参考値としてのみ保持する
- 辞書の将来更新で解釈が変わる → 辞書バージョンを結果に紐付ける

## Deferred Design Concerns

- 対応コンテナ / コーデックの初期セット | 自前デコードモジュールの
  段階的追加に依存(`../adr/implementation-stack.md`) |
  実装フェーズ | 対象外入力の取りこぼし
  → 解決済: WAV/PCM + レガシー AVI(PCM 音声・映像は宣言メタ
  データのみ)。`../impl/ir/avi-container-ingest-and-video-fixtures.md`
  を参照
- 映像イベント検出器の具体的実装 | 解析対象ゲームに依存 |
  実装フェーズ | 汎用機構なしの個別実装増殖

## Verification Notes

- [ ] 同一入力で同一のメタデータ / PCM ハッシュが得られる
- [ ] VFR 入力がタイムスタンプ系列で処理される
- [ ] 完全性観測が辞書照合の注釈のみで生計測を変更しない
