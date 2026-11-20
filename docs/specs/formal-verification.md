---
id: "SPEC-034FormalVerification256"
type: "spec"
status: "proposed"
title: "formal verification (D 層) 導入: 昇格路と第 1 標的 WAV codec (Issue #256)"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/256"
  changes:
    added:
      - type: "file"
        path: "crates/al-media/tests/formal_wav.rs"
      - type: "file"
        path: "tools/run-formal-kani.mjs"
      - type: "doc"
        path: "docs/specs/formal-verification.md"
      - type: "dir"
        path: "evidence/al256/"
    modified:
      - type: "file"
        path: "crates/al-media/Cargo.toml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".agents/skills/verify-audio-lens/SKILL.md"
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
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# formal verification (D 層) 導入: 昇格路と第 1 標的 WAV codec

Issue #256。検証環境の理想形(`.devin/verify-env-survey/synthesis.md`
§1)が置く 7 層のうち、D 層は「高コストバグ領域への形式的検証」
を担う。本文書は D 層の設計(器の選択規則と昇格路)と、第 1 標的
WAV codec の検証構成・bounds・assumptions・coverage 記録をまとめた
正本である。実行記録(コマンド・exit code・verdict 表・変異 fail
証拠)は `evidence/al256/` の台帳が持つ。

## 器の選択規則と昇格路

採用する規則は pstack `create-formal-verification-skill` の
「問いが器を選ぶ」である。テストは書いたケースだけを調べ、
形式的検証は宣言した範囲の全ケースを調べる。器はその範囲の形で
決まる。

| 問いの種類 | 器 | 本リポジトリでの使いどころ |
| --- | --- | --- |
| 実コードを生成入力で広く打つ(サンプリング+shrink) | proptest | parser/codec の round-trip・不変量のランダム検査。`cargo test` に載り追加 toolchain 不要 |
| 実コードが任意入力で性質を保つか(bounded 網羅) | Kani | parser/codec の任意入力安全・round-trip・算術の不変量。`cargo kani` でコードに直接かかる |
| 手順・イベントの悪い順序がありうるか(設計モデル) | TLA+ | job resume/cancel・staged publish の順序検査 |
| 全サイズで成立する証明が要る性質 | Lean 4 | 解析数学の不変量・baseline compare の決定性 — 文書化された promise に限る |
| 並行 interleaving(実コード・共有状態) | loom | RT 境界の lock-free 構造があれば。現状候補少なめ |

昇格路は proptest → Kani → TLA+ → Lean とする。proptest で property
を先に言語化し、同じ property を Kani で bounded 網羅化する順が
自然であり、proptest 側は toolchain 追加なしに常時 `cargo test`
で回る最も安い網羅的手段として残る。状態機械には op 列生成の
property test を TLA+ 着手前の段として置く。

## 第 1 標的: WAV codec (`crates/al-media/src/wav.rs`)

第 1 標的には「破損がユーザーに届き、テストが届きにくい」契約を
持つ最強の対象を選ぶ。WAV codec は decode(`decode_wav`・
`wav_frame_count`・`read_wav_range`)と write(`WavWriter`)の双方
があり、同 encoding の再 encode が lossless という明示の往復契約
(wav.rs `WavEncoding` doc)まで揃う。layout 走査 `wav_layout` は
checked_add・範囲照合で任意入力への耐性を意図して書かれており、
その意図を検証対象として固定できる。`al-media` に `unsafe` は
存在しない。

### 検証する property(coverage 記録)

`crates/al-media/tests/formal_wav.rs` 一ファイルに proptest suite
と `#[cfg(kani)]` bounded proof harness を同居させる。

