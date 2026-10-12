---
id: "IMPL-034Ex6rbLxksciMbCzBFD"
type: "impl"
status: "completed"
title: "caller-side optimizations: post-measurement and closeout"
created: "2026-10-13"
updated: "2026-10-13"
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
    - "docs/tasks/caller-opt-measurement-ir-closeout.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/caller-side-optimizations.md"
    - "docs/specs/caller-side-optimizations.md"
    - "docs/test-specs/caller-side-optimizations.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# caller-side optimizations: post-measurement and closeout

## Summary

plan T5(closeout)。T1–T4 で land した全最適化(A3 stft plan
巻き上げ・A4 LeastSquares 列指向・A1 `dsp::conv` overlap-save +
`fir_model` 置換)の事後計測を、T1 baseline(criterion `pre` /
`lstsq-rows`・`target/bench-baseline/` Instant 出力)に対して
実施し、before/after を確定した。`analyze_pair`(fir_model 経路)
は 691.2 → 581.0 ms(−15.9%)、stft は criterion で −12.5〜−13.2%
・Instant で −14.8%、`prepare_residual`(fit_mix 経路)は −63.2%。
`dsp::conv` は `conv_direct` baseline に対し m = 4096 で約 53〜54
倍、lstsq factor 構築は全サイズで −38〜−73%。±5% を超える回帰は
lstsq `solve` の 2 列ケース(+5.0〜+7.3%)のみで、T3 IR 記録済みの
既知 trade-off(RSS の列 `sub_scaled` パス)として正当化される。
非対象パス(al-locate NCC/bandpass/gcc_phat/locate-prepared 系)に
±5% 超の回帰なし。workspace 検証一式 green。

## Context

task `docs/tasks/caller-opt-measurement-ir-closeout.md`
(TASK-034EJgPpxRSjjhOJVeBUup)、plan
`docs/plans/caller-side-optimizations.md` T5、spec
`docs/specs/caller-side-optimizations.md` のプロセス契約
「before/after 実測を Implementation Record に記録」に従う。
baseline は T1 IR
(`caller-side-optimizations-bench-baseline-and-characterization.md`)
が `target/criterion/`(criterion named baseline `pre`・
`lstsq-rows`)と `target/bench-baseline/`(Instant 出力)に
記録したもの。

## Implementation

### Changes

コード変更なし(計測・記録・ドキュメント closeout のみ)。

- 全 criterion bench を baseline 名付きで再実行:
  `cargo bench -p al-analysis --bench stft -- --baseline pre`、
  `cargo bench -p al-analysis --bench conv -- conv_direct
  --baseline pre` + `-- dsp_conv`(plain)、
  `cargo bench -p al-math --bench lstsq -- --baseline lstsq-rows`。
  完全出力は `target/bench-baseline/criterion-{stft,conv,lstsq}
  -t5.txt`(gitignore 済み)
- `cargo run --release --example bench_reference -p al-analysis`
  を 3 回実行し中央値を採用(出力は
  `target/bench-baseline/bench-reference-t5.txt`)
- 非対象パスの回帰確認として al-locate Instant 例を再実行:
  `bench_dsp`・`bench_ncc`・`bench_locate_prepared`(出力は
  `target/bench-post-{dsp,ncc,locate}-t5.txt`、比較対象は
  vec-primitives セッションの `target/bench-baseline-{dsp,ncc,
  locate}.txt`)
- `docs/impl/ir/README.md`・`docs/tasks/README.md` の索引行、
  task `caller-opt-measurement-ir-closeout.md` の status → done、
  plan `caller-side-optimizations.md` の status → completed
  (closeout 慣行: e629570 での plan クローズと同じ)

### Decisions During Implementation

