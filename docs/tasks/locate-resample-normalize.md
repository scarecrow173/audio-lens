---
id: "TASK-035h6IfQGjVLTZjgH6kiRT"
type: "task"
status: "done"
title: "locate: サンプルレート正規化(リジェクト→リサンプル統一)"
created: "2026-10-09"
updated: "2026-10-12"
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
- `crates/al-locate/src/normalize.rs`(入力準備・永続化・座標対応)
- `crates/al-locate/src/decode.rs`(正規化 PCM の共有読込境界)
- `crates/al-locate/src/result.rs` / `stage.rs`(PCM 参照・正規化区間)
- `crates/al-analysis/src/stage.rs`(reference/expensive の保存 PCM 再取得)
- 対応する単体・統合テストと依存定義

## Done When

- [x] 44.1kHz source × 48kHz capture のペアがリジェクトされず
      正常に locate される
- [x] provenance に元/変換後レート・リサンプラ種別が記録される
- [x] 既存 golden(レート一致ケース)が変わらない

## Verification

- [x] `cargo test -p al-locate`

## 実装結果

source を capture 基準レートへアンチエイリアス付き polyphase FIR で
正規化する。Blackman 窓 sinc、unit-DC 係数、ゼロ端点拡張、ceil 出力長、
群遅延補償を採用し、SIMD と scalar は同じ実装・固定加算順序を使う。
レート一致は identity とし、振幅の overshoot はクリップしない。

`normalized_pcm` は `locate/normalized/<artifact_id>.f64le` に永続化し、
元 PCM/metadata の lineage、完全な変換設定、元/正規化レート・長さ、
有理数の時刻原点を params に記録する。成果物の params と元成果物 ID
をキーに再利用し、input ID だけでは共用しない。

alignment は source/capture の PCM 参照と `normalized_source_range` を
保持する。`source_range` は元レートの floor/ceil・clamp 済み座標、
`capture_range` は capture 座標である。time_map は正規化 source 座標
で推定し、その座標空間と PCM 参照を明記する。result params に正規化
provenance、derived に `resampled` 注記を残す。

reference/expensive の alignment 比較は `decode.rs` の artifact ID
読込境界を通る。role、配置、長さ、変換、lineage、区間を検証し、
参照欠落・不整合時は stage error とする。元 PCM を読む `decode_input`
は元入力観測用に維持する。

## 検証記録 (2026-10-10)

- `mise install --locked` は global config の未ロック tool で失敗。
  `mise install --locked rust python uv pipx:apm-cli pipx:serena-agent`
  と `mise run setup` は成功。
- `mise exec -- cargo build -p al-locate -p al-math`: 成功。
- `mise exec -- cargo test -p al-locate -p al-math -p al-analysis`:
  146 tests 成功。既存 locate golden 16 件、analysis golden 19 件を
  変更せず通過。
- `mise exec -- cargo clippy -p al-locate -p al-math -p al-analysis
  --all-targets -- -D warnings`: 成功。
- `cargo fmt --check` と `cargo fmt -p al-locate --check` は既存の
  `examples/bench_dsp.rs:22` のみで失敗。`cargo fmt -p al-math
  -p al-analysis --check` と、変更した al-locate の全 Rust ファイルを
  指定した `rustfmt --check --edition 2024 --config skip_children=true`
  は成功。既存 bench の書式は今回の範囲外として保持。
- 44.1k/48k の独立生成 chirp の定位・偽 drift がないこと、scalar/SIMD
  等価性、alias/image 抑制、遅延補償、overshoot 保持を確認。
- identity・整数 PCM の一度だけのスケール適用・チャンネル順序、複数
  目標レートの再利用分離、座標丸め、参照欠落/不整合を確認。
- 別プロセスで manifest を復元して reference/expensive を実行し、
  同じ PCM・区間・lineage を確認。元 decoded PCM を変更しても保存
  PCM を使用し、保存 PCM 欠落時は両 stage がエラーになることを確認。
