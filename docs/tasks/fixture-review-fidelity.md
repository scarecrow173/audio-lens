---
id: "TASK-034dLdqYq2QFIc8WBq2KLP"
type: "task"
status: "todo"
title: "al-testkit / ゴールデン fixture / 文書の残存精度(PR #23/#24 レビュー LOW)"
created: "2026-10-10"
updated: "2026-10-10"
owners: ["scarecrow173"]
relations:
  source: []
  implements:
    - "docs/plans/residual-fixes.md"
  implemented-by: []
  depends-on:
    - "docs/plans/residual-fixes.md"
    - "docs/designs/residual-fixes.md"
    - "docs/adr/residual-issue-boundaries.md"
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
    - "docs/test-specs/residual-fixes.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/30"
  defers: []
  deferred-by: []
---

# al-testkit / ゴールデン fixture / 文書の残存精度(PR #23/#24 レビュー LOW)

## Work

#30 の残件を以下の原文棚卸しに沿って確認する。Issue 全体を対応単位とし、修正・文書化・承認済み defer の各結果と証拠を項目ごとに残す。複数の独立修正を実施するときは承認後に task を分割し、この対応を保持する。

## Classification

- `bug-fix`
- draft plan のレビュー用候補。依頼により先に文書化するが、通常の承認済み計画由来の実装 task ではない。

## Upstream Documents

- `docs/plans/residual-fixes.md`
- `docs/specs/residual-compliance.md` AC-30
- `docs/designs/residual-fixes.md`
- Issue #30

## Dependencies

spec/ADR はチェーンの受理可能な status を満たし、design/plan と test-spec は承認済みのため status todo。

## Affected Files

- `crates/al-testkit/src/media.rs`
- `crates/al-media/tests/golden.rs`
- `crates/al-locate/tests/golden.rs`
- `crates/al-analysis/tests/golden.rs`
- `docs/specs/expensive-analysis.md`

## Issue Inventory (2026-10-10)

### 背景

PR #23/#24 のレビューで検出された非ブロッカー指摘。

### 項目

- `al-testkit` `push_list`: 奇数サイズ LIST ボディのワードパディングがなく、エクスポートされたヘルパーが非整列チャンクを生成しうる
- `AviSpec`: `MoviPacket.stream` の `0..=99` 制約がドキュメントのみで未強制(100 で `:` を生成)
- `write_avi`: `AVIF_HASINDEX` を設定するが `idx1` を書かない(旧 writer からの継承)
- `bitmap_info_header`/`AviSpec::cfr`: `width*3 % 4 == 0` の未パディング前提が旧 doc から落ちている
- `pinned_periodic_everywhere_not_accepted_at_wrong_phase`: `if accepted { ... }` のソフトピンで、全拒否でもパスする(どちらが正しいかを固定していない)
- テスト失敗時に一時ディレクトリが残る(`remove_dir_all` が成功パスのみ)
- golden fixture の `source_sha256` 一貫性: analysis はダミー値、locate は未設定(どちらも実在しない入力を指す — ステージが `decoded_pcm` を直接読むため動作はするが注記が欲しい)
- `strh_body` の `rcFrame` が全ストリームでゼロ(旧 writer は video に (0,0,w,h) を書いていた)
- 仕様 defer 品(spec `expensive-analysis.md`):精密スペクトル自己相関の独立指標化、エイリアス/折り返し独立相関記述子、ナイキスト領域構造、ドロップアウト/ギャップ形状の精緻化 — discovery で明示 defer 済みだが実装の受け皿がない

Refs #23 #24

## Steps

1. 原文の各項目を実装と既存 spec に照合し、修正か判断かを記録する。
2. 違反する契約を固定する回帰テストを追加する。
3. 対象テストを実行し、期待する失敗を確認する。
4. 所有境界で修正する。判断項目は承認後に着手する。
5. 対象検証を再実行する。
6. 項目ごとの証拠と文書を更新する。
7. 意味のある変更単位でコミットする。

## Done When

- [ ] 奇数 LIST、stream 99/100、index フラグ、DIB stride と rcFrame が生成するメディアの契約と一致する。周期 fixture は全拒否で成功しない。失敗時も一時ディレクトリが回収され、合成 source hash の意味と defer 項目の受け皿が明記される。
- [ ] 原文全項目に修正・文書化・承認済み defer と検証証拠の対応がある。
- [ ] 関連 spec/ADR/design/test-spec が実施内容と一致する。

## Verification

- [ ] `mise exec -- cargo test -p al-testkit -p al-media -p al-locate -p al-analysis` が exit 0。原文項目に対応した回帰テストを含む。
- [ ] `mise run fmt` と `mise run lint` が exit 0。
- [ ] 文書と Issue 項目の対応を review する。

現時点では文書整備のみであり、上記実装検証は未実行。
