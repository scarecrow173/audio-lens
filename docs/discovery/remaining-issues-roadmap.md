---
id: "DISC-034FcymLiMavdGUpbV1oTT"
type: "discovery"
status: "active"
title: "残余 issue ロードマップ分析: 13 open issue の優先度・依存順序・フェーズ構造"
created: "2026-10-13"
updated: "2026-10-14"
owners: []
relations:
  source:
    - "../../target/audio-lens-issue-atlas.html"
    - "https://github.com/scarecrow173/audio-lens/issues"
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
  related:
    - "./mechanical-caller-cleanup.md"
    - "./caller-side-optimizations.md"
    - "./al-math-simd-reduction.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "./locate-consensus-argmax-ambiguity-contract.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 残余 issue ロードマップ分析: 13 open issue の優先度・依存順序・フェーズ構造

## Exploration Goal

GitHub open issue 13 件の残件を、PR #44/#45/#49/#50/#51 のマージ後状態を
踏まえて再分析したロードマップレポート(HTML ダッシュボード
`audio_lens_issue_dashboard.html`)の内容を、正本記録として doc グラフへ
転記する。対象は issue の棚卸し・優先度(P0-P3)・工数(S/M/L/XL)・
依存順序 DAG・フェーズ構造・issue ごとの設計提案(decision/steps/tests/
options/risks)であり、今後の spec/ADR/design チェーンの着手順を決める
入力とする。

