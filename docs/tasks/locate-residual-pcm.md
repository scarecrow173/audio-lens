---
id: "TASK-034dLdqYvyaMjl7hTgg6IT"
type: "task"
status: "done"
title: "al-locate: 2ソース残差探索の残差PCMを保存する"
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
    - "https://github.com/scarecrow173/audio-lens/issues/32"
  defers: []
  deferred-by: []
---

# al-locate: 2ソース残差探索の残差PCMを保存する

## Work

#32 の残件を以下の原文棚卸しに沿って確認する。Issue 全体を対応単位とし、修正・文書化・承認済み defer の各結果と証拠を項目ごとに残す。複数の独立修正を実施するときは承認後に task を分割し、この対応を保持する。

## Classification

- `new-feature`
- 承認済み design/plan/test-spec と LEFT-R1 の使用 PCM 契約に基づく実装。

## Upstream Documents

- `docs/plans/residual-fixes.md`
- `docs/specs/residual-compliance.md` AC-32
- `docs/designs/residual-fixes.md`
- Issue #32

## Dependencies

spec/ADR はチェーンの受理可能な status を満たし、design/plan と test-spec は承認済みのため status todo。

## Affected Files

- `crates/al-locate/src/stage.rs`
- `crates/al-locate/src/residual.rs`
- `crates/al-locate/src/decode.rs`
- `crates/al-locate/Cargo.toml`, `Cargo.lock`
- `crates/al-dataset/src/audio.rs` の既存 API を再利用(変更なし)
- `crates/al-locate/tests/locate.rs`
- `docs/tasks/README.md`

## Issue Inventory (2026-10-10)

### 概要

ロードマップ監査(#5 照合)で検出。`al-locate` の two-source 残差探索
(`crates/al-locate/src/stage.rs` の `residual_search` 呼び出し)は
`residual_<parent>.json` として以下を保存している:

- parent_source / parent_alignment / child locate result
- channel_transform(符号付き混合行列)
- 削減前後エネルギー / confidence

しかし減算後の**残差 PCM 自体は保存されず破棄**される。issue #5 の
「残差は反復(iteration)とレビューのために保持する」要件に対し、
メタデータは満たすが残差信号の再聴取・再解析ができない。

reference 解析側では staged residual R0–R3 の音声が `residual` ロールに
保存済み(`crates/al-analysis/src/reference.rs`)なので、保存機構は
既存の `residual` ロール + audio store を再利用する形で足せる。

### スコープ

- residual_search 後の残差 channel samples を `residual` ロールの
  WAV/audio artifact として保存(親ソース ID 連動の命名)
- search JSON から残差 audio artifact への参照を lineage/params に追加
- 残差 child alignment は探索に実際に使用した減算後残差 PCM を
  独立した signal version として保存し、`capture_pcm` で明示参照する。
  `read_alignment_pcm` 等の再取得経路が探索時と同一信号を返すようにし、
  減算前の元キャプチャ normalized_pcm を使用 PCM として参照しない。
  参照欠落・不整合はエラーとし、元 PCM への暗黙 fallback を禁止する
- 必要なら残差の簡易スペクトル特性(PSD/帯域)を observation として併記

### 参照

- 監査対象: issue #5(残差検索・混合フィット)
- 実装: crates/al-locate/src/stage.rs, crates/al-locate/src/residual.rs
- 既存機構: crates/al-dataset/src/audio.rs, al-analysis residual 保存

## Steps

1. 原文の各項目を実装と既存 spec に照合し、修正か判断かを記録する。
2. 違反する契約を固定する回帰テストを追加する。
3. 対象テストを実行し、期待する失敗を確認する。
4. 所有境界で修正する。判断項目は承認後に着手する。
5. 対象検証を再実行する。
6. 項目ごとの証拠と文書を更新する。
7. 意味のある変更単位でコミットする。

## Done When

- [x] two-source 探索の残差 PCM を保存して再読出し・再聴取できる。search JSON から音声 artifact と親 source/alignment、混合行列の来歴を辿れ、保存前後のサンプル・エネルギーが一致する。
- [x] 残差 child alignment の `capture_pcm` が探索に実際に使用した減算後残差の signal version を明示参照する。dataset 再 open 後も `read_alignment_pcm` 等が探索時と同一信号を返し、参照欠落・不整合では fail-closed でエラーになる。
- [x] 原文全項目に修正・文書化・承認済み defer と検証証拠の対応がある。
- [x] 関連 spec/ADR/design/test-spec が実施内容と一致する。

## Verification

- [x] `mise exec -- cargo test -p al-locate -p al-dataset` が exit 0。原文項目に対応した回帰テストを含む。
- [x] `mise run fmt` と `mise run lint` が exit 0。
- [x] 文書と Issue 項目の対応を review する。

## 実施結果と証拠 (2026-10-10)

| 原文項目 / 契約 | 実施結果 | 検証証拠 |
| --- | --- | --- |
| 減算後 channel samples の保持・再聴取 | `al-dataset::write_wav` を再利用し、親ごとに `locate/residual/<parent_source>_<artifact_id>.wav` を Float64 / residual ロールで保存。全 child で共有する | `residual_child_reopens_the_exact_subtracted_signal_and_lineage` の二音源・2チャネル fixture で保存前後のサンプルと RMS が完全一致 |
| search JSON と親 source/alignment・混合行列の来歴 | JSON の `residual_pcm` と artifact inputs、WAV params の親 alignment artifact・source/capture PCM・符号付き行列・エネルギーを登録 | 同テストで JSON から WAV と親 locate_result を辿り、行列を照合 |
| LEFT-R1: child 使用 PCM の明示参照 | child の `capture_pcm` を残差 artifact に結び付け、`residual_parent_alignment` で親を明示。`read_alignment_pcm` の共有境界で WAV と来歴を検証し、保存済み信号を復元 | manifest をディスクへ保存・再読出し後、探索に渡した減算信号と完全一致し、元キャプチャと異なることを assertion |
| fail-closed | 参照欠落、元キャプチャへの差し替え、role/形式・rate/channels/frames・行列・親 alignment・inputs の不整合、WAV 破損/欠落をエラーとする | 同テストの参照欠落/差し替え、metadata 変異、inputs 欠落、WAV 破損/欠落ケース |

回帰テストは修正前に child の使用 PCM が `normalized_pcm` であるため失敗し、修正後に成功した。
対象テストは合計 93 件成功、`mise run fmt` と workspace 全 targets の
`mise run lint`(`cargo clippy --workspace --all-targets -- -D warnings`)も成功。
内部依存 `al-dataset` と `Cargo.lock` 更新は coordinator の明示承認を得た。
spec AC-32、design の独立 signal version / lineage / fail-closed 契約、test-spec AC-32 と対応し、上流の status は変更していない。

## Non-blocking

- 簡易 PSD/帯域 observation は任意項目であり、保存・再聴取・使用 PCM 再取得の AC-32 に必要ないため追加しない。再検討条件は残差のスペクトル比較が必要になったとき。
- WAV は既存 RIFF writer のサイズ制約に従う。上限を超える保存はエラーとし、別形式や大容量コンテナの導入は本 task の範囲外。
- JSON の浮動小数メタデータは parse 時の丸めを許容して照合し、PCM 自体の保存・再取得は Float64 の完全一致で検証した。
