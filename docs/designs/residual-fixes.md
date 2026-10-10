---
id: "DESIGN-034DlyKvtWhb8oy1WLAsPV"
type: "design"
status: "approved"
title: "残存課題の責務境界と検証設計"
created: "2026-10-10"
updated: "2026-10-11"
owners: ["scarecrow173"]
relations:
  source: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/overview.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/residual-compliance.md"
    - "docs/adr/residual-issue-boundaries.md"
  derived-by:
    - "docs/plans/residual-fixes.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/28"
    - "https://github.com/scarecrow173/audio-lens/issues/29"
    - "https://github.com/scarecrow173/audio-lens/issues/30"
    - "https://github.com/scarecrow173/audio-lens/issues/31"
    - "https://github.com/scarecrow173/audio-lens/issues/32"
    - "https://github.com/scarecrow173/audio-lens/issues/33"
  defers: []
  deferred-by: []
---

# 残存課題の責務境界と検証設計

## Context

#28〜#33 の共通の完了条件は、計測の意味と来歴を守り、仕様との残差を明示することである。spec と ADR の各 1 件を直接の derives-from とし、既存領域仕様は spec 経由で参照する。

### #30 の承認済み具体化 (2026-10-11)

fixture writer が RIFF word padding と packet stream 0..=99 を強制する。
index を出力しない AVI は HASINDEX を立てず、未パディング BGR24 は
DWORD 整列幅に限定する。video の rcFrame は寸法の正本である strf から
導出し、audio はゼロを維持する。周期 fixture の現行 ambiguous_alignment
を明示固定し、同じテストの非周期対照の受理で全拒否への回帰を検出する。
TempDir が作成直後から回収を所有し、構築途中と assertion の unwind を
含むスコープ終了を扱う。合成 hash は原入力の検証証拠と混同しない。
四つの未実装記述子は expensive-analysis spec の defer backlog に保持する。
承認と項目別証拠は fixture-review-fidelity task に記録する。

## Scope

### In Scope

Issue ごとの修正境界と検証設計。#33 の方式判断を実装修正から分離する。

### Out of Scope

未承認の方式決定、依存追加、高速化・デコーダ拡張・ROI 実装。

## Components and Boundaries

| Issue | 所有境界 | 設計方針 |
| --- | --- | --- |
| #28 | parser/clock、metadata、dataset、viz | 有理時計の余り保持、非デコード対象にも構造検証、unavailable の統一表現。codec の厳密判定・冗長 path・timeline 欠落は各所有者で修正する |
| #29 | expensive family と共通 emit | 元チャネル identity を保持して窓を切り、未支持/不良条件を validity に反映する。IR 窓と summary の意味を fixture で固定する |
| #30 | fixture builder とテスト所有者 | 実コンテナ契約に沿った padding/index/stride、強い assertion、失敗時 cleanup、合成来歴の説明を管理する |
| #31 | config 入力と avsync | マーカーの有限時刻と明示クロックを入力境界で検証し、検出イベントと同じ時刻表現へ一箇所で変換して既存照合へ合流する |
| #32 | residual_search と audio store、alignment PCM 読込境界 | 減算済みチャネルを既存 residual ロールで保存し、残差を独立した signal version として child alignment の使用 PCM に明示参照する。search JSON と親 source/alignment の lineage を同時に登録する |
| #33 | stage orchestration と dataset schema | 承認済み: 候補検出を論理段として fused 実行を仕様化、FLAC 保存を defer、Resolution.sample_rate を u32 で第一級化 |

### #28 の実施契約(2026-10-11)

native registry と direct parser は AVI layout / MP4 parse-stbl を共有し、
時計・構造検証を呼出側で補償しない。利用不能 integrity の組立は
`IntegrityObservation::decode_unavailable` に集約し、型付き
`UnavailableKind` を provenance と同じ分類で保持する。
luma は共通特徴抽出から decoder へ二枚の buffer を交互に返して再利用する。
dataset metadata は一パス一 artifact とし、PCM 不在時の役割を
`dataset_input_unavailable` にする。codec は native decoder が出力する
完全名だけを受理する。viz は declared JSON を入力検証証拠にせず、
各入力の Audio または metadata JSON を確認し、timeline がない全経路を
skipped に記録する。

時計・非フレーム packet-slot・dwStart・rec LIST・overflow の判断は
coordinator の question `msg_7c43909b855e` reply で承認済み。
#28 は task 分割せず、項目別証拠と論理別コミットで追跡する。
共通原点の具体式は [video-features.md](./video-features.md)、
MP4 の構造/局所失敗境界は [mp4-ingest.md](./mp4-ingest.md) を参照する。

## Data and Control Flow

#32 は残差を元キャプチャとは別の signal version として保存する方式を採る。
探索に渡した減算後チャネルと同一信号を復元できる PCM artifact を保存し、
child alignment の `capture_pcm` をその artifact に結び付ける。減算前の
元キャプチャ normalized_pcm は来歴であり、child の使用 PCM 参照にしない。
残差 WAV の保存と search JSON からの参照に加え、child alignment と search JSON
の `ArtifactRecord.inputs` に使用 PCM を含め、親 source/alignment、混合行列、
元キャプチャから減算後残差への lineage を辿れるようにする。
`read_alignment_pcm` 等の共有読込境界は残差 signal version の role・形式、
レート、チャネル数、バイト長・フレーム数、区間の整合を検証して探索時の
同一信号を返す。必要な残差形式の読込対応はこの境界に集約し、参照欠落・
不整合はエラーにする。元 PCM への暗黙 fallback や減算の再実行で補償しない。
これは `docs/designs/simd-dsp-foundation.md` の使用 PCM 明示参照・lineage・
fail-closed 再取得契約を残差探索にも適用する設計である。

1. Issue 原文の項目と AC を task に対応付ける。
2. 所有層で入力と状態を検証し、既存の計測・来歴表現を利用する。
3. 正常・無効・欠落の結果を dataset に保存して consumer から辿れるようにする。
4. #33 の判断が承認された後だけ仕様と実装を更新する。

## Non-Functional Constraints

未指定 marker の既存挙動と部分 ingest を保持する。チャネル identity と時計精度を損なわない。形式変更は再生成する。新依存の選定は承認まで保留する。

## Risks and Trade-offs

#28 の時計修正が #31 のイベント差分に影響するため #31 は #28 の完了後に検証する。#33 の schema 判断が #32 の参照構造に影響する場合は再 review し、互換補償を追加しない。

## Deferred Design Concerns

#33 の三つの方式は末尾の承認済み採用設計を参照する。marker の入力位置とクロック契約は承認時に明文化する。draft を実装承認と解釈しない。

## Verification Notes

- [ ] spec の AC-28〜AC-33 が plan/task/test-spec へ対応する。
- [ ] derives-from が 1 spec + 1 ADR のみを指す。
- [ ] Graph は関係欠落なしで承認待ちを返す。

## #33 の採用設計 (2026-10-11)

候補生成 → events 保存 → トリガ評価を `ExpensiveAnalysisStage` 内で維持し、
候補のみの独立再実行は要求しない。FLAC は承認済み defer とし、既存
Float64 WAV の残差と lineage を維持する。`Resolution` は window/hop と
必須の `sample_rate: u32` を一箇所に定義し、RecordBuilder と直接生成箇所は
計算に使用する decoded/normalized PCM レートを渡す。expensive 共通 Emit は
analyze_region のレートを保持して有効・無効の両経路へ伝播する。
互換層や manifest レートへの暗黙 fallback は設けず成果物を再生成する。
[承認・理由・証拠](../tasks/spec-residual-decisions.md) を参照。
