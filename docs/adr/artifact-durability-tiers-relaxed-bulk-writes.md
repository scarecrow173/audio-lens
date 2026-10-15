---
id: "ADR-034GsJKnwUmeinOPVoR2RQ"
type: "adr"
status: "accepted"
title: "Artifact durability tiers: relaxed bulk writes with resume verification"
created: "2026-10-15"
updated: "2026-10-15"
owners: []
relations:
  source:
    - "../discovery/durability-tiers-artifact-fsync.md"
    - "https://github.com/scarecrow173/audio-lens/issues/55"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/artifact-durability-tiers-resume-verification.md"
    - "../plans/implement-artifact-durability-tiers.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/durability-tiered-artifact-writes.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/durability-tiers-artifact-fsync.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Artifact durability tiers: relaxed bulk writes with resume verification

## Context and Problem Statement

全 artifact 書き込みが `al_job::fsutil::write_durable`(tmp 書込 +
`sync_all` + rename + 親 dir sync)を通り、dataset 生成 1 run で
~1-2K 回の fsync が発生して Windows 上で秒単位のコストになる。
一方、書き込まれるデータの大半(decoded PCM、normalized PCM、
table/tensor chunk、WAV payload)はユーザー提供の入力正本から
再生成可能であり、電源断喪失の損害は「当該 stage の再実行」に
留まる。現在の `StageRunner::resume` は `StageStatus::Completed`
を無検証で信用するため、単に fsync を外すと「manifest は完了を
記録したがファイルは消えた」状態を resume がそのまま読む。

## Decision Drivers

* 中間 artifact の喪失は再実行可能でデータ損失ではない(正本は
  入力ファイル)
* fsync 回数を artifact 数から「記録ファイル数」級へ削減する
* manifest 記録と実ファイルの整合: resume は「stage.artifacts に
  記録された artifact は存在し完全」を前提にできる
* モード切替を設けない(確定決定): 設計・テスト・説明コストを
  増やさない
* `write_durable` の原子性(tmp→rename で torn ファイルを
  見せない)は relaxed 経路でも維持する

## Considered Options

* A 全 artifact に fsync を維持する(現状)
* B 固定2等級 — bulk を `write_atomic`(fsync なし)にし、
  resume 時の存在検証で欠落 stage を再実行へ降格する
* C stage 境界でバッチ fsync する(relaxed 書込 + manifest commit
  前に全ファイル sync)
* D fast/safe モード切替を導入する

## Decision Outcome

Chosen option: "B 固定2等級 + resume 存在検証", because fsync を
記録系ファイルのみに削減でき、resume 検証が「manifest 記録 =
存在かつ完全」の不変条件を回復する。C は fsync がファイル単位
であり回数を減らせず、D は確定決定で却下済み、A は issue の
動機を解消しない。

等級割り当て:

| Tier | 対象 | 経路 |
| --- | --- | --- |
| durable | manifest.json(job/dataset)、schema.json、index.json、array.json、metadata/provenance 等の JSON 記録 | `write_durable` / `rename_durable`(現行) |
| relaxed | decoded.f64le、normalized PCM、table chunk、tensor chunk、WAV payload | `write_atomic` / `rename_atomic`(新規) |

順序規約: stage の manifest commit(durable)は全 artifact 書込
完了後にのみ行う。resume 側は Completed/Skipped stage の
`stage.artifacts` に列挙された artifact を検証する:
ファイルは実在+`params.byte_len` 一致、ディレクトリは
`params.files` 列挙の全実在。欠落時は stage を Pending に降格し
当該 artifact レコードを manifest から除去して再実行する。

### Consequences

* Good, because bulk 書き込みの fsync が無くなり、fsync 回数が
  manifest+メタデータの少数(1 run に数十)へ削減される。
* Good, because resume が欠落 artifact を検出して再生成するため、
  電源断後も「manifest 記録 = 存在かつ完全」が検証後に成り立つ。
* Bad, because relaxed artifact は「存在するが内容未フラッシュ」
  を resume が検出しない(サイズ一致までは検出、内容までは
  検査しない)。store reader の既存チェックが残存リスクとして
  残る。
* Bad, because producer が `params.byte_len`/`params.files` を
  記録する協定が増える。

### Confirmation

resume 検証: Completed stage の artifact を手動削除/切り詰めて
resume すると stage が再実行されるテスト。等級割当: bulk 書込が
`write_atomic`/`rename_atomic` を呼び、記録系が `write_durable`
を維持することをコードレビューとテストで確認。

## Implementation Plan

* Affected paths: `crates/al-job/src/fsutil.rs`,
  `crates/al-job/src/runner.rs`, `crates/al-media/src/stage.rs`,
  `crates/al-media/src/wav.rs`, `crates/al-dataset/src/{table,tensor,
  audio,writer}.rs`, `crates/al-locate/src/{normalize,stage}.rs`,
  `crates/al-dataset/src/stage.rs`
* Patterns to follow: `write_durable` と同じ tmp→rename 構造;
  `params` への検証情報記録は producer 側の ArtifactRecord 構築で
  一箇所に集約
* Patterns to avoid: caller 毎に fsync 有無を ad-hoc に散らす
  こと。等級は成果物種別で一意に決まり、fsutil の2関数で表現する
* Dependencies or configuration: 新規依存なし。manifest schema
  変更なし(`params` のみ)

## Verification

* [ ] `cargo test --workspace` 全件パス
* [ ] artifact 欠落/切り詰め → resume で stage 再実行のテスト
* [ ] relaxed 書込が `sync_all` を呼ばない構造確認
* [ ] 記録系が `write_durable` 維持の構造確認

## Pros and Cons of the Options

### Option A 現状維持

* Good, because 契約が単純で追加検証が不要。
* Bad, because fsync コストが残り issue の動機を解消しない。

### Option B 固定2等級 + resume 検証

* Good, because fsync を実質削減しつつ整合性を resume 側で回復
  できる。単一の動作モードで説明が単純。
* Bad, because resume 検証と params 記録の実装が必要。存在+サイズ
  以上の破損は検出しない。

### Option C stage 境界バッチ fsync

* Good, because manifest 順序規約を字面通り維持できる。
* Bad, because fsync はファイル単位でしか発行できず回数を
  減らせない — issue の動機を解消しない。

### Option D fast/safe モード

* Good, because ユーザーがリスク許容度を選べる。
* Bad, because 設計・テスト・ドキュメントが倍増する。確定決定で
  却下済み。

## More Information

* `docs/discovery/durability-tiers-artifact-fsync.md` — 呼出点
  棚卸しと resume 無検証の確認
* `docs/specs/durability-tiered-artifact-writes.md` — 要求仕様
* `docs/discovery/remaining-issues-roadmap.md` — #55 確定決定
  (無条件緩和 + manifest 順序規約)
