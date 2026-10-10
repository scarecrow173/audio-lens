---
id: "ADR-034DxdUT164IPZdcbKOLQo"
type: "adr"
status: "accepted"
title: "MediaDecoder は native 優先の registry と段階的 backend 導入を採用する"
created: "2026-10-10"
updated: "2026-10-10"
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
    - "../designs/media-format-analysis-extensions.md"
    - "../specs/media-ingest.md"
  implemented-by:
    - "../tasks/media-decoder-native.md"
  depends-on:
    - "implementation-stack.md"
  blocks:
    - "../tasks/media-decoder-native.md"
  supersedes: []
  superseded-by: []
  related: []
  refines:
    - "mp4-demux-decode.md"
  refined-by: []
  derives-from:
    - "../tasks/media-decoder-adr.md"
    - "../designs/media-format-analysis-extensions.md"
    - "../specs/media-ingest.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/media-format-analysis-extensions.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# MediaDecoder は native 優先の registry と段階的 backend 導入を採用する

## Context and Problem Statement

2026-10-10 に coordinator が本具体案を承認した。
TASK-036a(#36 フェーズ0)は、approved design の必須ゲートとして、
[フェーズ1](../tasks/media-decoder-native.md)より前に backend 境界を固定する。
現在の `al-media` は WAV/AVI の独自 parser と MP4 demux を持ち、
`stage.rs` が形式ごとに直接分岐する。AAC/H.264 の decode 経路はない。
新しい codec を追加する前に、選択・出力・部分失敗・来歴の責務を統一する。

native は「既存のリポジトリ内 Rust parser/decoder」を意味し、FFmpeg の
native library binding を意味しない。本 ADR の実装範囲は既存能力の移行のみ。
codec 対応拡大、外部依存導入、ROI 機能追加、リサンプル実装は含めない。

## Decision Drivers

* [media-ingest spec](../specs/media-ingest.md)の元 PCM、channel、独立 clock、
  decode provenance と explicit failure を保持する。
* [approved design](../designs/media-format-analysis-extensions.md)の native 優先、
  能力不足だけの fallback、部分観測保持を一つの境界で実現する。
* 既存の ingest/golden を維持し、新規依存・配布要件をフェーズ1に持ち込まない。
* backend ごとの demux 差や scale 差を下流 caller の分岐で補償しない。

## Considered Options

| 候補 | codec/観測能力 | 配布・再現性 | 今回の判断 |
| --- | --- | --- | --- |
| A: FFmpeg 外部プロセスを主 backend にする | 音声・映像 decode と probe が可能。build の codec 構成に依存 | executable 配布、version/build 固定、入出力と process failure の制御が必要 | 主 backend として不採用。後続の明示有効化 fallback 候補 |
| B: ffmpeg-next/ffmpeg-sys binding を主 backend にする | library API で demux/decode と timestamp を取得できる | FFmpeg library の build/link/ABI と Rust wrapper の対応管理が必要 | 不採用。既存移行に FFI と配布依存を加える |
| C: Symphonia 音声 decode + 自前 demux | 純 Rust の音声 decoder。映像 decode を埋めない。packet/codec 設定の adapter が必要 | Rust crate/feature 固定で構成できるが、依存追加と適合 fixture が必要 | 今回導入しない。音声拡大時の第一評価候補 |
| D: MediaDecoder trait + native 優先 registry、段階的移行 | 現行 WAV/AVI decode と MP4 timing を維持。codec 範囲は下表で固定 | 新規 runtime/build 依存なし。将来の backend を同じ出力境界へ追加 | 採用 |

D は codec library の代替ではなく、それらを導入する前の責務と導入順の決定。
A/B/C を直ちに主 backend に置き換える案と比較した。
公式資料の参照日は 2026-10-10。性能優劣は測定していない。

## Decision Outcome

Chosen option: "D: native 優先 registry と段階的移行", because 現行能力を
保持したまま backend 選択と出力契約を集中でき、依存導入を別の承認単位にできるため。

フェーズ1では `native` 一件だけを登録する。後続の音声拡大では Symphonia を
最初に評価し、それで満たせない能力に FFmpeg 外部プロセスを評価する。
これは依存追加の承認や AAC/H.264 対応の約束ではない。FFmpeg binding は採用しない。

### Native Backend Contract

#### detect / probe / decode の責務

`MediaDecoder` は backend ID/実装 version と次の境界を提供する。
具体的な Rust 型名は native タスクで決めるが、以下の情報・結果区分は必須とする。

* `detect(bytes)` は先頭 signature による match/no-match。拡張子を参照しない。
  WAV/AVI は RIFF form、MP4 は先頭 box の `ftyp` のみを認識する。
  signature match は構造の妥当性や decode 能力を保証しない。
* `probe(source)` はコンテナと stream ID、種別、codec、rate/layout、元 clock、
  取得可能な PTS/DTS/packet 長と要求別 capability を返す。PCM decode は行わない。
  detect/probe/decode は同じ pinned input を参照し、stage が hash を検証する。
* `decode(source, probe, request)` はコンテナ demux と codec decode を backend 内に
  一体で所有する。demux packet を公開する別 trait は今回作らない。
  request は audio/video/timing の要求と対象 stream ID を持つ。
  フェーズ1は既存の選択規則に限定し、AVI は最初の `auds` / `vids` を選び、
  全 stream の timing を保持する。任意 stream decode の追加はしない。
* 結果は共通 ingest result とし、stream ごとの decoded/unavailable と理由、
  metadata、timing、integrity observations、backend provenance を一つにまとめる。
  timing-only 成功は PCM decode 成功と区別する。backend は成果物を登録しない。

既存 parser が probe/decode を一体で返している場合、構造を解析した内部状態を
adapter 内で再利用してよい。probe のために PCM を先行生成したり parser を
別実装へ置換したりせず、既存検証ロジックを責務の所有者として再利用する。

#### フェーズ1の能力表

| コンテナ | 音声 decode | 映像 decode | probe/demux/timing |
| --- | --- | --- | --- |
| WAV RIFF/WAVE | format tag 1: unsigned 8 / signed 16・24・32 bit PCM、tag 3: IEEE float 32・64 bit | 対象外、要求時 unavailable | 現行 WAV metadata と sample clock |
| legacy AVI RIFF/AVI | 最初の auds、現行 WAVEFORMATEX parser が扱う上記 PCM/float | 最初の vids、BI_RGB DIB 24/32 bpp、top-down/bottom-up | 全 stream の現行 clock・packet 観測 |
| MP4/ISO-BMFF、先頭 ftyp | 全 codec unavailable。AAC/PCM を含め decode しない | 全 codec unavailable。H.264 を含め decode しない | 現行 mp4.rs の track/sample-table/PTS/DTS 観測のみ |

FLAC/MP3/AAC、圧縮 AVI、H.264/HEVC、未対応 RIFF variant や MP4 構造を
新たに decode/demux 対応にしない。現行 parser が拒否する構造を推測して補完しない。

#### PCM / frame / clock の共通表現

正本は既存 `DecodedAudio`。`samples: Vec<f64>` を frame-major の channel
interleaved とし、`sample_rate > 0`、`channels > 0`、sample 数は channel 数の
整数倍を満たす。元 rate、channel 順、bits_per_sample、SampleFormat、codec を残す。
integer PCM は整数値をそのまま保持し、unsigned 8 bit は 128 を引いた値とする。
float32 は f64 へ拡張、float64 は元値のまま。整数を [-1, 1] に scale しない。
数値の IEEE 特性も保持し、非有限値をゼロへ置換しない。
channel layout が未取得なら unknown とし、stereo 等を推測しない。

保存する `decoded_pcm` は現行どおり interleaved little-endian f64。
`normalized_pcm` や analysis 用 scale 変換は別処理の責務であり、backend が暗黙に
resample/downmix/gain normalization しない。変換を追加する際は元/後 rate/layout、
scale、設定と処理順を一箇所の履歴へ記録する。

DIB pixel decode は native が所有し、top-down の luma plane と width/height、
元 frame tick/timebase/start、必要な RGB preview を共通特徴抽出へ渡す。
全画面特徴/thumbnail の現行結果を維持し、ROI 算出を decoder 内に追加しない。
frame は全数保持せず逐次処理する。音声と映像の clock を別々に保持し、PTS/DTS
の欠測は欠測のまま、公称 FPS で timestamp を作らない。既存 gap/overlap を保持する。

#### エラー / 部分失敗 / registry 選択

registry が登録済み backend の固定順と能力選択を所有する。native が最優先。
将来の順序は native → 承認済み純 Rust 音声 backend → 承認済み FFmpeg fallback。
未導入 backend を登録せず、同じ backend を繰り返し試さない。

| 結果区分 | 扱い | fallback |
| --- | --- | --- |
| unknown format / unsupported codec または要求能力 | 明示の capability unavailable。理由と要求を保持 | 登録済みで要求に適合し、使用許可された次候補だけ |
| コンテナ破損・切断、I/O、hash 不一致 | 入力全体の失敗。成功成果物を登録しない | 禁止 |
| AVI stream header/payload の局所破損 | 当該 stream の malformed unavailable、独立して有効な timing/他 stream は保持 | 禁止 |
| backend 起動失敗、runtime failure、設定不正 | capability 不足と区別した失敗 | 禁止 |

現行 AVI adapter は文字列化前の `MediaError` を見て stream の unsupported と
malformed を区別する。文字列の substring 判定で fallback 可否を決めない。
映像 parser の unsupported/破損も native 境界で理由区分を保持する。
stream 局所失敗の保持は現行 AVI の責務であり、壊れたコンテナ全体を部分成功に
昇格させない。失敗 stream の途中までの PCM/frame 特徴を完成品として保存しない。

stage が共通 result に基づく部分 ingest ポリシーを一箇所で適用する。
WAV の audio decode 不可は input failure。AVI/MP4 は独立して有効な
container/timing 観測を保存し、audio/video unavailable を明記して ingest を継続する。
PCM がない場合 `decoded_pcm` を生成せず、無音による穴埋めもしない。
辞書は観測の注釈だけに使う。後段は成果物 availability を見て必要入力の不足を
扱い、backend ID で処理を切り替えない。

将来 fallback で満たすのは不足した要求だけとし、成功済み native 観測を上書きしない。
stream ID と clock を整合できない候補は unavailable とする。
FFmpeg は compile-time feature と job の明示許可の両方を必要とし、default は無効。
暗黙の PATH 探索や自動 download は許可しない。

#### provenance / デプロイ

共通 result から stage が input hash、選択 stream/要求能力、試行順、backend ID、
実装 version、設定、結果区分と理由、fallback 有無を成果物 provenance に記録する。
native の version は `al-media` package version、設定は今回の選択規則と
「変換なし」を含める。capability 不足で未実行の候補と実行済み試行は区別する。

フェーズ1は既存 Rust toolchain と workspace 依存のみ。新規 crate、FFmpeg
executable/shared library、OS codec、network、環境変数による有効化を要求しない。
将来外部 backend を導入する際は dependency/version/feature の承認、各 OS の
配布・license 構成、実行 budget、version/build/config 記録を別タスクで確定する。

### 既存 ADR との関係

[mp4-demux-decode.md](mp4-demux-decode.md)は accepted のまま維持する。
本 ADR は `relations.refines` による補足(amend)であり supersede しない。
MP4 codec decode をスコープ外とする判断を変更せず、「将来の別決定」を具体的な
backend 境界と導入ゲートにした。[avi.md](avi.md)の非圧縮 DIB 限定も維持する。
MP4 codec decode を追加する後続 ADR が成立する時点で demux-only の判断を
supersede/amend する。native 移行はその追加決定を待たず本 ADR に従って進められる。

### Consequences

* Good, because parser の検証、backend 選択、stage の登録責務が明確になり、
  現行 PCM/timing の契約を保ったまま次の codec backend を比較できる。
* Good, because フェーズ1に配布依存が増えず、破損を fallback で隠さない。
* Bad, because AAC/H.264 入力の音声/映像解析は引き続き unavailable。
  codec 拡大は依存承認と後続計画が必要。
* Bad, because stream エラーの型付き区分と共通 result への adapter が必要。
  これは native 移行タスクで実装し、caller ごとの補償を廃止する。

### Confirmation

TASK-036a の文書チェックと、TASK-036b の runtime 検証は区別する。
本 ADR の accepted 化は backend 契約の決定であり、decode 実装が済んだ証拠ではない。
下記 Verification の実装項目は native タスクで実行する。

## Pros and Cons of the Options

### A: FFmpeg 外部プロセス

* Good, because codec 対応を process に委ねられる。
  [公式 CLI](https://ffmpeg.org/ffmpeg.html)は stream 選択と decode の設定を提供する。
* Bad, because raw PCM だけでは元 packet の clock・完全性・channel 来歴を保存できず、
  probe 結果との対応付けと実行管理が別途必要。今回の既存移行には過大。

### B: FFmpeg Rust binding

* Good, because [ffmpeg-next](https://github.com/zmwangx/rust-ffmpeg)は
  FFmpeg library API の wrapper として細かな decode 情報を扱える。
* Bad, because FFmpeg library 依存と wrapper 対応を配布環境ごとに管理する。
  純 Rust の既存 backend 移行という制約に対して採用理由がない。

### C: Symphonia + 自前 demux

* Good, because [公式 README](https://github.com/pdeljanov/Symphonia)にある
  AAC/FLAC/MP3 等の音声 decoder を純 Rust backend として評価できる。
* Bad, because 音声 library なので H.264 映像 decode の解決にならず、
  自前 demux の packet/codec 初期化・clock 対応の検証は必要。今回の依存なし移行には不要。

### D: native 優先 registry

* Good, because 既存 parser を再利用し、一つの共通 result と選択方針で段階的に拡大できる。
* Bad, because trait/adapter の整理だけでは codec coverage が増えない。

## Implementation Plan

* Affected paths: `crates/al-media/src/`、`crates/al-testkit/`。
* Patterns to follow: 既存 parser の構造検証と `DecodedAudio` を再利用する。
* Patterns to avoid: 拡張子 dispatch、文字列による fallback 判定、backend 内の成果物登録。
* Dependencies or configuration: 新規依存なし、registry は native のみ。

1. [native タスク](../tasks/media-decoder-native.md)で
   `crates/al-media/src/decoder.rs` に trait、共通 result と registry を定義する。
   `format.rs` の signature 判定、`metadata.rs` の既存エラー/PCM metadata を再利用し、
   capability 不足と局所破損の結果区分を一箇所で定義する。
2. `wav.rs` / `avi.rs` / `mp4.rs` を native adapter から呼び出す。
   stream エラーは string 化する前に区分を残す。parser の置換、codec 追加はしない。
3. `video.rs` の DIB pixel decode と共通特徴抽出を分離し、既存全画面・thumbnail
   結果を維持する。`stage.rs` の直接 format 分岐と `MuxObservations` の統合責務を
   共通 result に移し、hash/辞書/登録/部分 ingest 判定を stage に残す。
4. `lib.rs` の公開境界を更新する。Cargo manifest/lock、mise toolchain に依存追加なし。
   provenance の出力契約を更新し、リリース前方針に従い成果物を再生成する。
5. 既存 `wav.rs` / `avi.rs` / `mp4.rs` / `stage.rs` / `video.rs` の tests と
   `crates/al-testkit` fixture を用い、下記 contract を検証する。

## Verification

### TASK-036a: 文書

- [x] `node .agents/skills/adr-doc/scripts/review_adr.js --file docs/adr/media-decoder-backend.md` が exit 0。
- [x] ADR の front matter、関係リンク、README 索引が整合し、native の前提として accepted。
- [x] `git diff --check` が exit 0。変更は ADR と task/index 文書のみ。

### TASK-036b: 実装時、未実行

- [ ] `mise exec -- cargo test -p al-media` が exit 0、既存 ingest/golden の観測値を維持する。
- [ ] signature match/no-match と固定 registry 順を検証し、未実装 backend が登録されない。
- [ ] PCM 全対応幅の整数値/float 値、channel interleave、rate/元 scale、f64le 保存を確認する。
- [ ] MP4 AAC/H.264 は decoded 不可、timing-only の成功と理由付き unavailable を保持する。
- [ ] WAV unsupported は失敗、AVI stream 局所破損は他の有効観測を保持し、破損/I/O は fallback しない。
- [ ] provenance に hash、native/version、設定、要求能力、試行と理由、fallback=false が残る。
- [ ] DIB top/bottom-up、全画面/thumbnail、独立 clock と欠測 timestamp の現行結果を維持する。

## More Information

### 未決事項と再検討条件

* codec 拡大時の Symphonia version/features、AAC profile/fixture の適合範囲は未決。
  依存承認と後続 codec 計画で決める。native フェーズ1の前提ではない。
* FFmpeg fallback の version/build 配布、各 OS の license 構成、process timeout/
  output budget、異なる demux 間の stream/clock 対応は導入前に決める。
  この条件が満たされるまで登録・有効化しない。
* native 実装で上記の出力/失敗区分を現行 parser から保持できない場合は、
  caller の workaround を増やさず本 ADR の契約との不一致を報告して再検討する。
