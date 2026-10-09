---
id: "TASK-035hJkLmNpQrStUvWxYz"
type: "task"
status: "todo"
title: "locate: サンプルレート正規化(リジェクト→リサンプル統一)"
created: "2026-10-09"
updated: "2026-10-09"
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
    - "docs/plans/simd-dsp-foundation.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/al-math-skeleton.md"
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
    - "docs/test-specs/simd-dsp-foundation.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/37"
  defers: []
  deferred-by: []
---

# locate: サンプルレート正規化

## Work

`sample_rate_mismatch` でリジェクトしている現行挙動を、
「source→capture 基準レートにリサンプル統一してから比較」に
変更する。

- `al-math` に polyphase FIR または FFT ベースのリサンプラを
  追加(線形補間は不可 — スペクトルイメージが `exp.src_image`
  や帯域ゲインを汚染する)
- locate 入口で source を capture のレートに変換
- 元レート・変換後レート・リサンプラ種別を provenance/result に
  記録
- `sample_rate_mismatch` diagnostic を「リサンプル済み」情報に
  置き換え(リジェクトではなく注記)
- drift/rate 推定との分離(リサンプル由来のレート差が
  time_map に混入しないこと)を golden で確認

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/simd-dsp-foundation.md`
- `docs/specs/source-localization.md`

## Dependencies

- `docs/tasks/al-math-skeleton.md`(al-math のリサンプラを使う)

## Affected Files

- `crates/al-math/src/resample.rs`(新規)
- `crates/al-locate/src/stage.rs`(正規化呼び出し・diagnostic 変更)

## Done When

- [ ] 44.1kHz source × 48kHz capture のペアがリジェクトされず
      正常に locate される
- [ ] provenance に元/変換後レート・リサンプラ種別が記録される
- [ ] 既存 golden(レート一致ケース)が変わらない

## Verification

- [ ] `cargo test -p al-locate`