| property | 検証する契約 |
| --- | --- |
| `decode_wav_never_panics_on_arbitrary_bytes` | 任意 `&[u8]`(0..=4096B)で panic しない。返り値は `Ok` または `Err(MediaError)` のみ |
| `frame_count_and_range_reads_hold_the_contract` | `wav_frame_count`/`read_wav_range` も同じ no-panic 契約を持つ。layout 失敗なら全 read が失敗し、countable かつ decodable なら全 decode と frame 数・sample slice が一致する。countable だが decodable でない入力では全 read が失敗する |
| `writer_output_round_trips_losslessly` | 5 encoding × channels 1..=8 × 任意 rate × frames について、`WavWriter`→`decode_wav` が metadata(channels/rate/bits/sample_format/codec)と samples を正確に往復する(1..=3 分割 write 含む)。`wav_frame_count` と `read_wav_range` が書き出し layout と一致する |
| `writer_new_rejects_exactly_the_documented_invalid_params` | `WavWriter::new` が channels==0・rate==0・block_align/byte_rate overflow のときに限り失敗する |
| `write_samples_rejects_exactly_unaligned_lengths` | `write_samples` が frame 境界でない長さに限り失敗する |
| `mutated_or_truncated_headers_never_panic` | 有効 WAV の header field 上書き・任意位置 truncate で 3 read 入口が panic しない |
| `kani_proofs::decode_never_panics_on_bounded_input` | 48B の全バイト列で 3 read 入口が panic しない(bounded 網羅) |
| `kani_proofs::writer_round_trip_is_exact` | symbolic encoding/params の writer 出力が正確に往復する |
| `kani_proofs::header_reads_agree_with_writer_layout` | writer 出力上で frame count が一致し、u8 由来の任意 range(0..=255 の start/end)について valid なら slice が一致・invalid(start>=end, end>total)ならエラー、の全分岐を網羅する |

proptest 側が覆うのは統計的サンプリング(既定 case 数)の範囲で
あり、Kani 側は宣言した bound 内の網羅を検証する。両者は同じ
property 言語の異なる強度の段である。

### bounds・assumptions・model/product の区別

- **入力 bound**: proptest の任意 byte 列は 0..=4096B。Kani の
  decode 入力は 48B 固定(最小 RIFF/WAVE header 44B + data 数 B を
  覆い、全 `wav_layout` 分岐と数回の sample loop を通せる最小の
  実用 bound)。いずれも「任意入力の安全性」を bound 内で述べる
  もので、より大きな入力への主張は持たない。
- **writer パラメータ bound**: Kani 側は channels 1..=4・rate
  1..=192_000・1 frame と、writer の有効空間内に制約する
  (symbolic `f64` 変換が支配的な検証コストであり、1 frame で
  write → decode → slice の全経路を通せる)。proptest 側は
  channels 1..=8・rate <= u32::MAX/64・frames 0..=72 の
  ランダム範囲。
- **unwind bound**: `decode_never_panics_on_bounded_input` は
  `#[kani::unwind(56)]`、writer 系 2 proof は
  `#[kani::unwind(32)]`。初版のより大きな bound([u8;128]+
  unwind160 / 2 frames+unwind64)はこの環境で 45min cap を
  超過したため、再実行しやすい bound へ縮小した —
  trade-off の実測は `evidence/al256/` に記録する。
- **sample の表現可能性**: 整数 encoding は範囲内の整値 f64、
  Float32 は f32 表現値、Float64 は非 NaN の有限範囲値だけを
  往復対象にする(doc 契約は clamp+round / cast を約束し、任意
  値の厳密往復は約束しない)。NaN は `!=` が成立しないため除外
  する。
- **model/product 区別**: 検証が写すのは wav.rs の文書化契約で
  あり、product 意味論が正本である。fail が出たとき、
  counterexample が文書化契約に反すれば product defect、検証側が
  契約を強く読みすぎていれば model error と見分ける。具体例と
  して、`wav_frame_count` は layout 走査だけで計数するため
  「countable だが decodable でない」ファイルが存在しうる —
  これは不具合ではなく契約の形であり、suite はその区別を
  property として固定している。
- **coverage の残り**: `WavEncoding` は PcmU8/PcmS8 を emit でき
  ない(decode-only)ため、writer 起点の round-trip は 5 encoding
  に限る。PcmU8 の decode は mutated/structured 入力側の
  no-panic 契約と、`read_wav_range` 経路の一部として間接に覆う。

## 昇格路: 第 2 標的以降の候補

