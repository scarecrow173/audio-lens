---
id: "TASK-034dLdqYk3faqjkGcbspVS"
type: "task"
status: "blocked"
title: "al-media: コンテナ解析の残存エッジケース(PR #25/#27 レビュー LOW)"
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
    - "https://github.com/scarecrow173/audio-lens/issues/28"
  defers: []
  deferred-by: []
---

# al-media: コンテナ解析の残存エッジケース(PR #25/#27 レビュー LOW)

## Work

#28 の残件を以下の原文棚卸しに沿って確認する。Issue 全体を対応単位とし、修正・文書化・承認済み defer の各結果と証拠を項目ごとに残す。複数の独立修正を実施するときは承認後に task を分割し、この対応を保持する。

## Classification

- `bug-fix`
- draft plan のレビュー用候補。依頼により先に文書化するが、通常の承認済み計画由来の実装 task ではない。

## Upstream Documents

- `docs/plans/residual-fixes.md`
- `docs/specs/residual-compliance.md` AC-28
- `docs/designs/residual-fixes.md`
- Issue #28

## Dependencies

spec/ADR と design/plan の承認を待つため status blocked。承認前は実行しない。

## Affected Files

- `crates/al-media/src/avi.rs`
- `crates/al-media/src/mp4.rs`
- `crates/al-media/src/video.rs`
- `crates/al-media/src/metadata.rs`
- `crates/al-dataset/src/stage.rs`
- `crates/al-viz/src/`

## Issue Inventory (2026-10-10)

### 背景

PR #25(`feat(al-media)`)と PR #27(`fix(dataset,viz)`)のレビューで検出された非ブロッカー指摘。部分 ingest 契約の境界は確立済みだが、以下の精度・整合性改善が残る。

### 項目

- `avi.rs` `tick_advance`: `len*scale/sample_size` をパケットごとに切り捨てるため、`dwSampleSize` 未整列ボディでタイムラインが最大 ~1 tick/パケットドリフトする(余りを繰り越すべき)
- `mp4.rs`: `stsc`/`stco` の妥当性は PCM 抽出時にしか検証されない。非デコード対象トラックのテーブル破損は未検証のまま通る(コーデックによって fatal か否かが変わる)
- `video.rs`: スクラッチ luma プレーンが `mem::take` で毎フレーム空になり `resize` で再確保される。「フレーム間再利用」のコメントと実装が食い違う
- `avi.rs`: 非フレーム `vids` パケット(`pc` 等)が `dwScale` タイムラインスロットを消費するため、`movi_order` 時刻がフレーム index×scale の意味論とずれる。意図的だがドキュメント化が必要
- integrity kind の命名ドリフト: `audio_decode_unavailable`(AVI)/ `decode/input unavailable`(MP4)/ `video_decode_unavailable` — 1 概念に 3 表記
- avsync の `dwStart` リベースは video 側のみ。`auds` の非ゼロ `dwStart` は相殺されない(設計注記はあるが `dwStart` が意味を持つ入力で実オフセットを捨てる)
- OpenDML AVI(`movi` 内 `LIST 'rec '`)はパースできるがパケット 0 で全 unavailable になり、「OpenDML 非対応」の明示理由がない
- `audio: null` のとき `metadata.json` の source_hash がファイルハッシュを失う(issue #3 の要求メタデータ)
- `dataset_input_unavailable.path` がメタデータ成果物パスを冗長に複製
- `encoding_for` の接頭辞一致が手編集された不正 codec 名を許容しうる
- viz の `expect("inputs")` は declared.json が常に JSON のため実質空のチェック
- AVI のセカンダリ `auds` は MP4 のセカンダリ `soun` より観測情報が少ない
- capture/input 不在時の timeline 欠落が他のスキップと違い記録されない
- 病的ヘッダの checked 時計算術が `Malformed` へエスカレートする境界の妥当性確認

Refs #25 #27

## Steps

1. 原文の各項目を実装と既存 spec に照合し、修正か判断かを記録する。
2. 違反する契約を固定する回帰テストを追加する。
3. 対象テストを実行し、期待する失敗を確認する。
4. 所有境界で修正する。判断項目は承認後に着手する。
5. 対象検証を再実行する。
6. 項目ごとの証拠と文書を更新する。
7. 意味のある変更単位でコミットする。

## Done When

- [ ] 未整列 AVI 音声パケットの時刻が累積有理時計と一致する。MP4 の非デコードトラックにも同じテーブル検証が適用される。音声が unavailable でも入力ハッシュと利用不能理由が残る。luma の再利用、非フレーム時刻、dwStart、OpenDML、integrity 名称、二次音声、欠落 timeline の扱いを各境界ケースで検証する。
- [ ] 原文全項目に修正・文書化・承認済み defer と検証証拠の対応がある。
- [ ] 関連 spec/ADR/design/test-spec が実施内容と一致する。

## Verification

- [ ] `mise exec -- cargo test -p al-media -p al-dataset -p al-viz` が exit 0。原文項目に対応した回帰テストを含む。
- [ ] `mise run fmt` と `mise run lint` が exit 0。
- [ ] 文書と Issue 項目の対応を review する。

現時点では文書整備のみであり、上記実装検証は未実行。
