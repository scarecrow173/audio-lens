---
id: "DISC-034GsIf9DWqfDWmEdUfBdz"
type: "discovery"
status: "active"
title: "Issue 55 耐久性等級: artifact 書き込み fsync の再設計"
created: "2026-10-15"
updated: "2026-10-15"
owners: ["scarecrow173"]
relations:
  source:
    - "remaining-issues-roadmap.md"
    - "https://github.com/scarecrow173/audio-lens/issues/55"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
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
  derived-by:
    - "../specs/durability-tiered-artifact-writes.md"
    - "../adr/artifact-durability-tiers-relaxed-bulk-writes.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue 55 耐久性等級: artifact 書き込み fsync の再設計

## Exploration Goal

al-dataset / ジョブ artifact 書き込みでファイル毎に `write_durable`
(tmp 書込 + `sync_all` + rename + dir sync)が走り、1 run で ~1-2K 回の
fsync が発生して Windows で秒単位のコストになる(issue 報告)。
耐久性を成果物種別で格付けし、再生成可能な中間 artifact の fsync を
削減する。確定決定(ロードマップ 2026-10-14): **無条件の耐久性等級
(モード切替なし)**、manifest + 最終レコードは full durability 維持、
中間 artifact は緩和、resume は「manifest 記録 = 存在かつ完全」を
前提にできること。

## Key Issues and Assumptions

- 現状の fsync 大量発生の本体は artifact 毎の `write_durable`
  (issue の「4MiB 毎」記述は概算/旧実装)。呼び出し点:
  - **bulk データ(relaxed 候補)**: `al-media` `decoded.f64le`、
    `al-dataset` table chunk `NNNNNN.bin`(1-2K/run)、tensor chunk、
    `audio.rs` の WAV streaming、`al-locate` normalized PCM
  - **記録系(full 維持)**: `al-job`/`al-dataset` manifest.json、
    table `schema.json`/`index.json`、tensor `array.json`、
    ingest metadata/provenance JSON、`persist_json` 等の小 JSON
- **確認済みの重大な前提**: `StageRunner::resume` は
  `StageStatus::Completed` を無条件に信用し、artifact の実在を
  検証しない。relaxed tier で電源断ロストした artifact を持つ
  Completed stage は resume でスキップされ、下流が存在しない
  ファイルを読む → **resume 時の存在検証(欠落なら stage を
  Pending に降格して再生成)** が必須。
- rename は原子性(読み手が torn ファイルを見ない)を与えるが
  耐久性(電源断後に残る)は与えない。relaxed 書込は
  「write→rename、fsync なし」= 内容の原子性は保持、
  電源断でファイル全体が欠落または未フラッシュ部分を含み得る。
- 中間 artifact は入力ファイル(正本・ユーザー提供)から完全に
  再生成可能であり、喪失の最大損害は当該 stage の再実行。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A 全 artifact に fsync 維持(現状) | 契約が単純 | fsync コストが残る | no |
| B 固定等級: bulk=緩和・記録=full + resume 存在検証 | fsync を記録系のみに削減。正本不変 | resume 検証の実装・検証コスト | yes |
| C stage 境界のバッチ fsync | 順序規約を字面通り維持 | ファイル毎 fsync は減らせず効果薄 | no |
| D fast/safe モード切替 | ユーザー選択可 | 設計・テスト・説明コスト倍増 | no(確定決定で却下) |

## Tentative Conclusions and Hypotheses

- **confirmed**: 耐久性等級は2等級で十分 — `durable`(manifest・
  store メタデータ・小 JSON 記録)と `relaxed`(再生成可能な bulk
  データ)。fsutil に `write_atomic`(tmp+write+rename、fsync なし)
  を追加し call site 毎に格付けする。
- **confirmed**: resume は Completed stage の artifact path の
  実在を検証し、欠落があれば当該 stage を Pending に降格して
  再実行する。降格時は stale な artifact レコードを manifest から
  除去する(再実行で新 ID が再登録されるため)。
- **hypothesis**: 「manifest 記録 = 存在かつ完全」は resume 検証
  通過後の不変条件として維持される — relaxed artifact は電源断で
  欠落し得るが、resume が検出して再生成するため観測可能な
  不完全状態は残らない。
- **hypothesis**: 切り詰め検出は存在検証のみでは不十分
  (未フラッシュ部分が残り得る)。`params` に byte_len を記録する
  producer ではサイズ照合を追加できる。範囲は設計で確定する。

## Open Questions

- store ルート(dir)型 artifact(table/tensor)の resume 検証粒度:
  ルート+メタファイル実在までか、index 記載 chunk 全数の実在までか
  — design で決定。
- relaxed artifact の切り詰め検出に `params.byte_len` を使うか —
  design で決定。

## Promotion Candidates

- [x] spec-doc needed: 耐久性等級と resume 検証の要求
- [x] adr-doc needed: 等級モデルと順序規約の正規決定

## Sources and Relations

- Issue #55(al-dataset の fsync 頻度再設計)
- `docs/discovery/remaining-issues-roadmap.md` — #55 確定決定
- `crates/al-job/src/fsutil.rs` — `write_durable`/`rename_durable` の現行契約
- `crates/al-job/src/runner.rs` — resume が `StageStatus::Completed` を
  無検証で信用する現行実装