| 器 | 候補標的 | 根拠 |
| --- | --- | --- |
| Kani | MP4 `decode_mp4`(`mp4.rs:1187`、純粋 `&[u8]` 入口)・AVI `decode_avi`(`avi.rs:370`、`VideoFeatureConfig` 併用)の任意入力安全+round-trip | WAV と同型の hardened layout を持ち、第 1 標的の harness 形がそのまま移植できる |
| Kani | `al-job` job 状態遷移関数・tolerance compare 算術 | 単位の小さい決定的関数で bounded 化しやすい |
| TLA+ | `al-job` `runner.rs` の resume/cancel、`fsutil` staged publish の順序 | 順序・設計レベルの問いであり、設計モデルとして写す。pstack `statechart` skill で結果を解釈する |
| Lean 4 | `al-locate`/`al-analysis` の解析数学不変量(spectral/feature 恒等式)・baseline compare の決定性 | 全サイズで成立する性質で、文書化された promise に限る |

synthesis.md §4.3 の Kani 標的表には `RevisionedStore` 不変量
(revision 単調・CAS 意味) が挙がるが、実体は `ui/src/view-state.ts`
の TypeScript 実装であり Rust Kani の標的に載らない。Rust D 層
では対象外として扱い、検証が要るなら TypeScript 側の property
test(fast-check 等)で別途扱う — 同じ property 名を勝手に
Rust 標的へ読み替えないのが、model/product 区別の約束である。

## Kani 実行環境と provision runbook

`cargo kani` は Windows native では動かず、Linux/macOS が対象。
本 lane は `tools/run-formal-kani.mjs` が実行環境を検出する形に
なっている(`mise run test-formal-kani` が入口)。

1. native(Linux/macOS)で `cargo kani` が使えればそのまま実行。
2. Windows では `wsl.exe -l -q` の distro を列挙し、WSL 内の
   `cargo kani --version` が通る distro を使う。`/mnt` 経由の
   ビルド I/O を避けるため、cargo が必要とする source tree
   (Cargo.toml/Cargo.lock/.cargo/crates/dictionaries —
   al-core/build.rs の lockfile 走査と al-media lib test の
   crate 外 include_str! が要る最小集合)を
   `~/.cache/al-formal-kani/src` へ mirror してから実行する。
   cargo の target-dir 設定・env 変数は変更せず、build 出力は
   mirror 内の既定 `./target` に収まり、Windows 側 repo の
   `target/` には異 OS の artifact を書き込まない。mirror 更新は
   `target/` だけ保持して source を再展開するため、dep build
   cache は再実行間で持続する。
3. 環境不在・未 provision では verdict `not_run`/`unsupported` +
   理由 + provision 手順を出力して exit 0。環境不在は lane
   失敗ではない(POLICIES §3)。実走して verification 結果が
   出れば `pass`/`fail` で exit 0/1。kani-driver は harness ごとの
   結果を process 終了まで flush しないため、lane は各
   `kani_proofs` proof を `cargo kani -p al-media --tests
   --harness <name>` で逐次実行し、完了した proof の verdict を
   個別に保持する(`#[test]` proptest は対象外)。

provision 手順(WSL2 Ubuntu 内):

```bash
cargo install --locked kani-verifier
cargo kani setup    # CBMC toolchain を ~/.kani に展開する
```

## 検証と再実行

正規入口は mise task である。

- `mise run test-formal`: proptest lane。native で常時実行できる
  第 1 標的の 1 コマンド再実行。
- `mise run test-formal-kani`: Kani lane。env-detect の結果に
  応じて pass/fail/not_run/unsupported を JSON verdict で返す。
- 変異 fail 証拠・実行環境・sync commit SHA・verdict 表は
  `evidence/al256/verification.md` と `actions.jsonl` が持つ。

## Sources and Relations

- Issue: <https://github.com/scarecrow173/audio-lens/issues/256>
- 設計正本: `G:/repositry/audio-products/.devin/verify-env-survey/synthesis.md` §4.3
- 対象コード: `crates/al-media/src/wav.rs`・`crates/al-media/tests/formal_wav.rs`
- lane runner: `tools/run-formal-kani.mjs`
- 証拠台帳: `evidence/al256/`
- 関連 skill: `.agents/skills/verify-audio-lens/SKILL.md` の
  Formal verification 節
