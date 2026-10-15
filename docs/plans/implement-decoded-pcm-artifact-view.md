---
id: "PLAN-034G5TxZGhHh39SQ8S5su4"
type: "plan"
status: "approved"
title: "decoded/normalized PCM read-only view の実装"
created: "2026-10-15"
updated: "2026-10-15"
owners: ["scarecrow173"]
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
    - "docs/specs/decoded-pcm-artifact-view.md"
  implemented-by:
    - "../tasks/pcm-view-foundation.md"
    - "../tasks/decode-callers-view-migration.md"
    - "../tasks/pcm-view-measurement-cache-decision.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/decoded-pcm-artifact-view.md"
    - "docs/designs/decoded-pcm-artifact-view.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/decoded-pcm-artifact-view.md"
  references: []
  defers: []
  deferred-by: []
---

# decoded/normalized PCM read-only view の実装

## Goal and Scope

approved spec/accepted ADR/approved design の PcmView 契約を実装する。
view 基盤 → 2 caller 移行 → 計測/cache 判定の1stream。
deinterleave 済み cache、stage 間受け渡し、residual WAV、部分読み取り
API、StageContext/Registry 共有可変拡張、job cache の先行実装はない。
計測結果が cache 必要性を示した場合のみ後続判断とし、本planの既定
着地は per-call view。

## Upstream Documents

- `docs/specs/decoded-pcm-artifact-view.md`(approved)
- `docs/adr/artifact-id-read-only-pcm-view-cache.md`(accepted)
- `docs/discovery/decoded-pcm-artifact-view.md`(anchor)
- issue: https://github.com/scarecrow173/audio-lens/issues/56

## Design Inputs

- `docs/designs/decoded-pcm-artifact-view.md`(approved)。
  PcmView/PcmBacking・融合 deinterleave の逐語契約・計測設計・cache
  条件付き選択肢は design が唯一の正。

## File Map

| パス | 所有する変更 |
| --- | --- |
| `crates/al-locate/Cargo.toml` | memmap2 依存追加(承認済み、7日超の安定版 pin) |
| `crates/al-locate/src/decode.rs` | PcmView/PcmBacking・open/byte_len/sample/deinterleave・2 reader 移行・`cfg(test)` モジュール(oracle 等価・fallback・境界・計数 alloc harness・計測) |
| `docs/test-specs/decoded-pcm-artifact-view.md` | 同義 test 契約 |
| `docs/tasks/{pcm-view-foundation,decode-callers-view-migration,pcm-view-measurement-cache-decision}.md` | 3 slices |
| `docs/impl/ir/` 対応3件と関連 index | 証拠・cache 判定・完成 review |
| `target/pcm-view-evidence/` | ignored run artifacts(fixture・計測ログ) |

`dsp::deinterleave` は residual/他 caller が使うため変更しない。
`checked_job_path`・manifest/lineage 検証は一切変更しない。

## Ordered Task Proposals

### Slice 1: pcm-view-foundation

1. `al-locate` に memmap2 を追加(crates.io 公開7日超の安定版を pin)。
2. `PcmBacking`(Mapped/Owned)と `PcmView`(open/byte_len/sample/
   deinterleave)を decode.rs に実装。empty・非通常ファイル・mmap
   失敗で Owned fallback、I/O エラーは伝播。unsafe は `Mmap::map`
   のみで不変性前提を comment 明記。
3. `dsp::deinterleave` を oracle とする等価テスト: scale=1.0/非1.0、
   partial frame drop、channels==0、空ファイル、len%8!=0 は caller
   側検証で処理(view 自体は byte view)。`PcmView` は `pub(crate)`
   のため等価・境界テストは `decode.rs` の `cfg(test)` モジュールに
   置く(integration test からは見えない)。fixture は temp dir に
   生成し新依存は使わない。
4. 強制 Owned 経路(`pub(crate)` な `PcmView::open_owned` または
   `PcmBacking::Owned` 直接構築)で mmap/Owned の同一結果を検証。

### Slice 2: decode-callers-view-migration (depends on Slice 1)

1. `decode_input` を `PcmView::open` + `view.deinterleave(ch, scale)`
   に置き換え、`Vec<f64>` 中間確保を除去。`len % 8` は
   `view.byte_len()` で同一エラーを維持。
