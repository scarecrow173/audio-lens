---
id: "DESIGN-034G5QmSv6Yzg756IThZZi"
type: "design"
status: "approved"
title: "decoded/normalized PCM read-only view 設計"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/decoded-pcm-artifact-view.md"
    - "docs/adr/artifact-id-read-only-pcm-view-cache.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/adr/analysis-dataset-storage.md"
  defers: []
  deferred-by: []
---

# decoded/normalized PCM read-only view 設計

## Context

`decode_input` と `read_normalized_record` は同一パターンで f64le
artifact を `fs::read` → `Vec<f64>` 変換 → `deinterleave` の3段全長
確保で読む。`decoded.f64le` は input 毎に run 内3回、normalized
`.f64le` は locate_result 毎に2回実行される。
本designは承認済み spec/ADR の API・所有境界を確定する draft であり、
動的実証は未実施。

## Scope

### In Scope

- `al-locate::decode` 内の read-only `PcmView`(memmap2 mmap 第一候補・
  fs::read owned fallback)。
- `decode_input`・`read_normalized_record` の読み出しを view + 融合
  deinterleave に置き換える(中間 `Vec<f64>` の除去)。
- `fs::read` vs mmap view vs fallback view の計測(確保・CPU・RSS・
  時間)と、job-scoped cache 導入判定の記録。
- `read_residual_record` の normalized 親PCM経路は
  `read_normalized_pcm` 経由で恩恵を受ける(残差 WAV 自体は対象外)。

### Out of Scope

- job-scoped cache の先行実装(計測で必要性が示された場合のみ)。
- deinterleave 済み `Vec<Vec<f64>>` のキャッシュ。
- residual WAV・`al_dataset::read_wav` 経路。
- `StageContext`/`ArtifactRegistry` の共有可変状態拡張。
- 部分読み取り(channel/range materialize)の API と consumer 改修 —
  現 consumer は全て全長を必要とし、将来用の API を先回りして追加
  しない(YAGNI)。

## Components and Boundaries

### `PcmView`(`crates/al-locate/src/decode.rs` 内、`pub(crate)`)

```rust
enum PcmBacking {
    Mapped(memmap2::Mmap),
    Owned(Vec<u8>),
}

pub(crate) struct PcmView {
    backing: PcmBacking,
}

impl PcmView {
    fn open(path: &Path) -> io::Result<Self>;
    fn byte_len(&self) -> usize;
    fn sample_count(&self) -> usize;      // byte_len / 8
    fn sample(&self, index: usize) -> f64; // f64::from_le_bytes
    /// scale 乗算(除算)と deinterleave を単一パスで行う。
    /// 現行 collect-then-deinterleave と bit-identical。
    fn deinterleave(&self, channels: u16, scale: f64) -> Vec<Vec<f64>>;
}
```

- `open` はファイルが空・mmap 失敗・非通常ファイルの場合に
  `fs::read` による `Owned` fallback を返す。I/O エラー自体は
  現行どおり `io::Error` で伝播する。
- mmap は `memmap2::Mmap::map` の unsafe を `open` 内部に局所化する。
  安全性条件: 「staged publish(StagedFile `seal` → `publish`)が
  完了した artifact ファイルは不変」という artifact store の既存
  不変条件。manifest が参照を記録
  した artifact は変更・削除されない(同時に別 writer が同じ job を
  書き換える運用は現行でも壊れる既存前提)。
- `sample(i)` は 8byte 境界の LE parse とし、`&[f64]` への cast は
  行わない(alignment・endianness を機械依存にしない)。
- `deinterleave` は `dsp::deinterleave`(`dsp.rs:17-33`)の契約を
  逐語同値に写す: `n_frames = sample_count / channels` の完全
  フレームのみ処理し末尾の部分フレームを drop、
  `out[ch][f] = sample(f * channels + ch) / scale` のチャンネル
  ブロック収集、`channels == 0` で空 Vec を返す。除算は
  現行どおり materialize 時に各要素へ1回適用する。
- `PcmView` は `Send` 可能だが今回はスレッド間共有しない
  (各 stage が自身の view を open する)。

### 呼び出し側の変更

- `decode_input`: `fs::read` → `PcmView::open`。`len % 8` 検証・
  scale・channel 0/rate 0・metadata 解決の全検証を維持。`Vec<f64>`
  中間確保を除去し `view.deinterleave(channels, scale)` を呼ぶ。
- `read_normalized_record`: 同様に `PcmView::open` + `deinterleave(ch, 1.0)`。
  有限性検証は materialize 後の各 channel で行い、現行と同じエラーを返す。
- 上記以外の manifest/lineage/path 検証は一切変更しない。

### job-scoped cache(条件付き)

