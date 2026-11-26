# al#274 WAV-RANGE-008 range arithmetic local proof 証跡

本 directory は Issue #274 の WAV-RANGE-008 slice に対する実行証跡である。
`evidence/al274-boundary/` は read-only baseline であり、ここでは変更しない。
implementation state と実行結果は本 directory の `ledger.json`・
`actions.jsonl`・`verification.md` に記録する。

対象は `crates/al-media/src/wav.rs` の pure range arithmetic helper
`WavDataLayout::range_plan` と、それを呼ぶ production `read_wav_range` の
接続である。公開 API 全体 no-panic、実 I/O short read、allocation 失敗、
PCM decode、diagnostic formatting はこの local proof の保証範囲外である。

主要な生証跡:

- `kani-initial-timeout/`: 初回 u128 oracle・whole u64 symbolic layout での timeout
- `kani-valid-u64-oracle-timeout/`: u64 direct oracle での timeout
- `kani-valid-kissat-wide-timeout/`: RIFF exact invariant + Kissat での timeout
- `kani-valid-u32mul-constructor-timeout/`: u32 乗算化初期形での timeout
- `kani-valid-constructive-kissat-timeout/`: constructive reparameterization + Kissat での timeout
- `kani-valid-bitwuzla-nativemul-timeout/`: native multiplication + Bitwuzla での timeout
- `kani-valid-shiftadd-bitwuzla-fail/`: shift/add 実装 + Bitwuzla での反例 verdict
- `kani/`: 最終 canonical subset(verdict が NotRun/Fail の場合も raw log を配置)
- `mutation-kani/`: 一時 mutation の canonical subset(意図的失敗。実施できた範囲を verification.md に記録)

検証の台帳・旧新 cost 比較・mutation 記録は `verification.md` を参照する。
