---
id: "SPEC-034GsJJVWBOQmxiKManOlL"
type: "spec"
status: "approved"
title: "Artifact durability tiers and resume verification (issue #55)"
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
    - "../adr/artifact-durability-tiers-relaxed-bulk-writes.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/durability-tiers-artifact-fsync.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/durability-tiers-resume-verification.md"
  references: []
  defers: []
  deferred-by: []
---

# Artifact durability tiers and resume verification (issue #55)

## What Should Be Built

ジョブが生成するファイルの書き込み耐久性を成果物種別の固定等級
(モード切替なし)で格付けし、再生成可能な bulk データの fsync を
削減する。同時に resume が Completed stage の artifact 実在を検証し、
電源断で欠落した relaxed artifact を持つ stage を再実行へ降格する。

## Why Now

現状は全 artifact 書き込みが `write_durable`(tmp + `sync_all` +
rename + dir sync)を通り、1 run で ~1-2K 回の fsync が発生して
Windows で秒単位のコストになる。中間 artifact は入力正本から
再生成可能であり、全件フル耐久化はコストに見合わない。

## Users and Value

- パイプライン利用者: ingest/dataset の wall-clock 短縮。
- 運用者: manifest 記録と実ファイルの整合が電源断後も保たれる
  保証(resume が再生成で修復する)。

## Scope

### In Scope

- `al_job::fsutil` に耐久性なしの原子書き込み経路(仮称
  `write_atomic` / `rename_atomic`)を追加する。
- 全 `write_durable` / `sync_all` 呼び出し点を等級で格付けし、
  bulk データ書き込みを relaxed 経路へ切り替える:
  - relaxed: `decoded.f64le`、normalized PCM(f64le)、dataset
    table chunk `NNNNNN.bin`、tensor chunk、WAV 音声 payload
  - durable(現行維持): job manifest、dataset manifest、store
    メタデータ(`schema.json`/`index.json`/`array.json`)、
    metadata/provenance 等の全 JSON 記録
- `StageRunner::resume` で Completed/Skipped stage の artifact
  実在を検証し、欠落を検出した stage を Pending に降格して
  再実行する。降格時に当該 stage の artifact レコードを
  manifest から除去する。
- resume 検証の粒度:
  - ファイル artifact: 実在 + `params.byte_len` が記録されていれば
    サイズ一致
  - ディレクトリ artifact(store ルート): `params.files` に列挙
    された全ファイルの実在
- bulk データ producer は `params` に検証情報(`byte_len` /
  `files`)を記録する。

### Out of Scope

- fast/safe モード切替(確定決定で却下済み)。
- relaxed artifact の内容ハッシュ検証(存在+サイズまで。
  「存在するが内容が未フラッシュ」は resume 検出の対象外とし、
  store reader 側の既存整合性チェックが残存リスクとして発火する)。
- manifest schema の変更(`params` は自由 JSON のため schema 不変)。

## Requirements

- 正常系: relaxed/durable いずれの書き込みも、クラッシュなしの
  run では現行と同一のファイル内容・manifest・provenance を
  生成する。
- 原子性: relaxed 書き込みも tmp→rename を維持し、実行中の
  読み手が torn ファイルを観測しない。
- 順序規約: stage の manifest commit は当該 stage の全 artifact
  書き込み完了後にのみ行われる(現行どおり durable save)。
- resume: manifest が Completed と記録する stage の artifact が
  欠落していれば、当該 stage を再実行して再生する。
  resume 検証通過後は「manifest の stage.artifacts 記録 =
  存在かつ完全」が成り立つ。
- 降格による artifact レコード除去で、下流 Completed stage の
  lineage が旧 artifact id を指し続ける(dangling)ことを許容
  する — `lineage_of` は不在レコードを静かにスキップし、
  再実行は当該 stage のみで完結する。
- 互換性: リリース前のため既存 job ディレクトリとの互換層は
  設けない。`params.byte_len`/`files` が無い旧記録は存在検証
  のみで扱う。

## Acceptance Criteria

- [ ] `cargo test --workspace` が全件パスする。
- [ ] Completed stage の relaxed artifact を手動削除した状態で
      resume すると、当該 stage が再実行されて成果物が再生される
      ことを示すテストがある。
- [ ] `params.byte_len` 記録済み artifact を切り詰めた状態で
      resume すると再実行されることを示すテストがある。
- [ ] relaxed 経路の書き込みが fsync を呼ばないことを構造的に
      確認できる(呼び出し点が `write_atomic`/`rename_atomic`
      であること)。
- [ ] manifest・store メタデータ・JSON 記録が引き続き
      `write_durable` 経由であることを確認できる。

## Deferred Design Concerns

- relaxed artifact の内容ハッシュ(SHA-256)検証 | resume 時の
  全件ハッシュは読み取りコストが大きい | 存在+サイズ検証を
  すり抜ける破損が実害として観測された場合 | 未フラッシュ部分が
  ゼロ化したファイルが resume を通過し、後段で数値を汚染する
- fsync 削減量の定量計測 | 環境依存が大きく CI で不安定 |
  Windows 実機で dataset run が依然として IO 律速の場合 |
  緩和しても十分な改善が得られない可能性

## Sources and Relations

- `docs/discovery/durability-tiers-artifact-fsync.md`(本 spec の
  上流調査: 呼び出し点棚卸し・resume 無検証の確認)
- `docs/discovery/remaining-issues-roadmap.md` — #55 確定決定
- ADR: `docs/adr/artifact-durability-tiers-relaxed-bulk-writes.md`
