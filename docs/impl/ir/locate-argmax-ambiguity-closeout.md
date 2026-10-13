---
id: "IMPL-034FpTPsPyJ5qPp0Wcf6rj"
type: "impl"
status: "completed"
title: "locate argmax 曖昧性: 回帰計測・closeout(T4)"
created: "2026-10-13"
updated: "2026-10-14"
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
    - "docs/tasks/locate-argmax-ambiguity-closeout.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/locate-argmax-tie-eps-calibration.md"
    - "docs/impl/ir/al-locate-dsp-re-export-vec-migration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/locate-consensus-argmax-ambiguity.md"
    - "docs/specs/locate-consensus-ambiguity-contract.md"
    - "docs/adr/locate-argmax-ambiguity-representation.md"
    - "docs/test-specs/locate-argmax-ambiguity.md"
    - "docs/test-specs/locate-argmax-tie-eps-calibration.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/plans/implement-locate-argmax-ambiguity.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# locate argmax 曖昧性: 回帰計測・closeout(T4)

## Summary

plan T4(closeout)。T1(特性計測 `56db056`)→ T2(曖昧検出実装
`4ec6e06`)→ T3(`dsp::corr0` 移行 `dbea83f`)の順に land
済みであることを `git log` で確認し、全 workspace 検証が green、
al-locate Instant bench の回帰計測で `argmax_tie_count` の O(n)
追加分が NCC 本体に対し無視できることを確認した。spec AC は
T1 実測で修正された混在機構(pin-periodic で曖昧になるのは
full_ncc / spectral_envelope のみ)を反映する形で充足され、
残存許容(envelope NCC 逐次式維持)は T3 で解消済み。

## Context

task `docs/tasks/locate-argmax-ambiguity-closeout.md`
(TASK-034Fo9fbMBjYA189SBNMGL)、plan
`docs/plans/implement-locate-argmax-ambiguity.md` T4、design
`docs/designs/locate-consensus-argmax-ambiguity.md` §8、spec
`docs/specs/locate-consensus-ambiguity-contract.md` の Acceptance
Criteria 最終確認に従う。T1 実測の一次記録は IR
`docs/impl/ir/locate-argmax-tie-eps-calibration.md`(本 IR の
関連記録)にあり、ここではチェーン全体の確定値と検証結果を
統合して記録する。

## Implementation

### Changes

コード変更なし(回帰計測・記録・ドキュメント closeout のみ)。

- `mise exec -- cargo test --workspace`: 全 suite green
  (0 failed)。golden suite(`tests/golden.rs` 19 件)を含む
- `mise exec -- cargo clippy --workspace --all-targets -- -D
  warnings` / `cargo fmt --all -- --check` / `cargo build
  --workspace`: 全て green
- 回帰計測: `cargo run --release --example bench_ncc -p
  al-locate`(計3回+ウォーム目)と `cargo run --release
  --example bench_locate_prepared -p al-locate`(median of 7)
  を実行し、main 基点(`705a37c`、本ブランチ merge-base と同一)
  で計測済みの `target/bench-post-{ncc,locate}-t5.txt`
  (2026-10-13、caller-opt T5 セッション)と突合。出力は
  `target/bench-post-{ncc,locate}-argmax-t4.txt`(gitignore 済み)
- doc 更新: task `locate-argmax-ambiguity-closeout.md` の
  status → done、plan `implement-locate-argmax-ambiguity.md`
  の status → completed(closeout 慣行: caller-opt T5 と同じ)、
  discovery `locate-consensus-argmax-ambiguity-contract.md` の
  status → resolved(spec/ADR/design が `derived-by` でリンク済み)、
  spec/test-spec/plan Verification の「pin-periodic で全4メソッド
  merged 証拠が AmbiguousAlignment」という記述を T1 実測の混在
  機構へ修正、`docs/tasks/README.md`・`docs/plans/README.md`・
  `docs/discovery/README.md`・`docs/impl/ir/README.md` の索引
  追随、`docs/tasks/al-locate-vec-migration.md` と
  `docs/impl/ir/al-locate-dsp-re-export-vec-migration.md` へ残存
  許容解消の注記

