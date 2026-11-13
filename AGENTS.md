# AGENTS.md

## ポリシー参照（必須）

Issueの起票・改訂、設計、実装、テスト、レビュー、受入判定の際は、このリポジトリの [POLICIES.md](POLICIES.md) を読み、その規則に従う。

`audio-lens`(AudioLens)は、ゲームのキャプチャ映像と比較用入力を
アラインし、音声に何が起きたかを計測して構造化解析データを
出力する、ゲーム音声デバッグ解析ツール。
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

- `scarecrow173/ruviform`
  (https://github.com/scarecrow173/ruviform) は再利用可能な
  共通基盤を担い、AudioLens はその利用側アプリケーションとする。
  処理単位で用途に依存せず再利用できるものは Ruviform への
  配置を検討し、AudioLens 固有の処理は AudioLens に残す。
  責務・不変条件・依存方向を踏まえて判断し、既存実装を
  無条件に移動・削除しない。
- AudioLens の開発・調査・レビュー中に Ruviform 側の不具合、
  契約上の問題、改善案や機能提案を見つけた場合は、Ruviform の
  既存 Issue を確認し、重複がなければ Ruviform リポジトリに
  Issue を起票する。既存 Issue がある場合は参照・情報を追加する。
- Issue には、背景・根拠・期待する振る舞い・影響範囲・検証条件を
  記載し、関連する両リポジトリの Issue / PR を相互参照する。
  AudioLens 側の作業が依存する場合は、その依存関係も記録する。
- Ruviform が責務を持つ問題を、AudioLens 側だけの恒久的な
  回避策や重複実装で隠さない。一時的な回避策が必要な場合は、
  理由・解消条件・関連 Issue を明記する。
  他リポジトリの実装変更は、そのリポジトリの作業規約に従う。

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

Rust toolchain は `mise.toml` では pin しない。通常開発は環境導入済みの
rust(rustup / global mise)を使い、cargo 系タスクはその toolchain で
実行される。CI 専用の pin は `mise.ci.toml` に置き、CI では
`MISE_ENV=ci` で有効化する(lockfile は `MISE_ENV=ci mise lock` が
生成する `mise.ci.lock`)。

ユーザー環境の global mise config に追加ツールがある環境で、repo の
toolchain のみを復元する場合は toolchain を明示した
`mise install uv github:microsoft/apm pipx:serena-agent prek biome git-lfs node actionlint`
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
