---
id: "TASK-035gGKtgDKLiTaF31a5GUM"
type: "task"
status: "done"
title: "locate: 帯域キャッシュ + coarse-to-fine 間引き探索"
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
  implemented-by:
    - "crates/al-locate/src/prepared.rs"
    - "crates/al-locate/src/methods.rs"
    - "crates/al-locate/src/stage.rs"
  depends-on:
    - "docs/tasks/al-math-correlation.md"
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
    - "https://github.com/scarecrow173/audio-lens/issues/40"
  defers: []
  deferred-by: []
---

# locate: 帯域キャッシュ + coarse-to-fine 間引き探索

## Work

`BandNcc` がペアごとにキャプチャ全区間に bandpass を掛けている
重複を、入力単位のキャッシュに移す。あわせて長尺ソースの探索を
間引き信号での coarse→fine に分割する。

- bandpass 結果を入力(capture)単位でキャッシュし、
  メソッド間・ペア間で共有(ステージループ外側に保持)
- locate 入口で入力ごとに間引き版信号(1/4 相当)を生成し、
  粗探索は間引き信号、精密化は原レートで行う
- 間引きは #37 のリサンプル正規化インフラと共用できる構造に
- 長尺ソース(数十秒)で効果を確認

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/simd-dsp-foundation.md`

## Dependencies

- `docs/tasks/al-math-correlation.md`

## Affected Files

- `crates/al-locate/src/stage.rs`(キャッシュ導入・間引き経路)
- `crates/al-locate/src/methods.rs`(coarse-to-fine)
- `crates/al-math/src/`(間引き用リサンプラ借用)

## Done When

- [x] BandNcc が入力単位のフィルタ済み信号を再利用
- [x] 長尺ソースで粗探索が間引き信号で動作
- [x] 既存 golden が全てパス

## Verification

- [x] `mise exec -- cargo test -p al-locate`

## 実装

- 入力準備層 `InputCache` が入力 ID、チャンネル、正規化レート、信号版、
  全区間 `[0, frames)`、FFT 長、帯域の半開境界、フィルタ方式をキーに保持する。
  従来と同じ次の 2 冪への末尾ゼロ埋め、Hz による bin 選択、元長への切り戻しを
  固定し、`bandpass_many` の共有 forward FFT を利用する。並列メソッドには
  `Arc` の不変データを渡す。素材にエネルギーがない帯域の NCC は実行しない。
- 素材・キャプチャの全区間フィルタを再利用する。アンカー窓は従来どおり
  全帯域 NCC で処理し、窓フィルタと全入力フィルタの応答を置き換えない。
  この経路では未キャッシュ波形・証拠と完全一致を要求する (許容差ゼロ)。
- 正規化後の各入力から `ResamplePlan` で `rate / 4` レートの間引き信号を生成する。
  4 の倍数でないレートは有理比で座標を変換する。2 秒以上の素材で NCC の
  粗候補を最大 8 件提案し、原レートで左右 `2 * FIR radius + ceil(rate/coarse_rate)`
  の近傍を精密化する。48 kHz では 12 kHz、近傍幅は左右 260 samples。
- F3 に従い、最終証拠は **全メソッドで全レート・元の全探索域**から生成する。
  FullNcc は必須の全探索曲線を近傍精密化でも共用し、FFT の二重計算を避ける。
  候補提案の背景比・競合比・幅を最終証拠や合議へ流用しない。粗候補なし・
  境界候補・精密候補なしの fallback 理由も記録する。
- ペア共通の候補提案を `locate_evidence` の FullNcc 行の `coarse_search` に記録する。
  元/粗レート、元探索域、近傍幅、粗/精密位置、全探索必須フラグ、fallback 理由を
  保存する。残差探索は child の `params.coarse_search` にチャンネル付きで保存する。
- 素材キャッシュは正規化レート別にペア間で共有する。キャプチャは通常・残差探索の
  完了後に解放し、残差は `residual:<parent alignment>` の別版・親ごとのキャッシュで
  全 child が共有する。既知 R1 の child `capture_pcm` 参照は変更しない。

## 検証記録 (2026-10-10)

- `mise install --locked python uv pipx:apm-cli pipx:serena-agent`、`mise run setup` 成功。
  setup が生成した `apm.lock.yaml` の deployment 差分は今回の範囲外として戻した。
- `mise exec -- cargo build -p al-math -p al-locate -p al-analysis` 成功。
- `mise exec -- cargo test -p al-math -p al-locate -p al-analysis` 成功 (176 tests)。
  locate は既存 golden 16 件、unit 33 件、integration 10 件、準備契約 5 件。
- `mise run test` (`cargo test --workspace`) 成功。analysis golden 19 件も維持。
- `mise exec -- cargo clippy -p al-math -p al-locate -p al-analysis --all-targets -- -D warnings`
  成功。
- `mise exec -- cargo fmt -p al-math -p al-locate -p al-analysis -- --check` 成功。
  `bench_dsp.rs` は変更していない。
- 追加検証: ID/チャンネル/レート/信号版/長さのキャッシュ分離、奇数長と端点 impulse の
  フィルタ完全一致、近傍外の強い競合、先頭/末尾ピーク、負極性、周期/広幅信号、
  背景比閾値直近の合議、粗情報不足の fallback、実 Stage の候補提案永続化。

## 性能測定 (2026-10-10)

`mise exec -- cargo run --release -p al-locate --example bench_locate_prepared`。
Windows / Intel Core i9-9900K (8 cores、16 logical processors)、Rust 1.98.1。
48 kHz、35 秒 mono capture (1,680,000 frames)、20 秒 source 2 本 (各 960,000 frames)、
LCG seed 901、同一プロセス内比較、warmup 2 回 + 測定 7 回、p95 はこの 7 回の最大値。
コンパイル・テストと重ねない再測定値を採用した。他プロセスの CPU 使用は制御していない。
各ペアの未キャッシュ/準備済み BandNcc と FullNcc の全証拠は実測前に完全一致を確認した。
精密化には真の挿入位置 48,001 / 480,003 が含まれ、素材の粗信号は各 240,000 frames。

| 対象 (2 ペア合計) | median ms | p95 ms |
| --- | ---: | ---: |
| 2 素材 + capture の cold 準備 | 780.779 | 841.545 |
| 未キャッシュ BandNcc | 3006.539 | 3112.198 |
| 準備済み BandNcc (準備時間を含まない) | 2519.798 | 3036.710 |
| 1/4 レート NCC 候補提案のみ | 100.867 | 102.179 |
| 全探索 FullNcc | 588.826 | 607.258 |
| 粗候補 + 原レート精密化 + 全域証拠 FullNcc | 715.304 | 746.683 |
| cold 準備 + 準備済み BandNcc + 粗精/全域 FullNcc | 4111.996 | 4489.564 |
| 未キャッシュ BandNcc + FullNcc | 3662.870 | 3791.857 |

BandNcc の準備済み再利用は中央値で約 16.2% 短縮。粗提案単体は全探索 NCC の
約 17.1% の時間だが、全探索証拠を含む FullNcc では約 21.5% 増える。
2 ペアの cold 経路全体は約 12.3% 増加しており、初回の高速化は未達である。
この測定対象は NCC/BandNcc と入力準備に限定し、GCC-PHAT、spectral envelope、
アンカー、残差、I/O を含む Stage 全体の高速化の証拠とはしない。

## 残課題 (non-blocking)

- F3 の契約を保つ限り全探索の相関計算は省略できない。粗探索単体の短縮を
  locate 全体の短縮とは扱わず、性能改善の中心はフィルタ済み信号の再利用とする。
- 全帯域の準備と間引きは初回費用・メモリを増やす。キャッシュ済み値だけで
  初回実行の高速化を主張しない。無音帯域も準備するが NCC は従来どおり省く。
- F5 の窓単位フィルタを全入力フィルタへ変える経路の同等性は今回の範囲外。
  今回は従来の区間・grid を維持するので、その未決の許容差を必要としない。
- 独立 reviewer の起動は Orca の `agent-hooks-review-prompt` により `agent_readiness`
  で停止し、レビュー未実施。未実行 Task の terminal は解放し、coordinator に報告した。
- 既知 R1 (残差 child の `capture_pcm` が元キャプチャを参照する) は範囲外のまま。