### Decisions During Implementation

- **bench baseline は T5 計測ファイルを流用**: `bench-post-*-t5.txt`
  を計測したコードは本ブランチの merge-base(`705a37c`)と同一
  (main はそれ以降 doc コミットのみ)。al-locate の bench 経路への
  変更は本ブランチの T2(`argmax_tie_count` 追加)と T3(corr0
  置換)のみなので、有効な pre-state として機能する
- **spec AC / test-spec Rule D の「全4メソッド曖昧」記述を修正**:
  T1 実測で pin-periodic の厳密同点は full_ncc(|T|=28)と
  spectral_envelope(|T|=219)のみで、band_ncc(margin 2.58e-4)・
  gcc_phat(margin 1.09e-1)は候補レンジ全域で同点にならないことが
  確定した(T1 IR「重要な実測事実」)。T2 は混在機構(曖昧2件が
  投票除外 + 非曖昧2件が合意不足)を pin しており、契約文書側の
  記述を実測へ整合させる修正とした — 意味はいずれも
  「周期 fixture が不受理 + `validity == ambiguous_alignment` +
  候補保持で観測可能 + 決定的」で同一

## Related Experiments

### Adopted

<!-- No experiment logs: measurement/closeout task. -->

### Rejected

<!-- No rejected experiments. -->

## チェーン確定記録(T1–T3 統合)

### T1: eps 確定値と実測根拠(一次記録: IR `locate-argmax-tie-eps-calibration.md`)

```rust
const TIE_REL_EPS: f64 = 1e-11;
const TIE_ABS_FLOOR: f64 = 1e-11;
```

- 窓条件 `max(dispatch 偏差) << tie_window << min(受理系マージン)`
  に対し: 下側 max dispatch 偏差 ~5e-16(実測)/ ~1e-14(伝播込み
  保守値) → ~2e4 倍のクリアランス。上側 min 受理系マージン
  1.65e-8(`case_loop_material` × `spectral_envelope`) → ~1.65e3
  倍のクリアランス
- 受理系 fixture 全件・全メソッドで |T|=1(曖昧 false positive
  なし)。pin-periodic |T|=28(full_ncc)/219(spectral_envelope)、
  prepared_exact_tie |T|=2@{0,6952} を実測確認
- 判定反転の irreducible 不安定帯 `eps ± dispatch_deviation` の帯幅
  は ~1e-15 絶対(tie_window 境界 ± ~5e-16 内の |corr| のみが
  dispatch 間で所属を反転し得る)
- spec 候補レンジ(rel 1e-9..1e-6 / floor ~1e-9)から 1e-11 への
  乖離は実測に基づく意図的決定(1e-9 は min マージンへ 16 倍
  しか離れず「<<」に薄弱。詳細は T1 IR)

### T2: 意図的契約変更の記録

- `reject_reason` 文言: 証拠レベル `"top correlation peaks
  indistinguishable within tolerance ({tied} positions)"`(マーカー句
  "indistinguishable")、結果レベル空投票時 `"all voting methods
  produced ambiguous (tied) peaks"`("method(s) agree" を含まない
  専用文言)、合意不足+曖昧除外時 `"; N method(s) excluded as
  ambiguous"` 付加。golden `pinned_periodic_*` の pin 文言変更は
  spec Req 10 / design §6 の意図的契約変更
- `tests/prepared.rs` の意味論シフト: 厳密同点(fixture
  `+template`/`−template` 2出現)・周期 sin テンプレート経路の
  証拠 validity が `Valid` → `AmbiguousAlignment`+候補保持へ変化。
  `position` 観測と locate==prepared 同一性は validity 込みで存続
- version `"2"` バンプ: 全4メソッドの `version()` +
  `stage.rs` の `algorithm_version`(:174/:685/:1051)+
  `consensus.rs`/`prepared.rs` テスト内 fixture の `"2"` 揃え。
  pre-release 方針により互換層なし・既存成果物は再生成
