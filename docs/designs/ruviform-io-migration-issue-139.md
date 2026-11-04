---
id: "DESIGN-034RN2eWIExEejH7mYYOyj"
type: "design"
status: "draft"
title: "Issue #139 Ruviform 共通 I/O 移行の残存分類 — StagedFile 移行記録・回帰契約・Structured/Archive 採否"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/139"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "artifact-durability-tiers-resume-verification.md"
    - "ruviform-dependency-ci.md"
    - "ruviform-lockfile-source-of-truth.md"
    - "pre-migration-baseline.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "ruviform-migration-responsibility-map.md"
    - "export-portable-projection.md"
    - "https://github.com/scarecrow173/audio-lens/issues/143"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "https://github.com/scarecrow173/ruviform/issues/284"
    - "https://github.com/scarecrow173/ruviform/issues/285"
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/288"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/283"
    - "https://github.com/scarecrow173/ruviform/issues/287"
  defers: []
  deferred-by: []
---

# Issue #139 Ruviform 共通 I/O 移行の残存分類 — StagedFile 移行記録・回帰契約・Structured/Archive 採否

## Context

AL-IO-01([#139](https://github.com/scarecrow173/audio-lens/issues/139))は
Ruviform 汎用 I/O 共通設計([ruviform #282](https://github.com/scarecrow173/ruviform/issues/282))
に対応する AudioLens 側の追跡・詳細化設計 Issue であり、実装完了を
意味しない。後続実装 Issue AL-IO-02([#143](https://github.com/scarecrow173/audio-lens/issues/143))が
PR #154 で main マージされ、`al-job::fsutil` の汎用 staging/publish は
`ruviform-io-file` の StagedFile type-state 契約
(`StagedFile` → `SealedFile::publish`)へ移行済みである。

本書は #139 の Design Acceptance Gate 全項目を main(4a99719 時点)の
現状と突合した分類記録と、残存した文書化項目(移行記録・Structured/
Archive 採否・Breaking Change 採用 Gate)の設計をまとめる。
コード変更は含まない。

## Scope

### In Scope

- Gate 項目の 完了 / 文書化 / 対象外 分類
- fsutil → StagedFile の移行一覧と振る舞い・Conflict・Durability 差異の
  明文化(現行契約の記録)
- Structured JSON / 将来 CSV・Archive・Compression の consumer 側
  ユースケース・採否・Schema 所有権の明文化
- File Write 失敗・電源断・cancel/retry・Lock 境界の回帰配置の対応表
- 新 Breaking Change 採用 Gate の明文化(#139 追補)

### Out of Scope

- コード変更。実装不足が見つかった場合は範囲限定の起票候補として
  報告し、本書では文書整合までとする
- 両 Consumer Gate([ruviform #288](https://github.com/scarecrow173/ruviform/issues/288))
  への AudioLens 検証証拠の送付 — Integration/Acceptance 段階の後続
- Structured/Archive/Compression の consumer 実装 — Ruviform 側が
  設計段階([#284](https://github.com/scarecrow173/ruviform/issues/284)
  /[#285](https://github.com/scarecrow173/ruviform/issues/285)
  /[#286](https://github.com/scarecrow173/ruviform/issues/286))で
  実装 crate が存在しない

## Gate 分類(2026-11-04、main 4a99719 時点)

| # | Gate 項目 | 分類 | 根拠 |
| --- | --- | --- | --- |
| 1 | fsutil→Ruviform API 移行一覧・振る舞い/回帰/Conflict/Durability 差異・段階移行方針 | **完了**(実装 #154・記録は本書) | fsutil.rs StagedFile adapter。既存 durability 設計書の旧 API 表記は本 PR で整合 |
| 2 | manifest.json・Job stage・Dataset reader/writer/chunk completion/precision の Golden 比較計画 | **完了** | `pre-migration-baseline.md` + `tools/baseline/`。適用関係は後述 |
| 3 | Structured JSON/将来 CSV・Archive の consumer ユースケース・採否・Schema 所有権の明文化 | **本書で文書化** | 後述の採否表。既存 docs には記述なし |
| 4 | 既存 File Write 失敗・電源断・cancel/retry/Lock 境界の Regression Test | **完了** | `crates/al-job/tests/`・fsutil 単体テスト。対応表は後述 |
| 5 | Ruviform 基盤の公開 API/DAG・MSRV・Feature 構成・必要な変更の Issue 化 | **完了**(継続追従は [#127](https://github.com/scarecrow173/audio-lens/issues/127)) | `ruviform-dependency-ci.md`・`ruviform-lockfile-source-of-truth.md`・`ruviform_deps.rs` 群 |
| 6 | 両 Consumer Gate ruviform#288 への検証証拠 | **対象外**(後続) | ruviform #288 は design 段階で open。#143 header の Acceptance 区分どおり後続ゲート |
| 追補 | 新 Breaking Change 採用 Gate への追加 | **本書で明文化** | #143 で採用実績済み。採用 Gate 一覧は後述 |

## fsutil → ruviform-io-file StagedFile 移行記録

### Ruviform へ移管した責務

- staging temp の生成・管理 — 固定 `<name>.tmp` から一意
  `.rvf-tmp-*` sibling temp へ
- seal → publish パイプラインと OS 固有の atomic publish 実装
- durability の実行(file sync / directory sync)と達成保証の
  `DurabilityEvidence` 報告
- publish 失敗の構造化分類(`PublishFailure`: kind / phase /
  effect / operation / temp state)
- temp recovery handle

### AudioLens に残した責務

- `checked_job_path` — persisted manifest は resume 時点で信頼しない
  入力であり、job dir escape を拒否する
- job advisory lock(`<base>/<job>.lock`)による job 実行の直列化 —
  同一 destination への staging 競合調整はこの lock が担保する
- stage resume の artifact 実在検証・Pending 降格・manifest commit
  規則(「manifest 記録 = 存在かつ完全」の不変条件)
- `manifest.json`・dataset schema(`schema.json` / `index.json` /
  `array.json`)・`index.json` commit marker・f64 binary chunk の
  精度契約。Schema 所有権は AudioLens
- `PublishOptions` の用途別選択 — 等級割当は artifact-durability
  ADR の 2 階層を呼出側が明示する
- `PublishError` への `PublishFailure` 構造化抽出と、既存公開境界
  でのみ行う `io::Error` flatten
- `Indeterminate` temp の保持と warn 診断(削除しない)方針

### 旧実装との振る舞い・Conflict・Durability 差異

- **temp 名**: 固定 `<name>.tmp` → 一意 `.rvf-tmp-*`。並行 staging の
  衝突は従来どおり job lock で抑止されるため振る舞い不変。孤児
  temp の個別識別が可能になった
- **rename→sync の保証**: 旧 `write_durable` は rename 後に dir sync。
  新契約は publish が要求された durability を内部で実行し
  `DurabilityEvidence` を返す — 達成保証が観測可能になった
- **失敗分類**: 旧は `io::Error` のみ。新は `PublishEffect`
  (`NotPublished` / `Published` / `Indeterminate`)・`PublishPhase`・
  達成 durability・temp state を保持する。`Indeterminate` は temp を
  消さず resume 診断へ報告する(唯一のコピーである可能性を潰さない)
- **PublishMode**: 旧実装は rename による無条件置換のみ。新は
  `Replace` / `CreateNew`(`DestinationExists` を分類報告)を選択可能。
  現行呼出点はすべて `Replace`
- **streaming**: 旧 API は bytes 一括のみ。新は `stage_file` が
  `Write + Seek` sink を返し `WavWriter` 等の streaming 出力に対応、
  `seal()` 後 `publish_sealed` で完了する

### 段階移行の方針(実施済み)

fsutil が薄い adapter となり、全呼出点は `PublishOptions` を明示する
形で一括切替した。旧 API 互換層は設けない(pre-release 方針・IO-05
追補どおり)。Ruviform 側に独立実装が揃っていない基盤(Structured/
Archive/Compression)は無理に移行しない — 現行境界の方針を維持する。

## Manifest / Dataset Golden 比較計画(既存資産の適用)

比較基盤は `tools/baseline/` と `baseline/`([pre-migration-baseline.md](./pre-migration-baseline.md))が
既に提供する。I/O 移行で新規計画を追加せず、同契約を継続適用する。

- `manifest.json`・Job stage: ID canonical 化・絶対パス placeholder・
  record-only field(`duration_ms` 等)の正規化後に構造一致を比較
- Dataset reader/writer: `store.mjs` が `schema.json` / `index.json` /
  chunk `.bin` を公開レイアウトから decode し、surface ごとに
  bit-exact / tolerance / record-only の規則で比較
- chunk completion: `index.json` commit marker 規則(`finish` のみが
  書き、未完 table は reader が拒否)は writer 契約として維持済み
- precision: f64 bit-exact 面と許容差規則は `contracts.json`
- resume: resume run vs canonical の normalized golden 一致
- Structured adapter 等の I/O 面の将来変更で byte 差異が生まれる場合は
  無理に置換しない(#143 の方針)。差分の有無は再計測 run と commit 済み
  baseline の比較で検証可能

## Structured / Compression / Archive の consumer 採否と Schema 所有権

| 基盤 | Ruviform 側状態 | AudioLens の consumer 要件 | 採否 |
| --- | --- | --- | --- |
| Structured JSON/CSV(IO-02 [#284](https://github.com/scarecrow173/ruviform/issues/284)) | 設計中・crate 非存在 | `manifest.json`・dataset 各 JSON は `serde_json` による現行 byte 互換が回帰契約。Serde trait 公開は許容するが、具体 backend 型を AL 公開 API に必須として漏らさない | **条件付き採用** — byte golden を維持できる場合に限り、Ruviform 実装後に個別 Issue で評価。CSV の consumer 要件は現状なし |
| Compression(IO-03 [#285](https://github.com/scarecrow173/ruviform/issues/285)) | 設計中・crate 非存在 | job artifact は非圧縮・dataset chunk は生 f64le で、圧縮の consumer 要件は現状なし | **採否留保** — 要件発生時に評価 |
| Archive Zip/Tar(IO-04 [#286](https://github.com/scarecrow173/ruviform/issues/286)) | 設計中・crate 非存在 | export パッケージは directory copy + staging publish([export-portable-projection.md](./export-portable-projection.md))。Zip/Tar 可搬 export のユースケース・脅威モデルは未確立 | **採否留保** — 要件確立後に個別 Issue。存在しない機能を既定仕様にしない |

Schema 所有権: `manifest.json`・dataset schema・provenance・
`duration_ms` 等の意味論・JobID/Stage 再開規則は AudioLens が所有する。
Ruviform は Schema/Job の規則を所有しない(現行境界どおり、
[責務マップ](./ruviform-migration-responsibility-map.md)と整合)。

## File Write 失敗・電源断・cancel/retry・Lock 境界の回帰配置

| 境界 | 回帰配置 |
| --- | --- |
| staging/write 失敗 | fsutil `staging_failure_reports_not_published`(effect=NotPublished・destination untouched) |
| publish 失敗分類・temp recovery | fsutil `create_new_reports_destination_exists_and_recovers_temp`・各テストの `staged_temp_left` assert(分類済 temp 除去・`Indeterminate` 保持) |
| streaming publish | fsutil `streamed_write_publishes_through_seal`(seal→publish・temp 非残留) |
| 電源断(relaxed artifact 喪失) | `resume_regenerates_missing_completed_artifact` / `resume_regenerates_truncated_byte_len_artifact` / `resume_regenerates_dir_artifact_with_missing_member`(tests/resume.rs) |
| cancel | `cancel_between_stages_leaves_pending_and_manifest_persisted`(tests/hooks.rs) |
| retry(= resume 再実行) | `failed_stage_preserves_completed_artifacts_and_resume_completes` ほか resume.rs 一式 |
| Lock 境界 | `concurrent_resume_of_same_job_is_rejected`・`leftover_lock_file_does_not_block_run` |
| manifest commit 順序 | durable publish が当該 stage の全 artifact 書込完了後にのみ行われる(更新後の durability 設計書が正本) |

#154 で adapter 層の失敗分類・temp recovery を含めて全 green。追加の
回帰テストは不要と判断した。

## 新 Breaking Change 採用 Gate(#139 追補の明文化)

Ruviform 側 breaking change(旧 API 撤去等)を採用する PR は、lockfile
主体の更新手順([ruviform-lockfile-source-of-truth.md](./ruviform-lockfile-source-of-truth.md))
に加え、以下を検証証跡として要求する:

1. 依存更新 — `update-ruviform-rev` で resolved SHA を揃え、
   `check-ruviform-lock` / `check-ruviform-rev` が pass
2. build — `cargo build` / `check` workspace green
3. Stage 再開 — 前表の resume 系テストが green
4. File Sync/原子性 — publish 経路の `Durability` 割当が等級表どおり
   (commit record = `FileAndDirectorySync`、再生性 bulk = `BestEffort`)
5. 失敗後 Recovery — `PublishFailure` 分類と temp recovery 方針
   (`Indeterminate` 保持・分類済み除去)が adapter に維持される
6. Golden — 変更が manifest / dataset / 解析成果物の byte 面に差分を
   生む場合は baseline 比較で差分を説明できること。生まない場合は
   既存 golden 維持

旧 API 互換層・長期 SHA 固定は求めない(pre-release 方針)。

## Ruviform 基盤との公開 API/DAG・MSRV・Feature 構成

- 公開 API 面: `crates/*/tests/ruviform_deps.rs` が smoke import で監視
- 依存 crate・feature: workspace `Cargo.toml` は git URL + features のみ
  (rev なし — Cargo.lock が resolved SHA の唯一の正本)
- MSRV: ruviform `rust-version` 1.89 ≤ AL mise pin 1.99.0。AL は
  `rust-version` を宣言しない方針([ruviform-dependency-ci.md](./ruviform-dependency-ci.md))
- DAG: `docs/plans/ruviform-foundation-migration.md` と責務マップが
  処理単位の依存順を所有
- 必要な変更の Issue 化: 継続追従は [#127](https://github.com/scarecrow173/audio-lens/issues/127)
  が担い、Ruviform 側の問題は ruviform repo へ起票する
  (AGENTS.md マルチリポジトリ規約)

## 対象外(Integration/Acceptance 後続)

- ruviform [#288](https://github.com/scarecrow173/ruviform/issues/288)
  (両 Consumer Gate)への AudioLens 検証証拠送付 — IO-06 gate は
  Ruviform 側 design 段階で open。#143 header の Acceptance 区分
  どおり、RF #294/#306 で独立検証する段階の後続である
- Structured/Archive/Compression の採用実装 — Ruviform crate が
  存在しないため実装 Issue は起票しない。要件発生時に前掲の
  採否表の条件で評価する

## Non-Functional Constraints

- 本書は記録・採否の正本であり、実装コードの変更を要求しない
- 既存精度・回帰契約(Job/Dataset/Provenance)は AudioLens 所有 —
  Ruviform 還元の設計記述と混同しない
- spec/ADR/plans/tasks/discovery/impl-ir の旧 API 名称(`write_durable`
  等)は当時の決定・実施記録として保持し、遡って改変しない。現行
  契約の正本は本書と更新済みの durability 設計書とする

## Risks and Trade-offs

- 分類は 2026-11-04 時点の main(4a99719)に基づく。Ruviform I/O 設計
  (io-02〜06)の進行で採否表の前提が変わり得る — 変化は #127 の
  継続追従で拾い、本書を更新する
- Structured adapter の byte 互換は未検証であり、条件付き採用の
  条件自体が成立しない可能性がある。その場合は現行 serde_json
  直使用を維持する(byte golden が回帰契約として優先)

## Deferred Design Concerns

- export の Zip/Tar archive 化 | ユースケース・脅威モデル未確立 |
  可搬 export に archive 要件が立った時 | 存在しない機能を既定
  仕様にして過剰実装するリスク
- Structured adapter 導入 | byte 差異の有無が未検証 |
  ruviform-structured 実装後 | 無理な置換による golden 破壊
- Compression 導入 | consumer 要件なし | dataset 肥大等で要件が
  立った時 | 不要な依存の持ち込み

## Verification Notes

- [x] Gate 全項目に 完了 / 文書化 / 対象外 の分類が付された
- [x] 回帰配置表の各テストが `crates/al-job` に存在する
- [ ] reviewer による採否表・採用 Gate の承認