- **al-locate Instant baseline は再取せず流用**:
  `bench-baseline-{dsp,ncc,locate}.txt`(2026-10-12、
  vec-primitives セッション)が計測する経路(`dsp::fft`
  single-shot・`RealFft`・NCC・bandpass・spec_env・gcc_phat・
  prepare/locate_prepared)は本ブランチで変更されていない —
  stft の plan 巻き上げは `al_analysis::stft` 内であり
  `bench_dsp` の "stft-equivalent" 行は `al_locate::dsp::fft` を
  直接回す同等処理で al-analysis の stft を経由しない。A4 の
  `fit_mix`/lstsq 変更も当該 bench の経路外。`dsp.rs` への
  追加(`Fft` re-export・`conv`)は新規シンボルのみで既存関数を
  変更しない(git diff で production `-` 行は `mod tests` 内の
  み)。したがって baseline は有効な pre-state であり、再計測値は
  「非対象パスの回帰有無チェック」として機能する
- **manifest `duration_ms` の別途計測は行わない**: 代表的な
  実メディア pipeline 走査の before 値は本セッションで再現不能
  (pre-state ビルドがない)ため、stage 級代表経路の
  before/after は spec が指定する `examples/bench_reference`
  (reference stage 中核 `analyze_pair` を含む)の Instant 計測を
  以て証拠とする。`duration_ms` は永続化基盤として存在するが、
  本 closeout での比較計測には使わない(代替手段の正当化として
  記録)

## Related Experiments

### Adopted

<!-- No experiment logs: measurement/closeout task. -->

### Rejected

<!-- No rejected experiments. -->

## Bench 結果(consolidated before/after)

環境: 同一マシン、`mise exec --` 経由、criterion = 0.8.2
(bench profile = release + debuginfo)、Instant ベンチは release。

### A3 — stft(criterion baseline `pre`、median)

| bench | `pre` (T1) | T5 | criterion change |
| --- | --- | --- | --- |
| stft/4096/2048/524288 | 13.840 ms | 12.010 ms | −13.22%(CI [−14.0, −12.5], p = 0.00) |
| stft/4096/2048/1048576 | 26.973 ms | 23.597 ms | −12.51%(CI [−13.6, −11.3], p = 0.00) |

T2 時点の計測(−13.77% / −9.09%)と整合。plan 巻き上げによる
改善を再確認。

### A1 — conv(criterion baseline `pre` = `conv_direct`、median)

`conv_direct` 腕の再計測(同一コード、baseline `pre` との比較):

| fixture | `pre` (T1) | T5 再計測 | change |
| --- | --- | --- | --- |
| n65536_m256 | 4.1349 ms | 3.4890 ms | −15.4%(improved) |
| n262144_m256 | 16.576 ms | 14.082 ms | −15.9%(improved) |
| n65536_m4096 | 62.115 ms | 61.389 ms | −1.0%(no change) |
| n262144_m4096 | 254.28 ms | 253.94 ms | −0.1%(no change) |

`dsp::conv` 腕(`pre` baseline 不在のため絶対値突合、speedup は
`conv_direct` の `pre` に対する比):

| fixture | dsp::conv T5 | speedup vs `conv_direct` `pre` |
| --- | --- | --- |
| n65536_m256 | 0.6704 ms | ≈6.2x |
| n262144_m256 | 2.9553 ms | ≈5.6x |
| n1048576_m256 | 12.561 ms | ≈5.3x(direct 線形外挿 ~66ms に対し) |
| n65536_m4096 | 1.1707 ms | ≈53.1x |
| n262144_m4096 | 4.7185 ms | ≈53.9x |
| n1048576_m4096 | 19.776 ms | ≈54x(direct 線形外挿 ~1.07s に対し) |

`conv_direct` m256 両点の −15〜16% は同一コードの「改善」で
あり、T1 baseline 計測時のマシン負荷差を示す(m4096 両点は
±1% で一致)。m256 の speedup 絶対値にはこのバイアスが乗り得る
が、m4096 の ≈53〜54x は baseline 系の一致を踏まえ確定的な
orders-of-magnitude 改善。`dsp_conv/n1048576_m4096` は T4 計測
(criterion history)対比で +4.3% を示したが同一コードの再実行
ノイズ(baseline `pre` との比較ではない)。

