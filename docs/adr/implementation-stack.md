---
id: "ADR-034yZb7m1ru4R4k7m8jLme"
type: "adr"
status: "accepted"
title: "実装言語に Rust を採用しコア処理を自前モジュールで実装する"
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
  refined-by:
    - "ruviform-foundation-migration.md"
  derives-from:
    - "../discovery/initial-requirements-consolidation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# 実装言語に Rust を採用しコア処理を自前モジュールで実装する

## Context and Problem Statement

AudioLens は、メディアのエンコード / デコード、DSP 計測
(STFT・相関・残差)、大規模中間データの保存、決定的な再実行という
処理を中核に持つ。これらを支える実装言語と構成方針を定める必要が
ある。要件上、解析アルゴリズムの追加・修正が頻繁に起き、かつ
出力は配布可能な形であることが望ましい。

## Decision Drivers

* 長尺キャプチャに対する定位・残差探索の計算性能を確保できること
* DSP / デコード処理を本プロジェクトで再現可能な形に実装できること
* モジュール分割により、将来別用途や他ツールへ再利用できること
* 配布・再現・検証を妨げない実行基盤であること
* 大規模データの保存 / 読み出しを安定して扱えること

## Considered Options

* Python 主体で、DSP / データ基盤は外部ライブラリに依存する
* Rust で、コア処理をプロジェクト内のモジュールとして設計・実装する
* C++ で実装する
* Rust をコア、Python をオーケストレーション / 補助に併用する

## Decision Outcome

Chosen option: "Rust で、コア処理をプロジェクト内のモジュールとして
設計・実装する", because エンコード / デコードと DSP を本プロジェクトの
責任として新規に設計・実装し、再利用可能なモジュール群として整理する
方針が採用されたため。

### Consequences

* コーデック / コンテナ / DSP の実装は段階的に追加し、未対応形式は
  `decode/input unavailable` として明示する
* モジュール境界を crate / ライブラリ単位で設計し、再利用性を高める
* 外部ライブラリ依存は最小限に抑え、必要な場合も置き換え可能な
  インターフェースで隔離する
* 実装量は増えるが、解析責務の内製化と再利用性を優先する

### Confirmation

* Rust toolchain とモジュール構成は `mise.toml` の開発インターフェース
  に従って整備する
* 少なくとも 1 つのデコード経路と 1 つの計測経路が、自前モジュールの
  みで fixture 検証できること

## Implementation Plan

* 実装モジュールの境界は、ingest / locate / analysis / dataset /
  visualization / export の各設計ドキュメントに従う
* 言語・toolchain・検証 task の具体設定は `mise.toml` と開発環境
  セットアップで行う
* モジュール分割の詳細は plan / task 文書で決める

## Verification

* 自前モジュールだけで、単一フォーマット入力の decode → locate →
  簡易計測が再現可能に動くこと
* 外部依存なしでモジュール境界をテストできること

## Pros and Cons of the Options

### Python 主体で、DSP / データ基盤は外部ライブラリに依存する

* Good: 開発速度が最も高い。既成ライブラリで短期に多くの観測を
  実装できる
* Bad: エンコード / デコードや DSP を自前実装する方針に反し、
  実行環境依存が強い

### Rust で、コア処理をプロジェクト内のモジュールとして設計・実装する

* Good: 性能と配布性が高い。設計を crate / モジュールに分けやすく、
  再利用しやすい
* Bad: 既成 DSP ライブラリに比べて実装量が増える。対応フォーマットを
  順次拡大する必要がある

### C++ で実装する

* Good: FFmpeg / DSP 直実装の自由度が最も高い
* Bad: 開発速度と安全性のバランスが悪く、モジュール化の再利用性より
  実装コストが支配的になる

### Rust をコア、Python をオーケストレーション / 補助に併用する

* Good: 実装言語を用途別に分けられる
* Bad: 二言語運用の複雑さが増し、モジュール化方針に対して過剰

## More Information

* この決定は開発環境選定後の実装範囲を定めるものであり、
  `docs/designs/` の各詳細設計は引き続き技術中立の構造を保持する