2. `read_normalized_record` を同様に移行(`scale=1.0`)。有限性検証は
   materialize 後 channel で同一エラーを返す。expected length 検証は
   `view.byte_len()` で維持。
3. 既存 `cargo test -p al-locate` / `cargo test -p al-analysis` が
   変更なし green。golden/locate/prepared 既存テストで
   manifest/artifact 検証の同値性を確認。

### Slice 3: pcm-view-measurement-cache-decision (depends on Slice 2)

1. `decode.rs` の `cfg(test)` モジュールに計数 allocator harness を
   実装し、baseline(fs::read+collect+dsp::deinterleave)・mmap view・
   Owned view の3経路で `channel_samples` bit-identical を検証。
2. 計測: alloc calls/bytes・wall time(p50/p95、N=30)・RSS・
   fixture(約36MB、実サイズ級は ignored ゲート)・環境を記録。
3. cache 判定: view open が materialize 全体の10%超または e2e 有意
   なら job cache を検討、小さければ per-call view で閉じる。
   判定根拠を IR に記録。
4. IR 3件・roadmap Stage 更新・完成 review・commit gate 確認。

3slices は依存順のみ、並列実装しない。

## Bundled Test-Spec Contract Proposal

この節は plan review に含める同義 test-spec/task 案であり、draft
plan から実ファイルを生成しない。plan 承認後に1test-specと3tasksを
忠実に生成する。

- Rule 1: view 経路の `channel_samples` は baseline と bit-identical。
  Example: scale 1.0/非1.0、奇数サンプルの partial frame drop、
  channels==0、空ファイル、mono/stereo/多ch。
- Rule 2: 強制 Owned fallback と mmap で同一結果・同一エラー。
  Example: mmap 不能を強制して decode_input 相当が同じ出力を返す。
- Rule 3: `len % 8 != 0`・rate/channels 0・manifest/lineage/path
  検証は現行と同一エラー。`decode_input` の Option 契約を維持。
- Rule 4: 計測は fixture・環境・p50/p95・alloc・RSS を記録し、
  cache 導入可否の根拠を残す。未計測の性能 claim をしない。
- Rule 5: unsafe は `Mmap::map` のみ、artifact 不変性前提を code
  comment で明記する。

## Verification Matrix and Commands

| 保証 | 実行/合格条件 |
| --- | --- |
| 等価性 | `mise exec -- cargo test -p al-locate decode::tests` で3経路 bit-identical・境界 |
| 回帰 | `mise exec -- cargo test -p al-locate -p al-analysis` 全 green、golden/prepared 変更なし |
| 計測 | decode::tests の計測 test が alloc/time/RSS を出力し target/pcm-view-evidence に記録 |
| 品質 | `mise exec -- cargo fmt --all -- --check`、`mise exec -- cargo clippy -p al-locate --all-targets -- -D warnings`、audit_docs、`git diff --check` |

## Risks, Rollback and Handoff

主 risk は mmap の利益が小さい判定(= per-call view で閉じる既定
着地)、`dsp::deinterleave` との微細な意味差(部分フレーム)、
Windows ファイルロック(短命 view で回避)。view は呼出内で drop
するため残留ロックなし。
memmap2 追加は承認済み。その他の依存・schema・manifest 変更なし。
既存 dirty や他 issue の変更は revert しない。

## Self Review

spec 全 AC(計測・bit-identical・fallback・cache 判定・resume・既存
検証維持)を task/rule/matrix に対応済み。専用 focused chain 1件、
計測は未実施として扱う。plan 承認後に test-spec/task を生成する。

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none(3slices は依存順)
- User approval required before dispatch: yes

## 2026-10-15 Plan Review

Reviewer A: spec 全 AC と design 契約の slice/rule/matrix への対応を確認。
Reviewer B: 4候補。Validator: IC-1 `pub(crate)` PcmView は integration
test から不可視のため等価/計測テストの置き場所が誤り CONFIRMED
(`decode.rs` cfg(test) モジュールへ修正、強制 Owned 経路を pub(crate)
構築として明記)。IC-2 空ファイル mmap は fallback でカバー済み
REJECTED、IC-3 有限性検証順序は結果同値 REJECTED、IC-4 byte_len
検証は等価 REJECTED。Judge ACCEPT、bundled test-spec/task 案も
同義承認。動的実証は未実施。