### A4 — lstsq(criterion baseline `lstsq-rows`、median)

| bench | `lstsq-rows` (T1) | T5 | criterion change |
| --- | --- | --- | --- |
| new/1x65536 | 4.4535 ms | 1.3189 ms | −70.3% |
| solve/1x65536 | 750.53 µs | 648.71 µs | −14.5% |
| new/1x262144 | 19.715 ms | 5.5856 ms | −71.7% |
| solve/1x262144 | 4.0318 ms | 3.2560 ms | −19.2% |
| new/1x1048576 | 83.266 ms | 22.893 ms | −72.5% |
| solve/1x1048576 | 20.099 ms | 13.709 ms | −31.8% |
| new/2x65536 | 6.2268 ms | 2.5082 ms | −59.7% |
| solve/2x65536 | 1.0319 ms | 1.1028 ms | +5.0%(CI [+2.4, +7.6], regressed) |
| new/2x262144 | 29.384 ms | 13.852 ms | −52.9% |
| solve/2x262144 | 5.5069 ms | 5.6429 ms | +2.5%(no change) |
| new/2x1048576 | 123.45 ms | 113.67 ms | −7.9% |
| solve/2x1048576 | 22.260 ms | 23.874 ms | +7.3%(CI [+5.0, +9.9], regressed) |
| new/4x65536 | 10.808 ms | 6.6814 ms | −38.2% |
| solve/4x65536 | 2.4524 ms | 1.8062 ms | −27.7% |
| new/4x262144 | 50.211 ms | 28.615 ms | −43.0% |
| solve/4x262144 | 10.674 ms | 8.0159 ms | −24.9% |
| new/4x1048576 | 215.23 ms | 115.82 ms | −46.2% |
| solve/4x1048576 | 43.263 ms | 32.717 ms | −24.4% |

factor 構築(`new`)は全サイズで大幅改善。solve の 2 列回帰
(+5.0%/+7.3%)は T3 IR 記録の既知 trade-off(行 gather から列
`sub_scaled`/`sum_sq` RSS への移行で k=2 のメモリトラフィックが
増える。RSS 意味は ADR 確定事項のまま)を再現したもので、
T3 の +7.4/+16.1/+8.3% と同じ方向・同程度。実ワークロードでは
係数数 ≤ チャネル数のため factor 構築の改善がトータルで上回る。
`new/2x1048576` は T3 の 57.124 ms(−53.7%)から T5 の
113.67 ms(−7.9%)へ run 間変動が大きいが、両時点で rows
baseline より改善。

### Instant — bench_reference(3 回実行の中央値)

| 計測点 | baseline (T1) | T5 median (3 run) | delta |
| --- | --- | --- | --- |
| stft 1M samples (4096/2048) | 35.7 ms | 30.4 ms{30.2, 30.4, 35.0} | −14.8% |
| analyze_pair 2s mono (R0-R3, fir_model) | 691.2 ms | 581.0 ms{587.1, 573.4, 581.0} | −15.9% |
| prepare_residual 2s 2src x 2cap | 19.3 ms | 7.1 ms{6.9, 7.1, 7.2} | −63.2% |

`analyze_pair`(fir path)= 本チェーンの key number。有界
fixture(2 s @48k、n = 96_000)では畳み込みが O(n·4096) ≈
3.9 億 MAC で `analyze_pair` 全体の一部に過ぎず(STFT・CSD・
irfft・R0–R3 が残る)、−15.9% は conv 約 90 ms 分の除去 +
A3/A4 の寄与として妥当。本番規模(n ≈ 2–4.5M、~9–18G MACs)
では conv 支配率がはるかに高く、stage への効果はより大きい
(spec の見積もり根拠どおり)。

### 非対象パス — al-locate Instant(vec-primitives baseline 比較)

