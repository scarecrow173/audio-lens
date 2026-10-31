---
id: "PLAN-034P3zvTd9Bg7Ps3kxAKOd"
type: "plan"
status: "approved"
title: "Ruviform 基盤移行の段階実施計画"
created: "2026-10-30"
updated: "2026-10-31"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/95"
    - "https://github.com/scarecrow173/audio-lens/issues/97"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/ruviform-foundation-migration.md"
    - "../adr/ruviform-foundation-migration.md"
  implemented-by: []
  depends-on:
    - "../designs/ruviform-migration-responsibility-map.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/ruviform-migration-responsibility-map.md"
    - "../adr/ruviform-foundation-migration.md"
    - "../specs/ruviform-foundation-migration.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/54"
    - "https://github.com/scarecrow173/audio-lens/issues/60"
    - "https://github.com/scarecrow173/audio-lens/issues/71"
    - "https://github.com/scarecrow173/audio-lens/issues/76"
  defers: []
  deferred-by: []
---

# Ruviform 基盤移行の段階実施計画 Implementation Plan

## Goal

ADR `../adr/ruviform-foundation-migration.md` と責務マップ
`../designs/ruviform-migration-responsibility-map.md` に従い、
Ruviform への基盤移行を段階ゲート・比較/rollback 手順・二重実装
撤去条件つきで実施する順序を固定する。各段階の実装本体は対応する
GitHub Issue(AL#98〜#106、R#150〜#157)が担い、本 plan は
「どの順で・何をゲートに・どう戻すか」の正本である。

## Scope

単一の実施計画として扱う。各段階は別リポジトリを跨ぐが、同一の
クリティカルパスと同一の移行完了条件を共有するため分割しない。
実装単位の正本は GitHub Issue(#98〜#106、#54)であり、
`docs/tasks/` 配下の task doc はそれらへの薄いリンクと依存辺のみを
持つ(内容の二重管理を避け、機械可読な DAG を供給する)。

## Upstream Documents

- `../specs/ruviform-foundation-migration.md`(要求仕様 SPEC-034P3nkrA1BoZbYjM3OrQ3)
- `../adr/ruviform-foundation-migration.md`(移行方針 ADR-034P3nB0CZJO83ZX105Cfq)

## Design Inputs

- `../designs/ruviform-migration-responsibility-map.md`
  (approved 責務マップ DESIGN-034P3nlejLy074yTmYSAcJ)

## File Map

移行対象の責務単位と所在。ファイル単位の変更一覧は各切替 Issue が
詳細設計で確定する。

| Path / 責務単位 | 移行後の所在 | 担当 Issue |
| --- | --- | --- |
| `crates/al-math` 全カーネル(fft/correlation/filter/resample/vec/linalg) | Ruviform #150/#152/#153。切替後 AL 側は撤去 | R#150-153、AL#101、撤去 AL#104 |
| `crates/al-media` 汎用 I/O(wav/avi/dib/mp4/symphonia/decoder/timing 観測原材料) | Ruviform media-io + backend(#151/#154)。AL 側は ingest adapter に縮小 | R#151/#154、AL#100、整理 AL#104 |
| `crates/al-locate` 意味論(methods/consensus/tracking/mixer/residual 等) | **残す** al-locate。`dsp.rs` 呼出先のみ Ruviform 化 | AL#101/#102 |
| `crates/al-analysis` 意味論(cheap/reference/expensive/records/events 等) | **残す** al-analysis。`stft.rs` の数値部のみ Ruviform 化 | AL#101/#102 |
| `crates/al-job` 不変 Job・StageRunner・Manifest | **残す** al-job | —(回帰 AL#103) |
| `crates/al-core` 語彙型 | **残す** al-core。`StageKind::Visualization` は AL#106 で撤去 | AL#106 |
| `crates/al-dataset` + 可搬 export(旧 `al-viz::package`) | **残す** al-dataset + export 再配置 | AL#103/#106 |
| `crates/al-viz` 表示専用部 | **廃止** AL#106 | AL#106 |
| `crates/al-observability` | **残す**(委譲判定は AL#104) | — |
| `crates/al-cli` | **残す**。al-service 経由へ再配線、export コマンド追加 | AL#105/#106 |
| `al-service`(仮称・新設) | **新設** AudioLens。headless 解析サービス | AL#105 |
| `al-gui`(仮称・新設) | **新設** AudioLens。Ruviform GUI host を利用 | AL#105 |

## Implementation Sequence(依存 DAG)

ノードは Issue。`A ← B` は「A は B の完了に依存」。同一段内の
項目は並行可能。

```text
[段0 設計] AL#97(本 plan/ADR/責務マップ) ── 全 AL 側作業の前提
   │
   ├─> [段1 前提] AL#98 baseline(Golden/実サンプル/性能固定)
   │              AL#99 依存・CI(Ruviform SHA 固定・toolchain・GUI/CLI 依存分離)
   │              ※ #98 と #99 は相互独立、どちらも #97 直後に着手可
   │
   ├─> [段2 Ruviform 整備]
   │      R#150 f64 基盤 ──┬─> R#152 相関 ─┐
   │                      └─> R#153 帯域・STFT ─┤
   │      R#151 観測契約 ──> R#154 codec backend
   │      R#156 GUI host ──> R#157 表示 primitive(数値部は R#150/#153 も利用)
   │      R#155 CLI/headless(R#150〜#154 の公開 API を利用)
   │
   ├─> [段3 AL 切替]
   │      AL#100 Media I/O 切替  ← AL#97・AL#98・AL#99・R#151・R#154
   │      AL#101 f64/DSP 呼出切替 ← AL#97・AL#98・AL#99・R#150・R#152・R#153
   │      AL#102 固有/汎用分離   ← AL#97・AL#98・AL#101・(R#152/#153 契約)
   │
   ├─> [段4 回帰] AL#103 Job・Dataset・Provenance・query 回帰
   │              ← AL#100・AL#101・AL#102
   │
   ├─> [段5 GUI] AL#105 GUI アプリ ← AL#97・AL#99・AL#103・R#155・R#156・R#157
   │
   ├─> [段6 撤去] AL#106 旧 viz 廃止・export 分離
   │              ← AL#105 検証済み・AL#103・(R#156/#157 利用側実績)
   │
   └─> [段7 完了] AL#104 旧基盤撤去・最終検証 ← 全先行完了ゲート
```

独立評価: AL#54(stage 内並列化)は AL#99 後に `ruviform-exec`
OfflineDriver の適合性を実測評価して独立に実施。AL#60/#71/#76 は
切替前後の性能比較対象として参照する(#98 baseline で before/after
を固定)。

クリティカルパス:
`AL#97 → R#150 → R#152/#153 → AL#101 → AL#102 → AL#103 → AL#105 → AL#106 → AL#104`

### 各段のゲート

| 段 | 通過条件(完了ゲート) |
| --- | --- |
| 段1 | #98: 再現可能な入力一覧・golden・性能 baseline 確定。#99: 固定 SHA から再現可能・CI/private access・GUI/CLI 個別ビルド |
| 段2 | 各 Ruviform Issue の受け入れ条件(契約文書化・scalar 参照差分・能力表・利用例) |
| 段3 | #100: 観測情報損失なし・段階別切替(WAV→AVI/MP4+AAC→動画/複数 stream)。#101: f64 精度・意味維持・性能比較記録。#102: 固有判断が AL に残り Ruviform 型に混入なし |
| 段4 | #103: ingest〜export 実ジョブ・再開・provenance(Ruviform revision/backend/params)・永続 Dataset が再解析なしに範囲読出し・絞込みできる reader/query port 前提が検証される(GUI 実側の確認は #105 で実施) |
| 段5 | #105: GUI が稼働・同一 Dataset を再解析なしで読める・GUI/CLI 同一結果・長尺データの応答性計測(応答性は #105 task で定義した目標基準を満たすこと。かつ #106 への移行判断は実使用者/レビュアによる明示的な受入確認をゲートとする)・GUI 依存が headless に逆流しない |
| 段6 | #106: 旧 viz 表示専用部削除・可搬 export が独立動作・StageKind/CLI/ドキュメント整合 |
| 段7 | #104: 未使用 crate/重複実装の撤去・性能/精度最終確認・依存/ライセンス/CI 監査・クロスリポジトリ Issue/ADR/DAG 最新 |

## 比較・rollback 手順

### 切替単位の比較手順(全切替共通)

1. #98 で固定した commit SHA・toolchain・入力 hash・設定・golden・
   計測手順を before 側として使用する。
2. 切替対象の処理単位ごとに、同一 fixture/形状で旧実装と新実装を
   実行し、Golden・数値差分・wall/busy-time・メモリ/alloc・I/O・
   生成物サイズを記録する。
3. 差分の評価は #98 が区分した「bit-exact 維持契約」と「浮動小数
   誤差許容契約」に従う。bit-exact 契約違反は即 blocker。
4. 暫定 adapter/比較フラグは目的・削除条件・関連 Issue をコード内
   と Issue に明記し、撤去対象として AL#104 に連携する。
5. 性能退行がある場合は現行を無条件削除せず、Ruviform 側の問題と
   して Issue 化し、対処後に切替する(AGENTS.md 規約)。
6. #60/#71/#76 等の同領域変更が #98 固定 SHA と切替の間に入った
   場合、その landing 時点で baseline を再固定し、差分は
   fixture+commit で帰属を分離する。

### rollback 手順

- **切替段階(#100/#101)**: 切替は処理単位の呼出差替として行い、
  ゲート未達の段階では旧実装呼出に戻す。二重実装が残るのは移行
  期間のみで、撤去条件(下記)を満たすまで旧実装を削除しない。
  rollback の検証: 旧実装呼出で #98 golden が再現できること。
- **責務分離(#102)**: 意味論リファクタの退行は Golden 差分で
  検出し、該当変更を revert。再開/manifest に影響する変更は
  #103 の回帰を再実行して確認。
- **GUI/撤去(#105/#106)**: #106 は「新 GUI が #105 完了条件を
  満たしてから」実施する前提。撤去前に比較用の参照資料(旧静的
  出力・Golden)を保存し、撤去後は再生成でしか旧成果物を復元
  しない(Pre-release 規約)。
- **撤去のコード rollback(#106/#104)**: コードレベルの rollback
  は撤去コミット群の git revert とし、#103 相当の回帰を再実行
  して検証する。上記の「再生成」は成果物レベルの復元手順であり、
  コードの rollback とは区別する。
- **Ruviform 側の問題発見時**: AL 側で例外化・恒久 adapter を
  作らず、Ruviform Issue に起票し相互参照する。切替はその Issue
  の解決待ちで保留できる。

### 二重実装撤去条件(AL#104 の正本)

次を全て満たした処理単位の旧実装のみ撤去できる:

1. 対応する Ruviform 側 API/backend が該当 Issue の受け入れ条件を
   満たして存在する。
2. AL 側の全利用箇所が新実装へ切替済みで、当該処理の Golden・
   性能比較に説明不能な差・未承認の退行がない。
3. 暫定 adapter/比較フラグに付記した削除条件が消えている
   (検証は削除せず適切な側へ移す)。
4. 例外として残す場合は Issue と理由を持つ(#104 完了条件)。

crate 全体の撤去は残責務次第: `al-math` は全カーネル移行で溶解
候補、`al-media` は ingest adapter 責務が残れば縮小存続、
`al-viz` は export 再配置後に全撤去、`al-core`/`al-dataset`/
`al-job`/`al-cli`/`al-observability`/`al-testkit` は存続。

## Dependencies and Risks

- クリティカルパス上の Ruviform Issue(#150/#152/#153)の遅延は
  AL#101 以降を直撃する。段2 の Ruviform 整備を先行させる。
- #151 の観測契約不足は #100 をブロックする。責務マップの
  「API 意味」列を #151 の要件として早期照合する。
- 暫定 adapter/比較フラグの残骸リスク: 撤去条件を付記しない一時
  実装を禁止し、#104 で残存を洗い出す。
- GUI 依存の headless 逆流リスク: #99 で CLI/headless と GUI の
  個別ビルド/テストを CI に入れ、#105/#106 で検証する。
- Private repo 依存の再現性リスク: #99 で最小権限 credential・
  SHA/lockfile 固定を完了させてから切替に入る。

## Task Breakdown

実装タスクの正本は GitHub Issue(#98〜#106、R#150〜#157、#54)。
`docs/tasks/` には各 Issue への薄いリンクと task 間依存辺を持つ
task doc を置き、Task Graph が上記 DAG を機械可読に投影する。

| Task doc | Issue | 主な依存(task 辺) |
| --- | --- | --- |
| `docs/tasks/ruviform-migration-baseline.md` | AL#98 | — |
| `docs/tasks/ruviform-dependency-ci.md` | AL#99 | — |
| `docs/tasks/ruviform-media-io-switch.md` | AL#100 | #98・#99(+R#151/#154) |
| `docs/tasks/ruviform-dsp-kernel-switch.md` | AL#101 | #98・#99(+R#150/#152/#153) |
| `docs/tasks/ruviform-domain-kernel-separation.md` | AL#102 | #98・#101(+R#152/#153) |
| `docs/tasks/ruviform-job-dataset-regression.md` | AL#103 | #100・#101・#102 |
| `docs/tasks/ruviform-gui-app.md` | AL#105 | #99・#103(+R#155/#156/#157) |
| `docs/tasks/ruviform-viz-retirement.md` | AL#106 | #103・#105(+R#156/#157) |
| `docs/tasks/ruviform-foundation-cleanup.md` | AL#104 | #106(全先行を推移包含) |
| `docs/tasks/ruviform-exec-parallel-eval.md` | AL#54 | #99・#98(性能計測は #98 baseline を基準。独立評価) |

本 plan が各 Issue に求める引き継ぎ事項:

- AL#98: baseline 区分(bit-exact/許容)と計測手順を本 plan の
  比較手順 1〜3 に使える形で固定する。
- AL#99: Ruviform crate の SHA 固定・GUI/CLI 依存分離 CI を段1
  ゲートとして提供する。
- AL#100/#101/#102: 責務マップ対応行の切替を実施し、比較手順
  1〜5 の記録を残す。
- AL#103: 段4 ゲートの回帰項目を検証する。
- AL#105/#106: 段5/段6 ゲートと撤去条件を実行する。
- AL#104: 二重実装撤去条件 1〜4 と最終検証を担う。

## Verification

- [ ] 本 DAG の全ノードが Issue 番号と対応し、依存宣言と矛盾が
      ない(各 Issue 本文の依存節と相互照合済み)。
- [ ] 段ごとのゲートが各 Issue の完了条件に対応する。
- [ ] rollback/比較手順が #98 の baseline 契約に接続されている。
- [ ] 二重実装撤去条件が #104 の最終ゲートと一致する。
- [ ] review-protocol で CONFIRMED blocker が残っていない。

## Open Questions / Gaps

- `al-service`/`al-gui` の正式名・配置は #105 の詳細設計で確定。
  本 plan では仮称で統一。
- `ruviform-exec` OfflineDriver の採否は #54 の実測次第。本 DAG
  では独立評価として扱い、クリティカルパスに含めない。
- Ruviform 側 Issue の詳細設計で責務マップとの差異が出た場合、
  本 plan ではなく対応 Issue 側で解決し、本 plan を更新して追跡
  可能性を維持する。

## Self-Review

- [x] 要求仕様 R5/DAG・R6/headless・R7/viz 撤去が段とゲートへ対応。
- [x] プレースホルダーなし。
- [x] ファイル・関係・status・用語の一貫性(仮称は明示)。

## Implementation Handoff

- Recommended mode: delegated 実装(各段は独立 Issue として別
  worktree/dispatch に委譲可能。段内の並行可能項目のみ並列化し、
  段間はゲート順守)。
