---
id: "TASK-034z9novQ1qbeZc0x58nsX"
type: "task"
status: "done"
title: "analysis 基盤: DatasetWriter open_or_create + FeatureRecord 拡張"
created: "2026-10-02"
updated: "2026-10-02"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/analysis-stages.md"
  implemented-by: []
  depends-on:
    - "docs/plans/analysis-stages.md"
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
    - "docs/test-specs/analysis-stages.md"
  references: []
  defers: []
  deferred-by: []
---

# analysis 基盤: DatasetWriter open_or_create + FeatureRecord 拡張

## Work

`al-analysis` クレートを新設し、解析系ステージが共有する基盤を実装する。

- `DatasetWriter::open_or_create(path)` を `al-dataset` に追加。
  既存 manifest があれば開き、schema version / input references /
  config fingerprint の一致を検証する。不一致は**明示的エラー**
  (互換レイヤ・マイグレーション禁止)。破損 manifest は黙って
  再作成せず明示的に失敗させる。
- `manifest.inputs` を DatasetStage が `declared_input` artifact から
  構築する入力参照構築を `al-analysis` 側に共通化し、各解析
  ステージと DatasetStage が同じ参照を書き込めるようにする。
- table / tensor / audio / JSON の各 store に「同一キー再書き込み
  で物理ストアごと安全に置き換える」セマンティクスを追加。
  manifest だけの dedup ではなく、orphan・partial store は
  retry 時にクリーンに置き換わること。
- `FeatureRecord` を拡張: resolution クラス / window / hop /
  validity / support を必須フィールドとして記録する。
- `locate` ロールへの結果物化: LocateStage の `result.json` 群と
  time-map データを Analysis Dataset の `locate` role に
  取り込むマテリアライズ関数を提供(DatasetStage または
  後続ステージから呼べる形)。
- `intermediate` ロール(再利用スペクトル置き場)の予約と
  writer API 経路の確立。
- `al-locate::decode` の `decode_input` / `format_scale` を共有
  ローダーとして al-analysis から再利用する(al-locate への
  依存を追加)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/analysis-stages.md`
- `docs/specs/analysis-dataset.md`
- `docs/adr/analysis-dataset-storage.md`
- `docs/tasks/dataset-writer-provenance.md`

## Dependencies

- `docs/tasks/dataset-integration-tests.md`(al-dataset が完了済み)

## Affected Files

- `Cargo.toml`(workspace members)
- `crates/al-analysis/`(新規)
- `crates/al-dataset/src/writer.rs`(`open_or_create`)
- `crates/al-dataset/src/{table,tensor,audio}.rs`(置換セマンティクス)
- `crates/al-dataset/src/types.rs`(`FeatureRecord` 拡張)
- `crates/al-locate/src/decode.rs`(再利用)

## Done When

- [ ] `open_or_create` が schema/inputs/config 不一致と破損
  manifest を明示的に拒否する
- [ ] 再書き込みで物理ストアが置き換わり、resume 後も
  破片が残らない
- [ ] `FeatureRecord` が実解像度メタデータを保持する
- [ ] locate 結果が `locate` role に物化できる

## Verification

- [ ] `cargo test -p al-dataset -p al-analysis`
- [ ] 破損 manifest / 不一致 schema の負例テスト