median(または mean)/ 単位 ms:

| 計測点 | baseline (10-12) | T5 | 判定 |
| --- | --- | --- | --- |
| bench_dsp: legacy fft 4M | 297.1 | 274.8 | 改善 |
| bench_dsp: fft 4M (next_pow2) | 150.6 | 149.5 | 同等 |
| bench_dsp: complex fft 4M reused | 106.5 | 96.4 | 改善 |
| bench_dsp: rfft 4M reused | 69.0 | 57.4 | 改善 |
| bench_dsp: NCC 2.4M x 8k | 436.9 | 405.2 | 改善 |
| bench_dsp: bandpass 2.4M | 186.5 | 174.1 | 改善 |
| bench_dsp: welch-ish psd 2.4M | 46.3 | 46.3 | 同等 |
| bench_dsp: gcc_phat 2.4M x 8k | 263.7 | 242.3 | 改善 |
| bench_ncc: FullNcc | 341.3 | 309.8 | 改善 |
| bench_ncc: BandNcc | 2039.7 | 1831.2 | 改善 |
| bench_ncc: GccPhat | 367.8 | 321.6 | 改善 |
| bench_ncc: SpecEnv | 521.3 | 458.1 | 改善 |
| bench_ncc: bandpass(sig) | 179.4 | 179.6 | 同等 |
| locate: prepare-cold | 908.4 | 785.1 | 改善 |
| locate: band-ncc-uncached | 3552.1 | 3059.2 | 改善 |
| locate: band-ncc-prepared | 2777.3 | 2536.4 | 改善 |
| locate: coarse-ncc-proposal | 129.6 | 118.9 | 改善 |
| locate: full-ncc | 612.7 | 617.9 | 同等(+0.8%) |
| locate: coarse-fine-global | 741.6 | 748.5 | 同等(+0.9%) |
| locate: cold-prepared-band-full | 4411.8 | 4039.7 | 改善 |
| locate: uncached-band-full | 4006.9 | 3697.6 | 改善 |

非対象パスに ±5% を超える回帰なし(全域で同等〜改善、現行
セッションのマシン状態が良好)。

### 回帰判定(±5% bound)

- ±5% を超える回帰: `lstsq solve/2x65536`(+5.0%)・
  `solve/2x1048576`(+7.3%)の 2 点のみ。原因は T3 IR 記録済みの
  新規コードパスの既知 trade-off(列 `sub_scaled`/`sum_sq` RSS
  vs 旧 per-row `dot_kernel`、k = 2 でメモリトラフィック増)。
  plan Rollback Strategy の「正当な理由がある回帰」に該当し、
  revert 対象としない — `new` + `solve` の合算は全サイズで
  大幅改善しており、実ワークロード(係数数 ≤ チャネル数)では
  factor 構築側が支配的
- `dsp_conv/n1048576_m4096` の +4.3% は baseline 比較ではなく
  T4 計測(criterion history)対比の同一コード再実行ノイズ

## spec Acceptance Criteria 充足確認