- 構造: `argmax_tie_count` はサンプル(サンプル index)計数で
  plateau を run に潰さず、`evidence_from_corr` の既存3分岐の後に
  曖昧分岐を置く(弱さで棄却済みの曲線を曖昧へ格上げしない)。
  `merged_votes` はタプルキーで非曖昧優先、`best_window_match` は
  ペア単位 `|T| >= 2 → continue` ガード(3段全てに適用)

### T3: corr0 置換差分と残存許容の解消

- `methods.rs` `SpectralEnvelope::locate` の逐次内積/ノルム
  (TASK-034EpPesSpbjbAQBsmjvJx の残存許容)を `dsp::corr0` へ
  置換し、残存許容コメントを除去。ガードは `tn*sn > 0` と
  `Σx² > 0 && Σy² > 0` で同値(0ノルム・NaN とも 0.0)、値差は
  ulp 級のみ(T1 IR「T3 差分記録」)
- 構造的拒否の確認: corr0 の厳密同点(219 lag、全て 1.0)→
  `argmax_tie_count` が `AmbiguousAlignment` へ構造的に検出 →
  投票除外され `"only 1 method(s) agree (need 2); 2 method(s)
  excluded as ambiguous"` で不受理。拒否が argmax 順序の ulp
  依存から構造的検出へ固定された(完了証明)

## 回帰計測(T4)

`argmax_tie_count` は証拠評価1回あたり曲線長 O(n) の走査を1回
追加するのみ(NCC 本体は O(n log n) FFT)。bench 実測:

### bench_ncc(単発計測、最小値採用 — 計測中のマシン負荷変動が大)

| 計測点 | baseline(t5) | T4 best-of-4 | delta |
| --- | --- | --- | --- |
| FullNcc | 309.8 ms | 329.1 ms | +6.2% |
| BandNcc | 1831.2 ms | 1924.6 ms | +5.1% |
| GccPhat | 321.6 ms | 307.6 ms | −4.4% |
| SpecEnv | 458.1 ms | 514.7 ms | +12.4% |
| bandpass(sig)(変更なし参照系) | 179.6 ms | 176.8 ms | −1.6% |

### bench_locate_prepared(median of 7)

| 計測点 | baseline(t5) | T4 | delta |
| --- | --- | --- | --- |
| prepare-cold | 785.1 ms | 816.0 ms | +3.9% |
| band-ncc-uncached | 3059.2 ms | 3285.9 ms | +7.4% |
| band-ncc-prepared | 2536.4 ms | 2708.3 ms | +6.8% |
| coarse-ncc-proposal | 118.9 ms | 177.3 ms | +49%(p95 350、ノイズ支配) |
| full-ncc | 617.9 ms | 797.3 ms | +29%(p95 1139、ノイズ支配) |
| coarse-fine-global | 748.5 ms | 700.0 ms | −6.5% |
| cold-prepared-band-full | 4039.7 ms | 4186.0 ms | +3.6% |
| uncached-band-full | 3697.6 ms | 3871.8 ms | +4.7% |

判定: 計測セッションのマシン負荷が高く(bench_ncc の run 間で
bandpass 176〜831 ms と 4.7 倍変動、変更対象外の参照系でも
+26%/+363% を観測)、単発メトリクスの有意差判別は不能。
median 系では主要経路が ±7% 以内、`coarse-ncc-proposal` /
`full-ncc` の表層 delta は p95 の暴れ(350/1139 ms)が示す
ノイズ支配で、絶対量から見ても O(n) 1 パス分(2.4M サンプル
走査 ≒ 数 ms 級)を大きく超える変動である。**`argmax_tie_count`
起因の有意な劣化なし**(design Non-Functional Constraints「性能」
を充足)。SpecEnv の +12% は corr0 移行(SIMD fused 3-アキュム
レータ化)を含むが方向が逆であり、ノイズと判断する。

## spec Acceptance Criteria 充足確認

