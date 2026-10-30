# AGENTS.md

`audio-lens`(AudioLens)は、ゲームのキャプチャ映像と比較用入力を
アラインし、音声に何が起きたかを計測して構造化解析データと
可視化用データを export する、ゲーム音声デバッグ解析ツール。
設計・仕様などのドキュメントは `docs/` にまとめる。

## 作業範囲と完了条件

- 依頼された範囲に変更を絞り、無関係な変更や既存の作業を保持する。
  範囲が曖昧な場合は、採用する前提を簡潔に示す。
- 調査・分析のみの依頼では編集しない。変更依頼では、実装、必要な
  文書・テストの更新、対象を絞った検証、今回の変更に起因する
  不具合の修正まで進める。
- Issue 駆動の変更を PR にまとめる場合、PR 本文に
  `Closes #NNN`(部分対応なら `Refs #NNN`)を含めて関連付ける。
  マージ時の自動 close はマージ前の記述にのみ発火するため、
  作成時に書く。

## Ruviform とのマルチリポジトリ運用

- 共通マルチメディア基盤は `scarecrow173/ruviform`
  (https://github.com/scarecrow173/ruviform) が所有し、AudioLens は
  その利用側アプリケーションとする。処理レベルで独立再利用できる
  数値演算・DSP・メディア I/O・データモデル等は Ruviform への
  配置を検討し、ゲーム音声デバッグ固有の locate 判断・解析
  オーケストレーション・Job・Dataset・可視化は AudioLens に残す。
  責務と移行状況が未確定の既存実装を無条件に削除しない。
- AudioLens の開発・調査・レビュー中に **Ruviform 側の不具合・
  契約違反・不足する汎用 API・改善案・機能提案**を発見した場合は、
  先に Ruviform の既存 Issue を検索し、重複がなければ **Ruviform
  リポジトリに Issue を起票する**。AudioLens の Issue だけに
  記録して終わらせない。内容が重複する場合は既存 Issue に
  参照・追加情報を残す。
- Ruviform 側 Issue には、背景・実際と期待の挙動・再現手順または
  根拠・汎用的な責務と対象 API/モジュール・影響範囲・検証条件を
  明記する。関連する AudioLens の Issue/PR と相互参照し、依存が
  ある場合は AudioLens 側にもブロッカーとして記録する。
  RT/非 RT の契約、f32/f64 精度、時刻・ストリーム情報、性能、
  互換性が関係する場合は、その制約とトレードオフも明記する。
- Ruviform が責務を持つ問題を、AudioLens 側だけの恒久的な
  workaround や汎用コードの重複で隠さない。必要な一時回避は
  理由・削除条件・Ruviform Issue の参照を記録する。
  **他リポジトリへの実装変更**はそのリポジトリの作業契約・
  レビュー手順に従い、AudioLens の変更から暗黙に行わない。
- 方針: 高精度オフライン解析の汎用 API は Ruviform へ追加する。
  メディアの入出力は Ruviform の抽象境界を使用し、解析に必要な
  観測情報が不足する場合は Ruviform の I/O 契約を拡張する。
  共通化の単位はクレート名や API 名ではなく、処理の意味・
  不変条件・汎用性を基準に判断する。

## 環境とコマンド

開発環境の正規インターフェースは mise task。新しいワークツリー
では、作業開始前に次を実行する(Windows / macOS / Linux 共通、
Bash 不要)。

```bash
mise install
mise run setup   # apm install + prek install + git lfs install/pull
```

`mise run setup` は `prek install` で pre-commit hook も有効化する。
hook 定義は `.pre-commit-config.yaml` — commit 時点では高速な整形・
衛生 check のみ担い、重い検証は `mise run lint` / `mise run test` に
委ねる。`git lfs install --local` + `git lfs pull` で LFS 実体も復元する
(git-lfs 未導入のまま clone した環境でも pointer のまま残らない)。

`.worktreeinclude` に列挙した gitignore 済み生成物（APM 依存・各 agent
向け deploy 物）は、新しいworktreeへ引き継がれる。ツールチェーンが
導入済みで依存定義も変わらない場合、worktreeごとの `mise run setup` は
通常不要。APM 依存が変更された場合や生成資産が存在しない場合は
`mise install` → `mise run setup` で復元する。

ユーザー環境の global mise config に追加ツールがある環境で、repo の
toolchain のみを復元する場合は toolchain を明示した
`mise install rust python uv github:microsoft/apm pipx:serena-agent prek biome git-lfs`
を使う。

mise 管理下のコマンドは `mise exec -- <コマンド>` で実行する
(例: `mise exec -- apm compile`)。

言語 stack は未定。決定次第 `mise.toml` に toolchain と
lint / typecheck / test 等の検証 task を追加し、この節も更新する。

## 開発原則

SOLID・YAGNI・DRY・KISS に従い、現在の要件に必要な範囲で設計・
実装する。

- **SOLID**: 単一責任、開放閉鎖、リスコフの置換、インターフェース
  分離、依存性逆転を踏まえ、責務と依存関係を明確にする。
- **YAGNI**: 将来使うかもしれない機能や拡張点を先回りして
  実装しない。
- **DRY**: 同じ知識や業務ルールの重複を避ける。見た目が似ている
  だけの処理を無理に共通化しない。
- **KISS**: 要件を満たす最も単純で理解しやすい構成を選び、不要な
  抽象化や複雑さを持ち込まない。
- **Pre-release**: このプロジェクトはリリース前のため、後方互換を
  維持するための互換層・deprecation 期間・マイグレーション・
  バージョン管理は設けない。形式や API を変更する場合は互換を
  保とうとせず、既存の成果物は作り直す(再生成する)方針とする。

実装内で表現ドリフトを起こさない。同じ情報を複数の表現方法
(型・フォーマット・命名)で扱うことを避け、同一概念の表現は
一箇所で定義して使い回す。表現の変換が必要な場合も変換処理を
一箇所に集約する。

## Git 運用

- ブランチは Conventional Branch に従い、`main` から作業単位の
  短命ブランチを切る。プレフィックスは `feature/`, `bugfix/`,
  `hotfix/`, `release/`, `chore/`、名前は小文字の kebab-case
- コミットメッセージは Conventional Commits に従う(`feat:`,
  `fix:`, `docs:`, `chore:` 等 + 命令形の要約)
- コミットは意味のある境界で分割し、適宜行う。無関係な変更を
  1コミットにまとめず、1コミットが1つの論理的な変更を表す
  ようにする

## APM / 生成物

- commit 対象: `apm.yml`, `apm.lock.yaml`, `AGENTS.md`
- commit しない: `apm_modules/` と `apm install` / `apm compile`
  が生成する各 agent 向け deploy 物(`.agents/`・`.claude/`・
  `.codex/`・`.cursor/`・`.github/`・`.windsurf/`・`.mcp.json`
  等)。ignore 済みかは `.gitignore` を参照
- clone / 生成資産のないfresh worktreeでは `mise run setup` で依存を復元する。
  資産を継承したworktreeでは依存定義の変更時など必要な場合に再実行する。

## 境界

- **Always**: 変更後は関連する検証(型チェック・テスト等、
  存在する範囲で)を実行してから完了とする
- **Ask first**: 依存の追加・変更、破壊的な Git 操作(ブランチ
  削除・履歴書き換え等)
- **Never**: 秘密情報のコミット、このリポジトリ外のファイルの変更

## 言語

文書、コメント、報告は日本語を基本とする。ユーザーが言語を
指定した場合はその指示を優先する。

スキル、エージェントのペルソナ・プロンプト、MCP 定義、自動処理や
言語処理の対象となる技術仕様では英語を使用できる。技術用語・
製品名は原語のまま記す。
