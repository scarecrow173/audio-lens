# WAV-RANGE-008 local proof verification ledger

対象: `WavDataLayout::range_plan`(`crates/al-media/src/wav.rs`)と、production
`read_wav_range` からの呼出。baseline は `evidence/al274-boundary/` の
WAV-RANGE-008(Required・Bounded Exhaustive local・proof 5分以内・4GiB未満)。

## 最終 verdict(2026-11-26, origin/main `fb95c86` merge後)

| proof | verdict | elapsed | raw verification time | 証跡 |
| --- | ---: | ---: | ---: | --- |
| `wav_range_plan_valid_ranges_exact` | **NotRun(timeout)** | 300012ms | 未取得(timeout) | `kani/kani-wav_range_plan_valid_ranges_exact.txt` |
| `wav_range_plan_invalid_ranges_error` | **Pass** | 4579ms(final subset) / 3406ms(revert後再確認) | 0.06818947s | `kani/kani-wav_range_plan_invalid_ranges_error.txt`, `kani-invalid-final/` |

final subset command:

```sh
AL_KANI_HARNESSES="wav_range_plan_valid_ranges_exact,wav_range_plan_invalid_ranges_error"   AL_KANI_TIMEOUT_MS=300000   AL_KANI_LOG_DIR=evidence/al274-wav-range-proof/kani   mise run test-formal-kani
```

lane verdict は `not_run` で `1/2 harness(es) verified`。valid proof は budget
300 秒で timeout したため **Required NotRun** であり、この slice は Issue
受入条件の「代表的な局所形式検証が予算内で証明完了する」を満たさない。
timeout/exit 0/CI green を Pass に見せない。

## 保証境界

- 保証対象: valid range (`start < end <= total`) の total/frame count、endpoint
  product difference による byte length、byte offset、exclusive byte end、
  data extent 内 containment、arithmetic overflow/underflow の不存在。
  invalid range (`start >= end` または `end > total`) の必須拒否。
- 入力領域: `block_align` は production 表現どおり u16、public frame index は
  u64。valid proof は valid end が u32-derived total に収まることを利用して
  direct u32 start/end へ exact reparameterization する。invalid proof は u64
  start/end を直接 symbolic にする。
- layout invariant: `block_align != 0`、data length は frame 境界、
  `data_len = total_frames * block_align`、data extent は RIFF u32 size が表す
  file length(`u32::MAX + 8`)内。これは任意 narrowing ではなく reader が
  受け入れる WAV container の exact invariant。
- 未証明領域: 実 I/O short read、seek/read error、allocation 失敗、PCM decode、
  diagnostic formatting、公開 API 全体 no-panic。local proof の成功をこれらに転用しない。

## 反復と失敗記録

| 形態 | solver | 結果 | 証跡 |
| --- | --- | --- | --- |
| u128 widened oracle / whole u64 symbolic layout | CaDiCaL | valid timeout 300000ms、invalid Pass 5465ms | `kani-initial-timeout/` |
| u64 direct oracle | CaDiCaL | valid timeout 300020ms | `kani-valid-u64-oracle-timeout/` |
| RIFF exact invariant + u64/native multiplication | Kissat | valid timeout 300007ms | `kani-valid-kissat-wide-timeout/` |
| u32 multiplication 初期形 + constructor division | Kissat | valid timeout 300016ms | `kani-valid-u32mul-constructor-timeout/` |
| constructive reparameterization + native multiplication | Kissat | valid timeout 300011ms | `kani-valid-constructive-kissat-timeout/` |
| constructive reparameterization + native multiplication | Bitwuzla 0.9.1 | valid timeout 300170ms | `kani-valid-bitwuzla-nativemul-timeout/` |
| production shift/add + native multiplication oracle | Bitwuzla 0.9.1 | valid FAILED(52792ms、byte_len oracle assertion)。concrete regression は production multiplication の一致を示すが、最終形には採用しない | `kani-valid-shiftadd-bitwuzla-fail/` |
| production shift/add + native multiplication oracle | Kissat | valid timeout 300018ms | `kani-valid-shiftadd-nativeoracle-kissat-timeout/` |
| endpoint/native/direct u32 start/end | Bitwuzla 0.9.1 | valid timeout 300018ms | `kani-valid-endpoint-native-direct-bitwuzla/` |
| shift/add + flattened ascending bit expansion oracle | Kissat | valid timeout 300012ms | `kani-valid-linear-flattened/` |
| branchless mask + assert/assume local lemma decomposition | Z3 5.3.3 | valid timeout 300068ms | `kani-valid-branchless-z3/` |
| final conservative primitive widening endpoint form | CaDiCaL | valid timeout 300012ms、invalid Pass | `kani/` |

standalone QF_BV benchmark でも、zero-extension した u32×u16 multiplication
monotonicity は z3/cvc5/bitwuzla で即時には解けなかった。根本原因は製品
コードの concrete defect ではなく、bitvector solver に対する exact nonlinear multiplication monotonicity・containment 証明のコストと判断する。

## 旧新 cost 比較

| proof系 | 旧 whole-call proof | 新 local proof | 比較 |
| --- | --- | --- | --- |
| valid range | trial 900s timeout/peak 約2.5GiB、canonical 1500s timeout | 300012ms timeout。peak memory は未測定(補助 monitor は WSL lifetime で継続せず)。timeout 中の観測 RSS は約45-56MiB | solver time は依然 budget 超過。観測 RSS は旧 evidence より小さいが、systematic peak ではないため Pass根拠にしない |
| invalid range | trial 600s timeout/peak 約5.2GiB、canonical 1500s timeout | Pass、elapsed 4579ms/3406ms、raw verification time 0.06818947s | local 化で verdict が得られ大幅改善。ただし valid Required NotRun のため property 全体は未完了 |

peak memory は canonical lane が直接収集しない。今回は未測定として記録し、
4GiB未満を証明済みとは扱わない。timeout 中の `ps` 観測は補助情報に留める。

## Mutation(Able to Fail)

一時 mutation: `WavRangePlanError.total_frames` を `self.total_frames` から
`0` へ変更した。command:

```sh
AL_KANI_HARNESSES="wav_range_plan_invalid_ranges_error"   AL_KANI_TIMEOUT_MS=300000   AL_KANI_LOG_DIR=evidence/al274-wav-range-proof/mutation-kani   mise run test-formal-kani
```

結果は exit 1・lane verdict `fail`・elapsed 4279msで、raw log に
`VERIFICATION:- FAILED`/`Verification Time: 0.09302156s` を記録した。
直後に backup から完全 revert し、`total_frames: self.total_frames` の存在と
format を確認した。revert後の invalid proof は `kani-invalid-final/` で PASS。
最終 diff に mutation は残っていない。

なお最初に試した `end_frame > total` guard removal mutation は、mutation後も
out-of-range arithmetic を solver が処理するため即時 verdict が得られず abort
した。この失敗も `actions.jsonl` に残し、検出 evidence には使わない。

## 結論

- production pure helper 化、production 呼出、invalid exact-complement proof、
  proptest 維持、mutation detection、runner evidence elapsed 記録は実装できた。
- しかし Required valid local proof は exact domain・300秒 budget で完走せず、
  WAV-RANGE-008 の Required assurance は **NotRun**。
- したがって本 slice は失敗として報告する。weakened proof や lemma assumption を
  Pass とはしていない。