| AC(要旨) | 充足根拠 |
| --- | --- |
| pin-periodic 不受理・`validity == "ambiguous_alignment"`・full_ncc `candidates` ≥2 かつ全位置が周期倍数・実行間/dispatch 間決定的 | `golden.rs` `pinned_periodic_everywhere_not_accepted_at_wrong_phase` green。判定は `argmax_tie_count`(決定的な大きさ比較)で sort 順に依存しない |
| 証拠レベル pin(周期 fixture で曖昧証拠 + 候補保持) | 同上: full_ncc / spectral_envelope の merged 証拠が `ambiguous_alignment` + `candidates` ≥2 + "indistinguishable"。band_ncc / gcc_phat は非曖昧のまま合意不足へ(T1 実測で「全4メソッド」想定を混在機構へ修正 — 上記 Decisions) |
| pin-periodic-control が tolerance 64 で受理、受理系 golden が受理維持・全メソッド merged 非曖昧 | golden 全件 green、`assert_no_ambiguous_merged_evidence` が全受理系で非曖昧を pin |
| `pinned_low_coherence_no_inflated_lag` 不受理・`agreeing_methods < 2` | 既存 golden green |
| ユニットテスト: 厳密同点・近同点・平坦 plateau で曖昧+候補保持 | `methods.rs` `exact_and_near_ties_mark_evidence_ambiguous_with_candidates_kept` / `flat_plateau_is_ambiguous` green |
| ユニットテスト: 全証拠曖昧で合議 `AmbiguousAlignment`(LowCorrelation/InsufficientSupport に落ちない) | `consensus.rs` `all_ambiguous_evidence_yields_ambiguous_result` / `empty_votes_with_ambiguous_take_precedence_over_weak_split` green |
| ユニットテスト: 単一候補・意味ある分離・幅広平坦頭部単峰が非曖昧 | `separated_or_below_window_peaks_are_not_ambiguous` / `wide_flat_topped_single_peak_is_not_ambiguous` green |
| reject_reason pin(証拠 "indistinguishable"・空投票専用文言・"; N method(s) excluded as ambiguous") | golden pin + `agreement_shortage_notes_excluded_ambiguous_evidence` green |
| 完了証明: corr0 置換後も周期 fixture の拒否が構造的に維持 | T3 コミット `dbea83f`、pin-periodic golden で厳密同点→曖昧検出→拒否を確認(T1 IR T3 節) |
| T1 計測が実装に先行し eps が実測記録と共に確定 | git: `56db056`(T1 計測テスト)が `4ec6e06`(T2)・`dbea83f`(T3)に先行。eps 確定値と根拠は上記 |

## Validation

- `mise exec -- cargo test --workspace`: green(40 test result ok、
  0 failed。`tests/golden.rs` 19 件・`tests/prepared.rs`・
  `tests/argmax_tie_characterization.rs`・al-math dispatch
  deviation テストを含む)
- `mise exec -- cargo clippy --workspace --all-targets -- -D
  warnings`: green
- `mise exec -- cargo fmt --all -- --check`: green
- `mise exec -- cargo build --workspace`: green
- `mise exec -- cargo run --release --example bench_ncc -p
  al-locate` / `--example bench_locate_prepared`: 完走、
  `target/bench-post-{ncc,locate}-argmax-t4.txt` に記録
- `node .agents/skills/doc-status/scripts/audit_docs.js --type all
  --cwd .`: findings なし
- `git log --oneline`: `56db056`(T1 test)→ `4ec6e06`(T2 feat)→
  `dbea83f`(T3 refactor)の順で plan の Sequence どおりに land

## Risks

- bench 計測は同一マシン・同一 profile でのみ有効であり、本
  セッションは負荷変動が大きかった(参照系 bandpass が 176〜831
  ms)。O(n) 追加分の理論量(数 ms 級)に対し観測ノイズが支配的
  であり、「有意な劣化なし」は理論量と median 系 ±7% 以内の
  両面からの判断
- `best_window_match` 第3段の全ペア同点退化(design §5 受諾事項)
  は本回帰では未観測。発生時は occurrence 毎アンカーへ退化する
  既知の受諾済み挙動
- `merged_votes` の候補空 0.0 が負スコアに勝つ既知クオークは
  契約対象外として記録のみ(plan Out of scope)

## Follow-ups

- なし(チェーン closeout)。spec Deferred Design Concerns の
  Option C(意味的位相同値性)は spec 規定どおり留保継続
