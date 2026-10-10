---
id: "TSPEC-034dLdqZ82hOq5hqHwU8MC"
type: "test-spec"
status: "draft"
title: "残存課題の契約回帰検証"
created: "2026-10-10"
updated: "2026-10-10"
owners: ["scarecrow173"]
relations:
  source: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/residual-compliance.md"
    - "docs/designs/residual-fixes.md"
    - "docs/adr/residual-issue-boundaries.md"
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

# 残存課題の契約回帰検証

## Purpose

#28〜#33 の観測・時計・妥当性・来歴の契約を、task 分割後も保持する。実装済みという主張ではない。

## Feature

部分 ingest と解析成果物が利用不能・無効・有効を正確に区別し、仕様との残差を追跡する。

## Rules

- AC-28: 未整列 AVI 音声パケットの時刻が累積有理時計と一致する。MP4 の非デコードトラックにも同じテーブル検証が適用される。音声が unavailable でも入力ハッシュと利用不能理由が残る。luma の再利用、非フレーム時刻、dwStart、OpenDML、integrity 名称、二次音声、欠落 timeline の扱いを各境界ケースで検証する。
- AC-29: 不均一チャネル長で panic とチャネル誤ラベルが起きない。ill-conditioned 帯域の係数にも妥当性が反映され、支持数ゼロや候補なしの測定を NaN/0 + Valid として出さない。IR の窓、ペア summary のチャネル意味、暗黙スキップとコメントの整合を fixture で検証する。
- AC-30: 奇数 LIST、stream 99/100、index フラグ、DIB stride と rcFrame が生成するメディアの契約と一致する。周期 fixture は全拒否で成功しない。失敗時も一時ディレクトリが回収され、合成 source hash の意味と defer 項目の受け皿が明記される。
- AC-31: ユーザー供給 visual marker が avsync のペア出力に反映される。未指定時の検出イベント経路と出力は変化しない。有限時刻・クロック・入力来歴を検証し、ゲーム固有 detector を core に追加しない。
- AC-32: two-source 探索の残差 PCM を保存して再読出し・再聴取できる。search JSON から音声 artifact と親 source/alignment、混合行列の来歴を辿れ、保存前後のサンプル・エネルギーが一致する。残差 child alignment の `capture_pcm` が減算後残差の signal version を明示参照し、dataset 再 open 後の `read_alignment_pcm` 等による PCM 再取得が探索に使った信号と一致する。元キャプチャとの違いも assertion で確認し、参照欠落・不整合では元 PCM に fallback せずエラーになる。
- AC-33: 候補検出の独立実行、FLAC 保存、Resolution.sample_rate の各項目に、既存 spec の該当文言、実装との差、選択肢、承認結果、実装または仕様改訂先を記録する。判断前に現状を適合済みとせず、選択した契約の回帰テストを承認後に追加する。

## Examples

| Issue | Given / When | Then |
| --- | --- | --- |
| #28 | 未整列 auds を多数連結、非デコード MP4 の stsc/stco を破損 | 余りの累積時計を維持、破損構造の結果が codec に依存しない |
| #29 | ch0 が窓始点より短く ch1 のみ支持、条件数 > 1e3、splice count=0 | panic なし、ch1 identity を保持、係数 demote、無支持を Valid にしない |
| #30 | 奇数 LIST と stream 99/100、周期 fixture の全拒否、テスト失敗 | padding/境界の契約を守り、全拒否では失敗、cleanup が実行される |
| #31 | 有限 marker と既知音声 onset を指定、別 run では未指定 | 指定 marker の差分が出る、未指定 run の既存結果と一致 |
| #32 | 既知の二音源を減算して元キャプチャと異なる残差で child を探索し、dataset を再 open して child alignment から PCM を再取得。別ケースで残差参照を欠落・不整合にする | 保存 residual と再取得した capture PCM が探索に使った減算信号と一致、元キャプチャとは異なる。search JSON と child alignment から使用 PCM と親の lineage を辿れ、参照欠落・不整合はエラー |
| #33 | 三項目の採択記録と既知 fixture | 採択した契約を個別の assertion で固定、未決定を適合済みとしない |

## Guarantees

task の原文 inventory 全項目をケースに対応付ける。上表は代表例であり全項目の代替ではない。#33 の具体的 assertion は選択を承認した後に確定し、仕様改訂を選ぶ場合も対応文書監査を行う。

## Non-goals

新コーデック、SIMD 性能、ゲーム固有 detector、未承認の方式選定。

## Risk

検証を省くと信号欠落、時計ドリフト、チャネル誤ラベル、無効値の Valid 化、fixture 偽成功が回帰する。