計測で view open コストまたは繰り返し read が支配的と判明した場合
のみ、`ArtifactRegistry` 内に
`Mutex<HashMap<ArtifactId, Arc<PcmView>>>` の bounded cache を追加
する選択肢を残す。key は Artifact ID、value は不変 view で Stage
契約上の「通信」ではない(派生表現の透明な性能層)。eviction は
最古/LRU で bounded。計測で利益が小さければ実装せず、per-call
view のみで閉じる — これが既定経路。

## Data and Control Flow

```text
stage → decode_input / read_normalized_record
      → manifest/artifact/lineage/path 検証(現行維持)
      → PcmView::open(mmap → fallback fs::read)
      → len % 8 検証(現行と同一エラー)
      → view.deinterleave(channels, scale)  [1パスで materialize]
      → 有限性検証(normalized)・scale 適用(decoded)
      → DecodedInput / PreparedInput
```

## Non-Functional Constraints

- **Bit-identical**: 値・scale・deinterleave 順・全エラー経路は現行と
  同一。view 導入は数値中立。
- **安全性**: `unsafe` は `memmap2::Mmap::map` 呼出しのみ。不変条件は
  artifact store の書き込み規約由来で、code comment に明記する。
- **メモリ**: materialize 済み PCM は消費側の既存予算。mmap view は
  RSS にページキャッシュ分のみ計上され、deinterleave 済み常駐は
  行わない。
- **依存**: `memmap2` を `al-locate` に追加(2026-10-15 承認済み)。
  バージョンは crates.io で 7 日以上経過した安定版を pin する。

## Measurement Design(計測ファースト)

計測は `crates/al-locate/tests/` に thread_local 計数 GlobalAlloc
(`al-math/tests/ncc_workspace.rs` の既存 pattern)で実装する。
fixture は temp dir に合成 f64le を生成する。CI/通常テストは
約36MB(stereo 48kHz 約50秒)までとし、実サイズ級(約8分 stereo
48kHz ≈ 46M samples ≈ 368MB)は impl/ir 記録用の明示実行とする
(ignored または環境変数ゲート)。mmap は Rust allocator を通らない
ため alloc 計数だけでは効果を表せず、wall time と RSS を併記する。

| 測定 | 内容 |
| --- | --- |
| baseline | 現行 `fs::read` + collect `Vec<f64>` + `dsp::deinterleave` |
| mmap view | `PcmView::open`(mmap)+ 融合 deinterleave |
| fallback view | `PcmView::open` を強制 `Owned` + 融合 deinterleave |
| view open のみ | mmap/Owned の open コスト単体(繰り返し) |

記録: alloc calls/bytes・dealloc・wall time(p50/p95, N=30)・
処理系 RSS(Windows GetProcess 情報等、依存追加なし。不能なら
未計測と明記)・fixture サイズ・環境。
cache 導入判定: view open が materialize 全体の一定割合(例 10%)を
超えるか、繰り返し open が e2e に有意な場合のみ検討し、結果を
impl/ir に記録する。bit-identical 検証は全3経路で
`DecodedInput`/`PreparedInput` 相当の `channel_samples` を比較する。

## Risks and Trade-offs

- mmap の主な効果は `fs::read` の 1 buffer 確保+コピー回避で、
  変換 CPU・deinterleave は残る。利益が小さい場合は per-call view
  のみで閉じ、cache は作らない(確定判断どおり)。
- Windows では mmap 中のファイル削除が失敗する側面があるが、
  artifact は job 存続中不変なので問題にならない。
- 融合 deinterleave は push ベースで `Vec::with_capacity` により
  現行と同等の最終容量を担保する(realloc は許容、bit-identical への
  影響なし)。

## Verification Notes

- [ ] 3経路(baseline/mmap/fallback)で `channel_samples` が
  bit-identical(decoded は scale 適用値、normalized は有限性検証付き)。
- [ ] `len % 8 != 0`・空ファイル・mmap 不可時に現行と同一エラー/結果。
- [ ] 計測結果と cache 判定が impl/ir に記録される。
- [ ] `cargo test -p al-locate` 既存テストが変更なしでパスする。

## Next Checkpoint

design review 承認後、plan に view 実装・caller 移行・計測・cache
判定の依存順を具体化する。plan 承認前に task は生成しない。

## 2026-10-15 Design Review

Reviewer A: spec/ADR の Deferred Concerns(所有境界・mmap 詳細・
cache eviction・計測 fixture/閾値・不変性前提)の網羅と内部整合を確認。
Reviewer B: 5候補。Validator: IC-1 融合 deinterleave と
`dsp::deinterleave` の同値性 CONFIRMED(逐語契約を明記する修正を適用)、
IC-2 mmap が計数 allocator を通らない CONFIRMED(wall+RSS 併記を明記)、
IC-3 大 fixture の CI 負荷 CONFIRMED(サイズ階層化を適用)。
IC-4 有限性検証の検査順序差は結果同値のため REJECTED、IC-5 byte_len
検証は REJECTED。Judge ACCEPT、Blocker0。
実装・計測・cache 判定の動的実証はこの承認に含まれない。
