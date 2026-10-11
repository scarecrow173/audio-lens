---
id: "TASK-034dLdqYq2QFIc8WBq2KLP"
type: "task"
status: "done"
title: "al-testkit / ゴールデン fixture / 文書の残存精度(PR #23/#24 レビュー LOW)"
created: "2026-10-10"
updated: "2026-10-11"
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

- [x] 奇数 LIST、stream 99/100、index フラグ、DIB stride と rcFrame が生成するメディアの契約と一致する。周期 fixture は全拒否で成功しない。失敗時も一時ディレクトリが回収され、合成 source hash の意味と defer 項目の受け皿が明記される。
- [x] 原文全項目に修正・文書化・承認済み defer と検証証拠の対応がある。
- [x] 関連 spec/ADR/design/test-spec が実施内容と一致する。

## Verification

- [x] `mise exec -- cargo test --workspace` が exit 0(対象四 crate を含む)。原文項目に対応した回帰テストを含む。
- [x] `mise exec -- cargo fmt --all -- --check` と `mise exec -- cargo clippy --workspace --all-targets -- -D warnings` が exit 0(mise fmt/lint と同一コマンド)。
- [x] 文書と Issue 項目の対応を review する。

実装・文書化・承認済み defer の結果と検証証拠は下記の通り。

## Implementation Inventory (2026-10-11)

Issue Inventory を正本として各項目を照合する。既存 golden の数値・許容誤差は変更しない。

| 原文項目 | 対応 | 証拠 |
| --- | --- | --- |
| 奇数 LIST | 修正: `push_list` は type と body を組み立て、共通 `push_chunk` でサイズ外の word padding を出力する | `odd_list_pads_outside_declared_size_before_next_chunk`: サイズ=5 の LIST の次チャンク位置が red → green |
| stream 0..=99 | 修正: `write_avi` の入口で packet stream の範囲を強制する | `stream_99_is_decimal_and_stream_100_is_rejected`: 99wb と 100 の panic を固定、red → green |
| index フラグ | 承認済み修正: idx1 を出力しない writer の AVIF_HASINDEX を解除した | `avi_without_index_does_not_claim_hasindex`: flags=16 の red → green |
| DIB stride | 承認済み修正: 未パディング BGR24 は width*3%4==0 に限定し明示・強制する | `unpadded_dib_requires_word_aligned_rows`: width=3 の未拒否で red → green、整列幅の biSizeImage=24 を維持 |
| 周期ソフトピン | 承認済み修正: 現行の曖昧性拒否を固定し、非周期対照の受理を必須にする | 受理必須 assertion は現行 accepted=false / ambiguous_alignment / agreeing_methods=1 で red。承認後は拒否分類・full_ncc の複数位相整合候補・非周期対照の 6000±64 samples での受理を固定して green |
| 一時ディレクトリ残存 | 修正: `al-testkit::temp::TempDir` を作成直後から所有し、media/locate/analysis golden の成功時のみの cleanup を置換する | `directories_are_removed_on_success_and_assertion_failure`: ネストしたファイルを含む成功・assertion panic の両方で回収。media golden 12 件、analysis golden 19 件、locate golden 16 件が通過 |
| source_sha256 | 文書化: analysis の aa/bb は識別用ダミーでファイル/PCM hash ではない。locate は未設定。両者は decoded_pcm を直接登録し、原入力来歴は ingest 結合テストの責務 | 両 golden ファイルのモジュール注記。既存 hash 値と未設定状態を維持 |
| rcFrame | 承認済み修正: video はストリームの strf 寸法、audio はゼロを出力する | `video_rcframe_uses_stream_dimensions_and_audio_is_empty`: video のゼロ矩形で red → green。avih と異なる stream 寸法を固定し audio はゼロ |
| expensive の四つの defer | 承認済み defer・文書化: 既存 discovery の defer を再開せず、spec に理由・再検討条件・codec/timing family の受け皿と follow-up Issue/task の着手条件を記録した | `docs/discovery/expensive-family-gaps.md` の四項目と照合。実装再開には別の仕様判断を要する。四項目を実装済みとは扱わない |

### Environment

`mise install --locked python uv pipx:apm-cli pipx:serena-agent` と `mise run setup` は exit 0。setup 由来の範囲外の apm.lock.yaml 更新は戻した。外部依存は変更していない。

### Decisions

coordinator は blocking question `msg_309723cfd7a8` への回答で task 分割なしの棚卸し単位対応、index 解除、DIB 制約、strf 由来 rcFrame、四つの defer 継続を承認した。周期については `msg_d61ffd62c42c` への返信 `msg_f962bc571552` で「曖昧性拒否を固定・非周期対照を受理必須」を承認した。spec/ADR の status と共有 AC は変えず、過去の draft/承認待ちの説明は文書整備時点の記録として保持する。

### Verification Results

- `mise exec -- cargo test -p al-testkit`: exit 0、24 tests。AVI 契約の追加 5 tests は変更前に全て red、修正後に green。cleanup の成功・assertion unwind も回収を確認した。
- 周期テストの対照結果を一時的に `accepted=false` に置換した mutation run は `expected accepted result` で失敗。置換を戻した最終コードの workspace run で同テストが green。周期の拒否と非周期の受理を同じテストで検証し、全拒否による偽成功を防ぐ。
- 初回 workspace run は registry の 3 fixtures が DIB 幅制約で失敗した。幅 2 の padded 行を幅 4 の未パディング frame に合わせ、payload 破損ケースは 1-byte body のまま維持した。既存 assertion は変更せず `cargo test -p al-media --test registry` が exit 0、10 tests。
- 最終 `mise exec -- cargo test --workspace`: exit 0、426 tests (36 test groups)。media/locate/analysis golden は各 12/16/19 tests。既存 golden の数値と許容誤差は維持した。
- 最終 `mise exec -- cargo fmt --all -- --check`: exit 0。
- 最終 `mise exec -- cargo clippy --workspace --all-targets -- -D warnings`: exit 0。
- Issue Inventory 全 9 項目、AC-30、spec/ADR/design/test-spec、defer 4 項目の理由と受け皿を文書監査し、task/index の done 同期と `git diff --check` を確認した。

今回の作業範囲の未対応項目はない。defer 四項目は承認済みの未実装 backlog として残り、再開時に別 Issue/task の計測契約と fixture を承認する。