**重要な定位**: 本記録の推奨順位・設計案・依存線はすべてダッシュボード
レポートの**提案**であり、GitHub 上で合意された blocking dependency や
issue label ではない(ダッシュボード `methodology` 明記)。依存 edge の
source→target は「先にやるとよい」推奨順序であり、GitHub の blocker
関係ではない。また `references` 上で #40→#42・#42→#40 と相互参照が
あるが、これは**循環ブロッカーではない**(相互に関連するだけで、DAG
edge としては #42→#40 の任意参照 1 本のみ)。

一次成果物(可視化コンパニオン)は `target/audio-lens-issue-atlas.html`
(untracked artifact。原本は `C:\Users\akiyoshi\Downloads\
audio_lens_issue_dashboard.html`)。本ドキュメントは同ファイル内の
`<script id="dataset" type="application/json">` JSON を機械抽出して転記した
もので、手転記ではない。

## Methodology と出典

ダッシュボード JSON の `methodology` フィールド(ほぼ verbatim):

> GitHub Open Issue 13件の本文・全コメント、PR #44/#45/#49/#50/#51の
> マージ状況と関連資料を照合。推奨順位・設計案・依存線は本レポートの
> 提案でありGitHubで合意されたblocking dependencyではない。性能値は
> Issue/PR記載の測定条件に限る。

- repository: `https://github.com/scarecrow173/audio-lens`
- snapshot: **2026-10-13 JST**

## Issue 棚卸し(13 件)

| Issue | Title | Domain | Prio | Effort | Stage | Phase |
| --- | --- | --- | --- | --- | --- | --- |
| #59 | 合議argmaxのFPノイズ依存を解消 | Locate | P0 | M | 未解決 | 0 |
| #36 | 主要コーデックの実デコード対応 | Media | P1 | XL | 一部実装※ | 1 |
| #52 | NCC plan/workspaceの再利用 | DSP | P1 | M | 未着手※ | 1 |
| #56 | 同一入力PCMの再デコードを回避 | Ingest | P1 | M | 実装済み | 1 |
| #42 | FFTカーネルの性能目標・再評価 | DSP | P2 | L | 一部実装※ | 1 |
| #55 | al-datasetのfsync頻度を再設計 | Dataset | P2 | M | 未着手※ | 1 |
| #33 | spec #1への残差適合 | Contract | P3 | L | 一部実装※ | 1 |
| #53 | local_lagの全探索を高速化 | Reference | P1 | L | 未着手 | 2 |
| #40 | coarse-to-fineと帯域cacheの残件 | Locate | P2 | L | 一部実装 | 2 |
| #57 | envelopeを線形時間化 | Cheap | P2 | M | 未着手 | 2 |
| #58 | matrix_to_tensorコピーを除去 | Dataset | P3 | M | 未着手 | 2 |
| #54 | cheap/reference stage内部の並列化 | Runner | P1 | XL | 未着手 | 3 |
| #60 | 残余小項目の性能棚卸し | Polish | P3 | S | 未着手 | 4 |

※ 2026-10-14 の deep-dive セッションで Phase 1 各 issue(#52/#56/#36/#42/
#55/#33)の設計決定が**確定**済み。今回の確定は実装完了を意味せず、Stage の
実装状態は変わらない(#33 の sample_rate は既に実装済み)。確定内容は各 issue セクションの「確定決定」を参照。

## 依存 DAG(11 edge — Phase 1 確定順序を反映した推奨順序)

| Source | → | Target | Strength | Reason |
| --- | --- | --- | --- | --- |
| #59 | → | #42 | 推奨 | FFT/相関の数値挙動を変更する前に曖昧位置の判定契約を決める |
| #59 | → | #40 | 推奨 | 間引き/帯域検索の変更前に周期・ループ誤受理を固定 |
| #36 | → | #56 | 推奨 | デコーダ非依存PCM handleの責務境界を先に揃える |
| #52 | → | #42 | 確定順序 | plan 再利用後に再計測し、kernel 交換の必要性を判定 |
| #42 | → | #40 | 任意 | bandpass方式見直しとFFT backend設計を整合させる |
| #42 | → | #53 | 任意 | FFT kernel・実数サイズ形状を考慮してlocal_lagを設計 |
| #52 | → | #53 | 推奨 | 高頻度窓NCCでplan/workspace再利用を活用 |
| #52 | → | #54 | 推奨 | parallel workerごとのworkspace管理を先に設計 |
| #56 | → | #54 | 推奨 | 共有PCMの寿命と並列メモリ予算を先に確定 |
| #40 | → | #54 | 任意 | pair間cache共有のスレッド安全性を先に定義 |
| #55 | → | #54 | 任意 | commit durabilityとordered writeの境界を合意 |

## フェーズ構造

| Phase | Issues | 意味 |
| --- | --- | --- |
| 0 | #59 | 正しさの契約固定(最優先・先行ゲート) |
| 1 | #52 #56 #36 #42 #55 #33 | 基盤整備(plan/cache/codec/kernel 再評価/durability/仕様残差) |
| 2 | #53 #40 #57 #58 | Phase 1 基盤を使った局所高速化 |
| 3 | #54 | stage 並列化(共有 cache・保存契約が揃った後) |
| 4 | #60 | 残余小項目の仕上げ |

## References マッピング(issue → 関連 issue/PR)

ダッシュボード JSON `references` フィールドの転記(GitHub # 番号、
issue/PR 混在):

| Issue | 関連番号 |
| --- | --- |
| #33 | #1, #44 |
| #36 | #3, #44 |
| #40 | #5, #37, #42, #44 |
| #42 | #40, #44, #45 |
| #52 | #42 |
| #53 | #52 |
| #54 | #52, #55, #56 |
| #55 | #54 |
| #56 | #36, #54 |
| #57 | #51 |
| #58 | #51 |
| #59 | #49 |
| #60 | #49, #51 |

## Issue 別の詳細(decision / steps / tests / options / risks)

### Phase 0

#### #59 合議argmaxのFPノイズ依存を解消 — P0 / M / 未解決 / Locate

- **概要**: 周期NCCの誤受理を構造的に防止
- **現状**: 周期入力で逐次NCCの丸め誤差が偶然ピークを散らし、曖昧位置の誤受理を抑えている。PR #49は安全性を優先して置換を保留。
- **ギャップ**: corr0へ置換すると相関1.0の同率ピークがlag 0に集まり、複数法の合意が成立して誤受理する。曖昧性判定と合議の前提条件が未定義。
- **提案決定**: 「位置が特定できる」の必要条件を、最大スコアだけでなくピークの一意性・分離度・周期性で明文化する。
- **手順**:
  1. 周期列・完全同率・近接タイ・微小ノイズ・反転位相のfixtureを固定する。
  2. methodごとに上位Kピーク、ピーク間隔、同率許容幅を計測し ambiguous を出す。
  3. 合議の入力に曖昧性と候補分布を含め、uncertainを許可。任意タイブレークによる確定を禁止する。
  4. 黄金テスト固定後にcorr0再導入を性能比較し、特定可能性が保たれるか検証する。
- **テスト**:
  - `pinned_periodic_everywhere_not_accepted_at_wrong_phase` が再現性を持って不受理
  - ピーク同率、近接ピーク、非周期信号、CPU dispatch差異で決定的
  - 曖昧ケースは誤ったvalidではなくuncertainと証跡を出力
- **選択肢**:

  | Option | Change | Pros | Cons | Generality | 採用 |
  | --- | --- | --- | --- | --- | --- |
  | A argmaxの決定的タイブレーク | 同率なら小lag・既知prior順で選ぶ | 差分が最小、結果の再現性は改善 | 誤った候補自体は受理される。構造問題の修正にならない | 低 | — |
  | B ピーク曖昧性を第一級出力化 | top-K peak群、prominence/周期間隔を保存し、手法合議をuncertain対応へ | 誤受理を抑え、他の周期・ループ素材にも一般化できる | 観測型・合議判定・goldenに変更が及ぶ | 高 | ○ |
  | C 周期モデル/自己相関による検証 | 周期性推定と候補位相群を明示して再評価 | BGMループ等の複雑な入力に強い | CPU負荷・閾値/モデルが増え調整が必要 | 高 | — |

- **リスク**: 同率許容幅の選択が音量・長さ・サンプルレートに依存し得る / 拒否しすぎると低SNR等の合法マッチも失う
- **影響**: al-locate, 合議・Validity, golden, 数値決定性
- **対象ファイル**: `crates/al-locate/src/methods.rs`, `crates/al-locate/src/stage.rs`, `crates/al-testkit/`
- **証拠**: [PR #49：意図的なcorr0置換見送り](https://github.com/scarecrow173/audio-lens/pull/49)
- **優先度の理由**: 他の高速化より先に、正しい位置を受理する条件を固定する。
- **備考**: 優先度P0は本レポートの提案でありGitHub labelではない。

### Phase 1

#### #52 NCC plan/workspaceの再利用 — P1 / M / 未着手 / DSP

- **概要**: 大量NCCのFFT準備と確保を削減
- **現状**: Issueによればband_gain::windows等で約98K回のncc呼出しがあり、毎回RealFft::new(16384)と複数Vec確保が発生。
- **ギャップ**: FFT plan・scratchの繰り返し構築。公開API、スレッド所有権、shape keyed cache未設計。
- **提案決定**: 不変のFFT planと可変workspaceを分離。既存ncc wrapperを残し、明示ctx APIで内側ループのみ最適化。
- **確定決定(2026-10-14)**: **案B + 分離型で確定**。不変 `RealFftPlan`(FFT長紐づけ、Send+Sync 共有可)と、呼出側/worker が排他所有する mutable Workspace を分離し、`plan.ncc(&mut ws, template, signal)` 形とする。基盤は汎用化し、NCC と GCC-PHAT が FFT plan を共有できる構造にする(PHAT 正規化は実行時指定とし plan には持たせない)。呼出し側移行は `band_gain::windows`(約98K calls)と `cheap.rs` IPD drift(約1.8K calls)の**2サイト限定**。al-locate の ncc 系は shape が入力依存で plan ライフサイクル設計が複雑なため対象外(将来プロファイルで有意なら別途)。TLS キャッシュ・scratch pool は棄却。`ncc()`/`gcc_phat()` フリー関数は維持。
- **前提条件(2026-10-14 発見の既存バグ)**: `cheap.rs:1278-1280` の IPD drift は `xa`/`ya` が同長 `block` のため、`ncc` 内で `max_lag` が `signal.len()-template.len()=0` に clamp され、drift 計測が実質 lag 0 しか見ていない既存バグ。Plan API 移行前に独立して修正・検証する。
- **手順**:
  1. 呼出shapeのヒストグラムとallocate回数をプロファイルして基準値を固定。
  2. CorrelationPlan/CorrelationWorkspaceを設計し、key=(FFT長,手法,精度,backend)を検討。
  3. workspaceはworkerまたは呼出側が所有しmutable共有を禁止、memory budgetとLRU等を決める。
  4. ホットサイトから置換、初回cold/steady-state両方を計測。
  5. 将来のFFT kernel変更(#42)を同じ公開APIで吸収できるかテスト。
- **テスト**:
  - 既存nccとの差は許容誤差範囲・境界ケース全網羅
  - 98K callsケースでplan生成回数・alloc数が大幅削減
  - 並列呼出・異なるshape・空入力で競合なし
  - cold/warm、p50/p95、RSSを記録
- **選択肢**:

  | Option | Change | Pros | Cons | Generality | 採用 |
  | --- | --- | --- | --- | --- | --- |
  | A thread_local plan/workspace cache | 従来ncc内部でshape keyed TLSを持つ | 既存caller変更が少ない | 寿命不透明、スレッド数×shape数のメモリ増、制御難 | 中 | — |
  | B 明示CorrelationPlan + per-worker Workspace | 不変plan共有・scratch排他所有の再利用APIを追加 | 所有権とRSSが明瞭で並列化しやすく将来kernel交換可能 | APIとcaller修正が増える | 高 | ○ |
  | C 小規模なscratch pool | 型を隠すpoolからbuffer借用 | caller表面が少なく動的jobに適応 | ロック競合と長時間借用リスク | 中 | — |

- **リスク**: thread_local無制限キャッシュはRSS膨張 / バックエンド固有型の公開露出 / PlanとScratchを同じArcで共有するとデータ競合
- **影響**: al-math, al-locate facade, al-analysis, 並列化時RSS
- **対象ファイル**: `crates/al-math/src/`, `crates/al-locate/src/dsp.rs`, `crates/al-analysis/src/band_gain.rs`, `crates/al-analysis/src/cheap.rs`
- **証拠**: [caller側最適化の残件調査](https://github.com/scarecrow173/audio-lens/blob/main/docs/discovery/caller-side-optimizations.md), [機械的cleanupの除外対象](https://github.com/scarecrow173/audio-lens/blob/main/docs/discovery/mechanical-caller-cleanup.md)
- **優先度の理由**: 反復回数の根拠があり、他のFFT利用と並列化の共通基盤になる。

#### #56 同一入力PCMの再デコードを回避 — P1 / M / 実装済み / Ingest

- **概要**: 入力PCMをjob内で再利用する
- **現状**: Issueによれば同一入力を約3回デコード、対象例は137MB級。
- **ギャップ**: job/stageを跨ぐPCMライフタイム・メモリ上限・decode backendとの境界未決定。
- **提案決定**: cacheableなDecodeHandleをjob内に導入し、サイズ上限・参照カウント・再デコードfallbackを設ける。
- **確定決定(2026-10-14)**: 正本は manifest の `decoded_pcm` artifact(`decoded.f64le`、interleaved f64 LE)とし、Artifact ID を識別子とする。メモリキャッシュは正本ではなく artifact の派生ビュー。共有表現は Artifact ID に紐づく job-scoped bounded cache の read-only PCM ビューとし、mmap を第一候補・`fs::read` fallback を用意する。`Stage` trait 契約(stage 間通信は manifest Artifact ID のみ・隠れた共有状態禁止)を維持し、`DecodedInput` の stage 間直接受け渡しは採用しない(共有は不変 PCM ビューのみ)。deinterleave 済み `Vec<Vec<f64>>` はキャッシュせず、消費側が必要 ch・範囲を正規化+deinterleave する(全長必要ならその範囲だけ materialize)。#56 では部分読み取り対応への全体改修はしない。resume で manifest+artifact から独立復元可能であり、cache hit/miss・eviction で解析結果が不変であること。materialize PCM と worker scratch には別メモリ予算を設ける(mmap だけでは RSS 上限を保証しない)。
- **検証前提(2026-10-14)**: OS ページキャッシュで既に物理 I/O が消えている可能性があるため、`fs::read` vs mmap vs 直接 deinterleave を実測比較する(alloc 量・変換 CPU・peak RSS・e2e)。job-scoped cache の利益が小さければ cache 管理を追加せず per-stage mmap ビューのみも許容。
- **実装結果(2026-10-15)**: `PcmView`(mmap 第一候補・owned fallback・融合 scale+deinterleave)を `crates/al-locate/src/decode.rs` に実装し、`decode_input` / `read_normalized_record` を移行。36MB・368MB fixture の計測で mmap `open` は materialize の 0.00-0.04% のため **job cache は不採用・per-call view で確定**。alloc bytes は baseline 比 1/3、p50 は 1.13-1.14x。3経路 bit-identical。詳細は `docs/impl/ir/pcmview-measurement-cache-decision.md`。
- **手順**:
  1. 再デコード発生箇所・実データ量・ステージ跨ぎ参照寿命をtracingで可視化。
  2. decoder backendから独立したNormalizedPcmIdとPCM handleを定義。
  3. job scoped LRU/容量上限を決定し、巨大入力はmmapまたは再デコードへfallback。
  4. #54のworker数を掛けたピークRSS見積もりと失効/ファイル変更対策を確認。
- **テスト**:
  - 同一入力のデコード回数が原則1回(予算超過fallbackは計測で明示)
  - 137MB級×複数入力でRSS予算を守る
  - backend切り替え、cache miss、abort後再開が正しい
- **選択肢**:

  | Option | Change | Pros | Cons | Generality | 採用 |
  | --- | --- | --- | --- | --- | --- |
  | A stageローカルcache | 各stageで同じ入力のdecode結果を持つ | 境界変更が少なく安全 | stage間の再デコードは残る | 高 | — |
  | B job-scoped bounded DecodeHandle | 入力IDと正規化条件をkeyとするLRU/cacheで共用 | 再デコードを最大限避け、backend非依存、上限明確 | 寿命管理・失効設計が必要 | 高 | ○ |
  | C mmapを正本表現 | diskのPCM成果物をmmapしzero-copy viewで参照 | 大型ジョブでRSS削減可能 | 既存Vec APIと寿命・alignment/endianness適合が難しい | 中 | — |

- **リスク**: cache keyでsample rate/ch/layoutを取り違えると解析値破損 / 未圧縮PCM常駐によるRAM浪費 / mmapのプラットフォーム差・ファイル寿命
- **影響**: al-media, al-job, al-analysis, Cache/RSS, Security
- **対象ファイル**: `crates/al-media/`, `crates/al-job/`, `crates/al-analysis/src/stage.rs`
- **優先度の理由**: 局所的な改善で大きな冗長IOを除去し、後続並列化のメモリ設計にも効く。

#### #36 主要コーデックの実デコード対応 — P1 / XL / 一部実装 / Media

- **概要**: MP4/AAC/H.264等を直接入力可能にする
- **現状**: PR #44でMediaDecoder registryとnative backend統合が実装済み。WAV/AVI PCMとMP4メタデータ・PCM demuxが基盤。
- **ギャップ**: AAC/H.264/HEVC/Opus/AV1等の実デコーダ、主要コンテナの幅広い対応、バックエンド選択/運用契約が未完成。
- **提案決定**: 依存追加の承認を受け、最初の実コーデック対象を決定する。推奨は音声純Rust優先+映像ffmpeg subprocess fallback。
- **確定決定(2026-10-14)**: 音声は **symphonia(純 Rust)を依存追加して registry backend として採用**(依存追加は承認済み)。native 優先 resolve を維持し、第1対象は AAC-LC in MP4。部分結果統合: symphonia が音声を復号しても native が取得した映像・timing 情報を保持する。正確性検証は PCM値・チャンネル・sample_rate・AAC priming/delay・PTS・複数トラック選択を対象とし、provenance/attempts/unavailable 契約を維持する。映像は既存 AVI 非圧縮デコードを維持し、MP4/H.264 の VideoThumbs・VideoFeatureArtifact は Unavailable 継続で理由を明示する。H.264/H.265 実デコードは次フェーズの optional ffmpeg subprocess backend(外部バイナリ不在でも音声解析は実行可能)。
- **手順**:
  1. 対応マトリクスを must / should / later と decode/probe別に定義。AAC/H.264入りMP4を最優先。
  2. decoder backendをCapabilities→Probe→Decodeに分け、エラーをunavailable/malformed/fallbackに分類。
  3. 時間軸: DTSとPTS、edit list、start time、drift、複数trackの契約を固定。
  4. 純Rust音声と外部ffmpeg映像を分離し、バージョン・選択理由・fallbackをmanifestに記録。
  5. コンテナ×codec×OSのE2E行列で検証し、未対応を明示レコードとして保持。
- **テスト**:
  - MP4 H.264+AACの音声と動画サムネイルを直接生成
  - PTS/DTS、VFR、B-frame、音声遅延、複数trackが正常
  - デコーダ不在はunavailable、破損ファイルはfatal区別
  - 外部プロセスの引数固定・資源制限・stderr/exit codeとprovenanceが残る
- **選択肢**:

  | Option | Change | Pros | Cons | Generality | 採用 |
  | --- | --- | --- | --- | --- | --- |
  | A 純Rustデコーダのみ | symphonia等で対応する音声/コンテナを増やす | 配布が単純でFFI・実行バイナリ不要 | 動画コーデック網羅は難しい。対応速度に限界 | 中 | — |
  | B 音声純Rust + ffmpeg subprocess | 検出/probeを共通化し、音声は純Rust優先、動画等はコマンド実行fallback | フォーマット網羅と配布・保守の均衡。出力契約を統一可能 | 外部バイナリ・バージョン・資源制限の管理が必要 | 高 | ○ |
  | C ffmpeg FFIを中心に一本化 | libavcodec/libavformat直接呼出しのbackendを主系に | 高性能で多codecに広く対応 | ABI・クロスプラットフォーム配布・ライセンス保守が重い | 高 | — |

- **リスク**: 外部ffmpegの配布/ライセンス/脆弱性更新 / CLI子プロセスの権限と非信頼入力の安全性 / ハードウェアデコード差異による結果非決定性
- **影響**: al-media, Ingest, CLI, A/V timing, Dataset provenance, Security
- **対象ファイル**: `crates/al-media/`, `crates/al-cli/`, `crates/al-job/`, `crates/al-dataset/`
- **証拠**: [PR #44：registry/native実装、実codecは残件](https://github.com/scarecrow173/audio-lens/pull/44), [Issue記載のADR参照](https://github.com/scarecrow173/audio-lens/issues/36)
- **優先度の理由**: 一般的なゲームキャプチャを直接受け付けるための実用上の主要ギャップ。
- **備考**: 「すべて対応」はライセンス/DRM/実装可能性の制約を受けるため、probe可否・decode可否の対応マトリクスとして契約化する。

#### #42 FFTカーネルの性能目標・再評価 — P2 / L / 一部実装 / DSP

- **概要**: 既存pulp SIMD実装の残余改善
- **現状**: PR #44でal-math/pulp SIMD・FFT/NCC導入、PR #45でsplit re/im公開境界へ移行。PR #45の実使用経路は概ね改善、直接plan経路で約10%の劣後事例。
- **ギャップ**: Issueが元来掲げたFFT 3×以上などの目標はPR #44時点で一部未達。rustfft置換案は現在のアーキテクチャと再比較が必要。
- **提案決定**: 既存split APIを維持したまま実負荷プロファイルを更新。交換可能なkernel adapterを内部に限定し、採用はベンチと等価性で決める。
- **確定決定(2026-10-14)**: **rustfft 導入は棚上げ**。issue の前提(手書き radix-2 複素FFT・非SIMD)は PR #44/#45 の pulp SIMD 実数FFT で既に解消済み。#52 完了後に実使用プロファイルで再ベースラインし、kernel 本体が律速なら rustfft prototype と等価性・速度を比較する。依存追加はその時点で再承認とする。計測には pow2 非適合サイズを含める(現行は pow2 pad 限定で、混基数の利点が残り得る)。e2e 改善が小さければ close with rationale。3x 目標は microbench 単発ではなく実使用経路の改善に再定義する。
- **手順**:
  1. #44/#45/#49/#50/#51マージ後の同一機材・同じデータでbaselineを更新。
  2. kernelをreal-FFT、bandpass、NCC、呼出し時plan構築に分解してボトルネック判定。
  3. #52のplan再利用とkernel差し替えを混同せず、まず両者を独立計測。
  4. AoS hybrid workspace vs rustfft/realfft backend vs現行pulpのプロトタイプで比較。
  5. 目標3×の適用範囲と失敗時のclose条件を再定義。
- **テスト**:
  - golden・scalar参照・境界サイズ・異ISAを許容誤差内で維持
  - 実使用e2e/各kernelで改善、退行した経路は理由を記録
  - 外部ライブラリ追加はADR承認後のみ、pulp公開漏出なし
- **選択肢**:

  | Option | Change | Pros | Cons | Generality | 採用 |
  | --- | --- | --- | --- | --- | --- |
  | A 現pulp内部のhybrid AoS/SoA最適化 | ワークスペースの局所layoutとFFTサイズ特化を内部で調整 | 依存追加不要、公開API維持 | サイズ別チューニング/保守負担が大きい | 中 | — |
  | B rustfft/realfftを内部backend追加 | FFT実装をライブラリに差し替える実験 | 混合基数・実数FFTの成熟実装を利用 | 外部依存・版・変換copyが増え、必ずしも速いとは限らない | 高 | — |
  | C カーネル現状維持し#52に集中 | plan/cache最適化と呼出し局所化のみを進める | 最小変更でe2eの効果を拾いやすい | 単体FFT目標が未達のまま | 高 | ○ |

- **リスク**: 旧NumPy比較は現HEAD性能を表さない / 交換によるFFTサイズ/paddingの変更と丸め差 / 直接planとfacade経路で性能の方向が異なる
- **影響**: al-math, al-locate, al-analysis, API/ABI, Numerical, 所有権
- **対象ファイル**: `crates/al-math/src/fft.rs`, `crates/al-math/src/correlation.rs`, `crates/al-math/src/filter.rs`, `crates/al-locate/src/dsp.rs`
- **証拠**: [PR #44：SIMD基盤実装・目標一部未達](https://github.com/scarecrow173/audio-lens/pull/44), [PR #45：公開境界変更と性能結果](https://github.com/scarecrow173/audio-lens/pull/45), [pulp採用ADR](https://github.com/scarecrow173/audio-lens/blob/main/docs/adr/simd-arithmetic-pulp.md)
- **優先度の理由**: 重要だが基盤は実装済み。旧性能比較をそのまま現課題の優先順位に用いない。

#### #55 al-datasetのfsync頻度を再設計 — P2 / M / 未着手 / Dataset

- **概要**: 成果物種別の固定耐久性等級で fsync を削減(モード切替なし)
- **現状**: Issueでは4MiBごとのsync_allが1run約1〜2千回発生しWindowsで秒単位コストと報告。
- **ギャップ**: 頻度低減すると電源断後に失われるchunk範囲・manifestとの整合が変わる。ユーザー承認が必要。
- **提案決定**: 成果物種別に耐久性等級を定義し、書込完了はtemp→sync(file)→rename→必要に応じparent dir fsyncで保証。
- **確定決定(2026-10-14)**: **無条件の耐久性等級(モード切替なし)**。manifest + 最終 export は full durability(temp→sync→rename→dir sync)を維持し、中間 artifact(`decoded.f64le`・normalized PCM・tensor chunk 等の再生成可能物)は緩和する。順序規約: manifest が artifact 完了を記録する前にその artifact は耐久化済みとし、resume は「manifest 記録 = 存在かつ完全」を前提にできる。理由: 正本はユーザー提供入力ファイルであり、中間物喪失は再実行で済みデータ損失ではない。補足: 現状の fsync 大量発生の本体は artifact 毎の `write_durable`(issue の「4MiB毎」記述は概算/旧実装)。
- **手順**:
  1. chunk同期/ファイル同期/commitの関係とOS別fsync挙動を整理し、旧保証を書き下す。
  2. scratch派生物と正式manifestで durability policy を区別するADR。
  3. 一時ファイルを閉じてsync→rename→manifest更新の状態遷移を実装。
  4. 中断/電源断を模した故障注入とWindowsベンチで比較。
- **テスト**:
  - 再開時に未コミットpartが見えない
  - 正常完了時のartifact+manifestが整合
  - sync回数とe2e短縮効果を測定
  - 固定等級と manifest 記録前の耐久化順序が説明・検証可能
- **選択肢**:

  | Option | Change | Pros | Cons | Generality | 採用 |
  | --- | --- | --- | --- | --- | --- |
  | A sync_allを最後1回だけにする | 全chunk同期を最終file同期へ集約 | 変更量少・fsync回数最少 | 中途障害でchunkを失い、既存保証を暗黙に変更 | 中 | — |
  | B artifact単位atomic commit + durability policy | tmp write→file sync→rename→manifest順序を管理 | 再開境界と成果物種別の固定耐久性を明文化 | OS別検証・writer API変更が必要 | 高 | ○ |
  | C 現行chunk fsyncを維持 | 既存同期契約のまま | クラッシュ時粒度を維持 | 大きなfsyncコストが残る | 高 | — |

- **リスク**: renameは必ずしも電源断耐久性を意味しない / filesystem差異を無視した簡略化 / 既存のchunk単位耐久保証を黙って弱めること
- **影響**: al-dataset, Manifest/Resume, Export, Windows IO, クラッシュ耐性
- **対象ファイル**: `crates/al-dataset/src/`, `crates/al-job/`, `crates/al-cli/`
- **優先度の理由**: 性能以前にデータ損失保証を決定する必要があるため、設計ADRを先行。

#### #33 spec #1への残差適合 — P3 / L / 一部実装 / Contract

- **概要**: 候補stage・FLAC・Resolution.sample_rate
- **現状**: 候補はExpensiveAnalysisStage内部で抽出済み、WAV保存とparams経由のsample_rate保持も実装。PR #44でstage構造とsample_rate適合の一部が処理されたと記述。
- **ギャップ**: 候補のみ再計算できない、FLAC encodeなし、Resolutionにsample_rateが第一級でない(現行Issue本文記載)。
- **提案決定**: 3論点を一括実装しない。まず仕様とのズレが機能要件か記述だけかを選び、sample_rate型追加は低リスクだがスキーマの移行方針を定義。
- **確定決定(2026-10-14)**: ①候補検出は独立 StageKind を追加せず `ExpensiveAnalysisStage` 内 fused を維持し、spec 文言を実装に合わせて改訂する。候補データは `events/candidates` の独立成果物契約を維持(空テーブルでも保存し「候補なし」と「未実行」を区別)。manifest/resume 境界は不変。将来単独実行需要が出たら再検討。②FLAC は実装せず WAV のみを正式仕様にする(spec 改訂)。FLAC エンコーダ依存は導入しない。将来は別 Issue(浮動小数点 PCM の量子化精度損失に注意)。#36 の FLAC デコード(入力)とエンコード(出力)は独立。③`Resolution.sample_rate` は実装済み(`types.rs:67`、5箇所の構築サイト、commit 3386fab)のため確認して close のみ。残作業は **spec 文言改訂のみ(実装不要の docs タスク)**。
- **手順**:
  1. spec の候補検出を fused 実行として改訂し、候補抽出→トリガ評価→Expensive Analysis の順序を明記する。
  2. `events/candidates` の独立参照・空テーブル保存契約と、現行 manifest/resume 境界を明記する。
  3. 残差・中間音声の出力仕様を WAV のみに改訂し、FLAC 出力は将来の別 Issue とする。
  4. 実装済み `Resolution.sample_rate` と構築サイトの整合を確認し、close 可否の根拠を記録する。
- **テスト**:
  - 候補-only再計算の要否と契約がドキュメントで決まる
  - Resolution sample_rateとrecord params間の整合
  - WAV 出力仕様と既存 provenance 契約の整合を確認(FLAC encode は対象外)
  - 仕様改訂を選んでも未実装理由が明記される
- **選択肢**:

  | Option | Change | Pros | Cons | Generality | 採用 |
  | --- | --- | --- | --- | --- | --- |
  | A 現状追認(仕様を更新) | fused stage維持、WAV維持、実装済みResolution.sample_rateを確認 | 変更最小で機能欠落は実質少ない | Stage単独再実行不可、FLAC省容量化なし | 中 | ○ |
  | B 独立タスクごと段階適合 | candidate関数分離、rate第一級化、FLACは容量根拠に従う | 必要な汎用性だけ確保し無駄な抽象化を避ける | spec全文に対して即時完全適合にはならない | 高 | — |
  | C 仕様厳密実装 | 独立StageKind+FLAC+schema刷新を同時に実施 | 仕様と実装が全点一致 | 新しい変更面が広く性能改善から脱線 | 高 | — |

- **リスク**: 独立stageでcheckpoint/順序/manifestが増える / FLAC encode依存とlossless保証 / sample_rate二重表現の不整合
- **影響**: al-analysis, al-job, al-dataset schema, Codec, al-viz
- **対象ファイル**: `crates/al-analysis/src/stage.rs`, `crates/al-dataset/src/audio.rs`, `crates/al-dataset/src/types.rs`, `crates/al-viz/`
- **証拠**: [PR #44：一部適合とFLAC defer](https://github.com/scarecrow173/audio-lens/pull/44), [Issue #1：正本仕様](https://github.com/scarecrow173/audio-lens/issues/1)
- **優先度の理由**: 3つの独立問題を無理にまとめない方が、実装コスト/一般性を制御しやすい。

### Phase 2

#### #53 local_lagの全探索を高速化 — P1 / L / 未着手 / Reference

- **概要**: 約18G MACs/pairのlag走査を再設計
- **現状**: reference.rsのlocal_lagは±512 lagについて窓ごとにcorr0を全走査する。Issue見積もりは約18G MACs/pair。
- **ギャップ**: FFT-NCCへの単純置換ではper-lag energyとoverlap/boundary semanticsが変わり、結果の意味が変化する可能性。
- **提案決定**: まず現行式を数学的に固定し、FFTで分子を計算しても分母は同一範囲の累積二乗和で再現する。
- **手順**:
  1. 現行スカラー実装をreference oracleとして保存し、同長/短長/端点/ゼロ窓をテスト化。
  2. 分子をFFT相関、分母をlagごとのprefix energyで構成するprototypeと、窓サイズ別SIMD直接法を比較。
  3. local_lagのAPIを「候補lag、正規化、重なり範囲、Validity」を明示した形にする。
  4. #52 plan/workspace再利用後のsteady-stateを計測し、goldenと誤差・ピーク位置の非回帰を判定。
- **テスト**:
  - peak lag・gain・Validityの差異を許容範囲で説明できる
  - zero-energy / equal-length / window-boundary / ±512 edgeの全てを通過
  - 現行窓単位のCPU時間とe2e時間を測定し回帰なし
- **選択肢**:

  | Option | Change | Pros | Cons | Generality | 採用 |
  | --- | --- | --- | --- | --- | --- |
  | A 既存nccへ置換 | 窓ごとにFFT-NCCを直接呼ぶ | コード少・期待速度大 | 正規化/境界が違い意味互換なし。採用前に仕様変更の承認が必要 | 中 | — |
  | B FFT分子 + 同一分母のexact semantics | 畳込みと累積エネルギーで現行corr0数式を再現 | 既存の観測意味を維持しつつ大幅加速が可能 | 実装/検証複雑。FFT誤差は許容比較が必要 | 高 | ○ |
  | C lag幅限定SIMD/ブロック計算 | ±512のまま連続データに局所最適化 | 数値意味と実装形が近く短い窓で有利 | 演算量自体は大きく残る | 中 | — |

- **リスク**: 正規化差は単なるulp差でなく選択lagを変え得る / FFTの短窓ではオーバーヘッドが勝ち得る / #59の曖昧ピーク対策がないとピーク一致だけの評価は危険
- **影響**: al-analysis, al-math correlation, lag & drift, golden
- **対象ファイル**: `crates/al-analysis/src/reference.rs`, `crates/al-math/src/correlation.rs`, `crates/al-testkit/`
- **証拠**: [#53 Issue記載の負荷見積](https://github.com/scarecrow173/audio-lens/issues/53)
- **優先度の理由**: 大規模な改善余地があるが、数値意味の検証を先に行う。

#### #40 coarse-to-fineと帯域cacheの残件 — P2 / L / 一部実装 / Locate

- **概要**: pair間フィルタ共有と精度維持
- **現状**: PR #44で間引きcoarse-to-fineと入力単位bandpass cacheが導入済み。
- **ギャップ**: PR #44によりpair間の再利用は後続対応と明記。bandpass方式のIIR化等も未評価。
- **提案決定**: 実装済み範囲を再監査し、pair間の共通フィルタ結果だけをbounded shared cache化。帯域特性が変わるIIR移行は別判断。
- **手順**:
  1. 実際にinput-local/cache同一性とペアループでのfilter呼出回数を測定。
  2. cache keyをinput-id/sample-rate/ch/band/algorithm params/normalizationで構成。
  3. 同じfilter出力を複数pairで安全に参照、上限・eviction・並列化連携を設計。
  4. 既存spectral maskとIIR/複数帯域IFFTを別ベンチで比較、周波数応答・位相の差を評価。
- **テスト**:
  - 長尺・loop・skip/resume golden維持
  - キャッシュhit率・miss率・RSS・e2e時間を計測
  - 異なる帯域/サンプルレート/チャンネル間でcross-contaminationなし
- **選択肢**:

  | Option | Change | Pros | Cons | Generality | 採用 |
  | --- | --- | --- | --- | --- | --- |
  | A pairをまたぐbounded cache | 既存spectral mask出力をinput+band keyで共用 | 数学結果不変、実装変更が局所的 | 大きいcapture×band数分のメモリ | 高 | ○ |
  | B IIR/SOS帯域抽出へ移行 | spectral mask FFT→IIR系へ変更 | 計算量とストリーミング適性改善余地 | 位相/過渡/零位相応答が変わるため互換性コスト大 | 中 | — |
  | C 1 forward FFT→bandごとの逆FFT | bandpass_manyや内部の共有スペクトルを一層徹底 | 既存周波数応答を維持しやすい | 長尺保持とinverse本数が必要 | 高 | — |

- **リスク**: filterアルゴリズム差異がlag/gainに影響 / cacheがpair数に比例して増大する実装 / coarse探索で周期の誤位相を採択し得る
- **影響**: al-locate, al-math filter, メモリ/Cache, coarse探索, golden
- **対象ファイル**: `crates/al-locate/src/stage.rs`, `crates/al-locate/src/methods.rs`, `crates/al-math/src/filter.rs`
- **証拠**: [PR #44：一部実装とpair間共有の残件](https://github.com/scarecrow173/audio-lens/pull/44), [PR #51：bandpass_many呼出改善(別経路)](https://github.com/scarecrow173/audio-lens/pull/51)
- **優先度の理由**: 部分完了済み。実測で重複が残ることを確認したあと拡張する。

#### #57 envelopeを線形時間化 — P2 / M / 未着手 / Cheap

- **概要**: 精度を守りながら窓積算をO(n)にする
- **現状**: cheap.rs envelopeは直接和O(len×width)。単純prefix sumでは大きい累積値の差分で桁落ち懸念。
- **ギャップ**: 浮動小数の長尺キャンセレーション、許容誤差、短い窓での overhead の定量化が必要。
- **提案決定**: 補償付きsliding sumを中心に、定期的な直接再計算で累積誤差の上限を制御。ブロックprefixも比較。
- **手順**:
  1. 窓幅別・信号分布別(DC、小ノイズ、極端dynamics、長尺)で現行値を保存。
  2. naive prefix、block-prefix、compensated sliding、direct baselineを比較。
  3. 誤差の絶対/相対許容基準をvalidityへの影響とともに決める。
  4. 速度改善が有意なlen/widthから選択し、短尺は直接和へfallback。
- **テスト**:
  - O(n)アルゴリズムの長尺ベンチ
  - 結果誤差分布と最大誤差が記録される
  - NaN/Inf/ゼロ長/窓境界の現契約維持
  - featureレコードのValidity不意変更なし
- **選択肢**:

  | Option | Change | Pros | Cons | Generality | 採用 |
  | --- | --- | --- | --- | --- | --- |
  | A 単純prefix sum | 累積積算から各window差分を取る | 最短実装で計算量O(n) | 長尺で差のキャンセレーション、精度低下 | 中 | — |
  | B 補償付きsliding sum + 定期rebase | 窓の入出力を補償加算し定期的に窓を直接再評価 | O(n)かつ長尺ドリフトを制御しやすい | rebase周期と初期化誤差の検証が必要 | 高 | ○ |
  | C ブロックprefix + 高精度局所和 | 一定block内のみprefixを作り、境界を個別加算 | 長さ依存のキャンセレーションを抑えられる | 窓幅跨ぎの処理が複雑 | 高 | — |

- **リスク**: 窓幅固定でも長尺累積ドリフト / bit一致を無理に要求すると改善できない / 局所数値差がcandidate threshold近傍へ伝播
- **影響**: al-analysis cheap, 数値再現, 長尺入力, Golden
- **対象ファイル**: `crates/al-analysis/src/cheap.rs`, `crates/al-testkit/`
- **優先度の理由**: 数値契約の慎重さが必要だが、他のIssueから独立して着手可能。

#### #58 matrix_to_tensorコピーを除去 — P3 / M / 未着手 / Dataset

- **概要**: フレーム行列の再flattenを抑止
- **現状**: matrix_to_tensorでフレーム行列からtensor用に再flatten copy。PR #51では永続形式に波及するため除外。
- **ギャップ**: row-majorのin-memory viewとchunk writerの契約、永続フォーマットを変えるか未決定。
- **提案決定**: 永続フォーマットを維持してborrowed row-major adapter/iterator経由でchunk書込。型/寿命が障害なら現行copyを維持。
- **手順**:
  1. matrix_to_tensorのcopy/alloc bytesとwriter側連続chunk変換を計測。
  2. row-viewとchunk encoderを分離し、永続フォーマットはbyte-identical優先。
  3. borrowed dataをchunk単位で書く経路を追加。
  4. 参照ビュー不可能なら、フォーマット変更のコストを別ADRで評価。
- **テスト**:
  - 既存tensorのshape/endianness/chunk順/byte列が変化しない
  - peak RSS・alloc bytes・e2e時間を計測
  - al-viz/reader/Exportの互換性検証
- **選択肢**:

  | Option | Change | Pros | Cons | Generality | 採用 |
  | --- | --- | --- | --- | --- | --- |
  | A borrowed row-major streaming encoder | frame毎sliceを受けchunkingし永続形式不変 | 汎用reader互換、コピー削減、変更局所 | 現行writer APIを一般化する必要 | 高 | ○ |
  | B row-major in-memory型へ統一 | STFT/feature生成時点から単一flat buffer | 連続化とSIMD最適化に有利 | 解析側API広範囲変更・コピーの移設の可能性 | 中 | — |
  | C 永続schemaを変更 | tensor storeに新しいframe-row layoutを採用 | 将来的なzero-copy可能性 | 互換/マイグレーション/全consumer変更が重い | 中 | — |

- **リスク**: in-memory Vec<Vec>の非連続配置によるcopy不可避箇所 / 永続フォーマット変更に伴う全consumer追従 / 借用データ寿命と非同期writerの相性
- **影響**: al-analysis, al-dataset tensor, al-viz reader, Artifact schema
- **対象ファイル**: `crates/al-analysis/src/cheap.rs`, `crates/al-analysis/src/stft.rs`, `crates/al-dataset/src/tensor.rs`, `crates/al-viz/`
- **優先度の理由**: 現時点で効果測定が乏しく、フォーマット変更の波及が大きい。

### Phase 3

#### #54 cheap/reference stage内部の並列化 — P1 / XL / 未着手 / Runner

- **概要**: 入力・ペア単位を並列計算し確定順で保存
- **現状**: Issueによればスレッドプールは主としてlocateにあり、cheap/referenceは入力・ペア・ch単位で逐次実行。
- **ギャップ**: 並列実行してもresume、manifest順序、プロセス完了判定、duration_msの契約を保つ必要。
- **提案決定**: 計算とコミットを分離し、workerはimmutable inputから計算、coordinatorがdeterministic順序でcommit。
- **手順**:
  1. stage内部のwork unit(input/pair/ch)をID付きで切り出し、artifact書込をworkerから分離。
  2. bounded queueとmemory tokensを導入し、decode/FFT workspace/tensorの同時保持量を制限。
  3. coordinatorがmanifest順にcommitし、失敗時はartifactの原子的な可視性とresume境界を保証。
  4. tracingでwall/cpu/worker時間を区別し、serial modeを検証・fallback用に保持。
  5. Windows/低コア/大量入力/キャンセル/クラッシュリカバリの性能・一意性試験。
- **テスト**:
  - 1/2/4/8 workerでexportの論理値が等価、順序が安定
  - キャンセル・途中失敗・再開で重複/欠損artifactなし
  - ピークRSS・FD数が事前定義の上限以下
  - e2e speedupをCPU/IO別で測定、duration_msの意味が一致
- **選択肢**:

  | Option | Change | Pros | Cons | Generality | 採用 |
  | --- | --- | --- | --- | --- | --- |
  | A pair単位にparallel iterator | 既存stage内部に並列iteratorを導入 | 最小コードでCPU利用改善 | 書込と処理順序が絡み決定性・resumeで破綻しやすい | 低 | — |
  | B compute worker + ordered commit | work unitごと純計算を行い、単一committerが順序制御 | 追跡/再開/決定性が明快で負荷制御しやすい | 結果の一時保持/スケジューラ追加が必要 | 高 | ○ |
  | C 全stage非同期DAG executor | runner自体をtask-DAGワークフローへ変更 | 長期拡張性は最大 | 現段階には広すぎてテスト表面と複雑さが急増 | 高 | — |

- **リスク**: 同時FFT scratch・PCM cacheでOOM / workerからmanifest更新すると順序とresumeが破れる / 同一結果でも浮動小数reduction順序を跨ぐと差異
- **影響**: al-job, al-analysis, al-dataset, al-observability, Memory
- **対象ファイル**: `crates/al-job/`, `crates/al-analysis/src/stage.rs`, `crates/al-dataset/`, `crates/al-observability/`
- **証拠**: [PR #48：stage timing observability](https://github.com/scarecrow173/audio-lens/pull/48)
- **優先度の理由**: 最終的なe2e利得が大きい候補だが、共有cache/保存の契約を先に詰める。

### Phase 4

#### #60 残余小項目の性能棚卸し — P3 / S / 未着手 / Polish

- **概要**: Residual gather・LOD・コピーの端数改善
- **現状**: PR #49でresidualのper-source sub_scaled化済、PR #51でdeinterleave block化済。
- **ギャップ**: residual列gatherはslice API非適合。al-viz LODはincremental streaming契約で通常のvec primitivesと合わず、残余interleave/copyが点在。
- **提案決定**: まずプロファイルで費用対効果を検証し、既存API契約を崩さない局所変更だけを採用する。
- **手順**:
  1. 残存loopをcall count/striding/alloc/byte量で再棚卸し。
  2. gatherは可搬SIMDを増設する前にtranspose/ブロック化の効果を計測。
  3. LODはstreaming chunk境界・累積誤差・部分読み出しの契約を固定。
  4. 採用するものだけ細分化してベンチを先行、改善不明はclose with rationale。
- **テスト**:
  - LOD chunk境界のbit/数値同一性
  - resident/RSS増加なし・perf有意差がある場合のみmerge
  - 既存Vec API変更なし、no-regression benchmark
- **選択肢**:

  | Option | Change | Pros | Cons | Generality | 採用 |
  | --- | --- | --- | --- | --- | --- |
  | A 局所loop改善 | コピー削減・ブロック化だけ実施 | リスク少・汎用性維持 | 改善幅は小さい可能性 | 高 | ○ |
  | B strided/gather vec API拡張 | gather/scatter等をal-mathへ追加 | 一般的なstrided演算を再利用可能 | API面積・SIMD差異・保守コスト増 | 中 | — |
  | C 大規模メモリlayout再編 | residual/LODを連続配列中心に再設計 | 長期的な効率の上限を上げる | 局所課題に対して変更量過大 | 中 | — |

- **リスク**: 汎用vec APIを過度に専用化 / gather SIMDはメモリアクセス律速で逆効果 / 小項目が大規模設計変更へ膨張
- **影響**: al-locate residual, al-viz LOD, al-media, al-math vec
- **対象ファイル**: `crates/al-locate/src/residual.rs`, `crates/al-viz/src/lod.rs`, `crates/al-locate/src/dsp.rs`, `crates/al-math/src/vec.rs`
- **証拠**: [PR #49：residual最適化と残余許容](https://github.com/scarecrow173/audio-lens/pull/49), [PR #51：deinterleave改善](https://github.com/scarecrow173/audio-lens/pull/51), [vecプリミティブ仕様](https://github.com/scarecrow173/audio-lens/blob/main/docs/specs/al-math-vec-primitives.md)
- **優先度の理由**: 手戻りの少ない最後の仕上げ。測定で改善が小さければ不要。

## Alternatives and Comparison

ロードマップ全体の構成化についての比較:

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| Phase 0-4 の段階的実行(本レポート案) | 正しさの契約(#59)を先に固定し、基盤(#52/#56)→局所最適化→並列化の順で手戻りを抑える | 単線化しすぎると並行着手可能な独立 issue(#57 等)が遅れる | yes |
| Prio 順のみで実行 | P1 を最優先で潰せる | #59 の正しさ契約が後回しになり、先行最適化が誤受理を固定化するリスク | no |
| Domain 別に並列チェーン化 | 各ドメインの専門性で進められる | #54 は複数 issue の契約確定が前提で、ドメイン分断では前提管理が難しい | no |

## Tentative Conclusions and Hypotheses

- confirmed(前回分析からの主要な再ランク付け): **#42 は P2 へ降格**。
  PR #44/#45/#49/#50/#51 のマージでボトルネックプロファイルが変わった
  ため、kernel 差し替えの判断前に同一機材・同一データで baseline を
  更新する必要がある(旧 NumPy 比較は現 HEAD 性能を表さない)。
- confirmed(混同禁止): **#52 の plan 再利用と #42 の kernel 差し替えは
  別の問題**であり、まず両者を独立計測する。plan 再利用による e2e 改善が
  kernel 交換の必要性評価を左右するため、#52 を先に固める順序が提案される
  (#52→#42 は Phase 1 で確定した順序)。
- confirmed(ゲート): #59 は性能系の先行条件 — FFT/相関の数値挙動や
  coarse-to-fine を変更する前に、曖昧位置の判定契約(uncertain の第一級化)
  を固定する。
- confirmed(依存前提): #54(並列化)は #52(worker 別 workspace)、
  #56(共有 PCM 寿命/メモリ予算)が推奨前提、#40(pair 間 cache の
  スレッド安全性)・#55(commit durability と ordered write の境界)が
  任意前提。#33/#57/#58/#60 は他 issue と独立して着手可能。
- hypothesis: フェーズ順に進めれば、Phase 1 の基盤整備だけでも
  冗長 IO(#56)・plan 再構築(#52)・fsync(#55)の削減が見込め、
  Phase 2-3 でさらに e2e 短縮が狙える。効果量は全て実測で確認する
  (Issue/PR 記載の測定条件に限る)。

## Open Questions

- ~~#42: 再 baseline 計測の結果、「FFT 3×」目標の適用範囲と close 条件を
  どう再定義するか~~ → **2026-10-14 確定**: 3x 目標は実使用経路の改善に
  再定義済み。残件は #52 後の再ベースライン実測のみ(改善が小さければ
  close with rationale)
- ~~#36: 依存追加(ffmpeg subprocess 等)の承認と、最初の実コーデック対象の
  決定~~ → **2026-10-14 確定**: symphonia 依存追加は承認済み、第1対象は
  AAC-LC in MP4。映像 H.264/H.265 は次フェーズの optional ffmpeg
  subprocess backend
- ~~#55: 耐久性等級のユーザー承認(fast/safe モードの保証説明)~~ →
  **2026-10-14 確定**: モード切替なしの無条件等級(manifest+最終exportは
  full durability、中間 artifact は緩和)
- #59: 同率許容幅・曖昧性閾値の具体的値 — fixture 計測と spec で確定
- ~~#33: 3 論点それぞれ「機能要件か記述だけか」の切り分け結果~~ →
  **2026-10-14 確定**: ①fused 維持+spec 改訂、②FLAC 非実装で WAV のみ
  正式仕様、③`Resolution.sample_rate` は実装済みで close のみ。
  残作業は spec 文言改訂のみ
- #54: 並列化の前提契約(worker 別 workspace、PCM 寿命、ordered commit)
  が Phase 1 で揃うか → **2026-10-14 部分解消**: worker 別 workspace(#52)・
  PCM 寿命(#56)・durability 規約(#55)の契約は確定済み。実装は未着手

## Promotion Candidates

- [ ] spec-doc needed: #59 の曖昧性判定・合議契約(uncertain 第一級化)、
  #52 の CorrelationPlan/Workspace API、#56 の DecodeHandle/PCM 寿命契約、
  #53 の local_lag 数値契約、#57 の envelope 許容誤差、#54 の並列実行・
  ordered commit 契約 — 各 issue 着手時に個別 spec として起こす
- [ ] adr-doc needed: #55 の durability policy(scratch 派生物 vs 正式
  manifest の等級)、#42 の kernel adapter/外部ライブラリ追加判断、
  #33 の spec 適合方針(現状追認 vs 段階適合)、#36 の codec backend
  選択・外部プロセス方針、#58 の永続フォーマット変更判断(必要時)
- [ ] 本ロードマップ自体は issue 着手のたびに `updated` を更新し、
  全 issue が close/昇華されたら status を `resolved` へ移行する

## Sources and Relations

- 一次成果物: `target/audio-lens-issue-atlas.html`(untracked。可視化
  コンパニオン。原本: `C:\Users\akiyoshi\Downloads\
  audio_lens_issue_dashboard.html`)。データは同ファイル内の
  `<script id="dataset" type="application/json">` を抽出して転記
- 調査対象: GitHub open issue 13 件の本文・全コメント、PR #44/#45/#49/
  #50/#51 のマージ状況(methodology 参照、snapshot 2026-10-13 JST)
- 関連探索(related): `caller-side-optimizations.md`(A1/A3/A4 適用済み・
  A2 defer の発端。issue #52 の根拠)、`mechanical-caller-cleanup.md`
  (A6-A16 バンドル。#52/#58 等の除外根拠)、`al-math-simd-reduction.md`
  (al-math kernel・SIMD reduction 順序。issue #42 の文脈)
- 既存 ADR/spec との接続: pulp 採用 ADR(`docs/adr/simd-arithmetic-pulp.md`)、
  stage timing(`docs/adr/stage-timing-manifest-persistence.md`)、
  vec プリミティブ spec(`docs/specs/al-math-vec-primitives.md`)— issue
  別セクションの証拠リンク参照