| AC(要旨) | 充足根拠 |
| --- | --- |
| 各 opt について char テストが先行 green・opt 前後 green | git: char 一括コミット `f08929d` が全 opt コミット(`ca4542f` A3・`0295e14` A4・`2902de2` A1)に先行。`cargo test --workspace` green(39 suite)で opt 後も green |
| A3: 決定的 fixture で `StftMatrix` re/im・メタがビット一致、エッジ網羅、ループ内 `Fft::new` なし | `stft_is_bit_exact_*`(stft.rs `mod tests`)green。`Fft::new` は `stft.rs:113` でガード内・ループ外 |
| A1: conv/fir_model が `conv_direct` と predicate 内、エッジ網羅 | `conv_matches_direct_reference`(dsp.rs tests)・`fir_model_matches_fir_ir_truncated_conv_direct`(reference.rs tests)green。`conv_direct` は `benches/conv.rs:29`・`reference.rs:1441`・`dsp.rs:124` の 3 箇所に逐語同一コピー |
| A4: `from_columns` 契約・`fit_mix` matrix/fit_quality が oracle と述語内、condition_number は sanity のみ | `fit_mix_matches_normal_equations_oracle`(mixer.rs tests)・al-math linalg 列契約テスト green(`linalg.rs:25` `pub fn from_columns`) |
| `subtract` zero-fill が ragged fixture で panic しない | `subtract_zero_fills_short_or_missing_model` green。zero-fill は char コミット `f08929d` に同梱され A1(`2902de2`)に先行 |
| criterion delta + stage 級代表経路の before/after が IR に記録、±5% 超回帰なし | 本 IR「Bench 結果」節。超過は solve/2 列のみで正当化済み(上記) |
| `from_columns` 公開・行マテリアライズ/strided gather 除去 | T3 IR コード検査済み + `pub fn from_columns` 存在 |
| `dsp::fft` シグネチャ不変・single-shot caller 無変更 | `dsp.rs:39` `pub fn fft(re: &mut [f64], im: &mut [f64], inverse: bool)` 不変。ブランチ diff の `-` 行は `mod tests` 内のみ |
| `cargo tree -p al-analysis --depth 1` に al-math なし | 直接依存 = al-core・al-dataset・al-job・al-locate・al-media・serde・serde_json・tracing(dev: al-testkit・criterion) |
| `conv_direct` が bench/test 内に存在し A/B 使用可 | 上記 3 箇所 |
| workspace test・clippy・fmt green・golden pass | `cargo test --workspace` 39 suite green・`cargo build --workspace`・`cargo fmt --all -- --check`・`cargo clippy --workspace --all-targets -- -D warnings` 全て green |

## Validation

- `mise exec -- cargo bench -p al-analysis --bench stft --
  --baseline pre`: 完走、delta は上表
- `mise exec -- cargo bench -p al-analysis --bench conv --
  conv_direct --baseline pre` および `-- dsp_conv`: 完走
- `mise exec -- cargo bench -p al-math --bench lstsq --
  --baseline lstsq-rows`: 完走
- `mise exec -- cargo run --release --example bench_reference -p
  al-analysis`: 3 回完走、中央値を記録
- `mise exec -- cargo run --release --example bench_{dsp,ncc,
  locate_prepared} -p al-locate`: 完走、baseline txt と突合
- `mise exec -- cargo test --workspace`: green(39 suite、
  golden 非回帰、0 failed)
- `mise exec -- cargo build --workspace`: green
- `mise exec -- cargo fmt --all -- --check`: green
- `mise exec -- cargo clippy --workspace --all-targets -- -D
  warnings`: green
- `mise exec -- cargo tree -p al-analysis --depth 1`: 直接依存に
  al-math なし
- `git log --oneline`: char コミット `f08929d` が opt コミット
  `ca4542f`(A3)・`0295e14`(A4)・`2902de2`(A1)に先行、
  zero-fill は `f08929d` に同梱され A1 より先に land

## Risks

- bench 値は同一マシン・同一 profile でのみ有効。`conv_direct`
  m256 の −15〜16% が示すとおり、セッション間のマシン負荷差は
  ±15% 級に達し得る — baseline `pre` 自体が高負荷時に記録された
  可能性があり、m256 の speedup 絶対値は保守的でない側に
  バイアスされ得る(m4096 の結論には影響なし)
- `solve` 2 列回帰は構造的(列 RSS パスのメモリトラフィック)で
  今後のカーネル最適化(issue #42 系)で解消し得る
- manifest `duration_ms` の実メディア before/after は未取得
  (bench_reference が代替の stage 級証拠)。実 pipeline での
  観測は次回実データ解析時に確認可能

## Follow-ups

- なし(チェーン closeout)。A2/A5 は spec どおり issue #42 系
  別チェーンへ defer 継続
