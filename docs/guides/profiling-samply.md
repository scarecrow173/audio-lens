---
id: "GUID-01M4PA3KJZZA9F6CYDTGG611DJ"
type: "guide"
status: "active"
title: "samply による CPU プロファイリング手順"
created: "2026-10-11"
updated: "2026-10-11"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/observability-foundation.md"
    - "../specs/observability-foundation.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/observability-foundation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# samply による CPU プロファイリング手順

samply は外部 CPU sampling プロファイラで、必要なときに随時使う
(L3 計測)。**計装は不要** — 対象バイナリに変更を加えず、OS の
sampling 機構でコールスタックを収集し Firefox Profiler 形式の
`.samply` ファイルに保存する。tracing の span 計装(L2)や
criterion(L1)とは独立して使える。

## 導入

```bash
cargo install samply --locked
```

`samply --version` で導入を確認する。

## release ビルドの debug info

root `Cargo.toml` に `[profile.release] debug = true` を設定済み。
codegen・最適化は変わらず、PDB / DWARF のデバッグ情報が
release 成果物に付くため、samply が関数名付きのスタックを解決
できる。trade-off は成果物サイズの増加のみ。

## 実行例

release ビルドしてから `samply record` に解析コマンドを渡す:

```bash
mise exec -- cargo build --release -p al-cli
samply record -- target/release/al run --base-dir ./jobs --stages ingest,locate --input cap.wav
```

`--` 以降は通常の `al` 実行そのまま。`AL_LOG` 等の環境変数も
そのまま効く(計装と sampling は独立)。

## 出力の見方

実行が終わると `profile.samply`(既定ファイル名、`--output` で
変更可)が生成され、Firefox Profiler がブラウザで開く。

- **Stack Chart / Call Tree**: 壁時間ベースのスタック集約。
  ホットスポットの関数名と自己/累積時間を読む
- **Marker / スレッド**: スレッド単位のタイムライン。al-locate
  の worker pool 等、並列実行の偏りを確認できる
- `.samply` ファイル自体は再読込・共有可能なアーカイブ

サンプリング分解能は OS と負荷に依存する。span レベルの因果
(どの `input_id`・`method` か)が必要な場合は L2 の
`AL_LOG=<crate>=debug` による tracing span を併用する。
