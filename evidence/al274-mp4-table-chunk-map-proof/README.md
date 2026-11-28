# evidence: MP4 sample table stsc/chunk-map compositional exact proof

Issue #274 の partial slice `feature/274-mp4-table-chunk-map-proof` の証跡。対象は MP4-TABLE-005a/b の bounded local proof であり、MP4-TABLE-005 全体の Pass 化ではない。

## 読む順番

1. [boundary-analysis.md](boundary-analysis.md): 最終 compositional boundary、production seam、exact input domain、assume/stub 方針、resource abort による設計根拠。
2. [verification.md](verification.md): Runtime/Formal Verification、mutation、byte-for-byte revert、post-revert lane、NotRun/NotApplicable。
3. [ledger.json](ledger.json): property、verdict、proof、cost、mutation、residual scope の機械可読 ledger。
4. [cost-comparison.md](cost-comparison.md): direct proof の resource abort と最終 proof family の限定 cost 比較。
5. [actions.jsonl](actions.jsonl): baseline、proof redesign、operator error、abort、canonical、mutation、revert、最終 hygiene の時系列。
6. [environment.txt](environment.txt): OS/toolchain/Kani/CBMC 環境。

## 主な raw evidence

- Required proof: `kani-canonical/verdict.json`、`kani-canonical/kani-mp4_*.txt`。
- Mutation: `mutation/lane.txt`、`mutation/kani/verdict.json`、`mutation/kani/kani-mp4_*.txt`。
- Byte-for-byte revert: `mutation/pre-mutation.sha256`、`mutation/post-revert.sha256`、`mutation/revert-check.txt`。
- Post-revert proof: `kani-post-revert/verdict.json`。
- Regression: `kani-mp4-scan-regression/verdict.json`、`kani-mp4-avi-boundary-regression/verdict.json`、`kani-mp4-pcm-declaration-regression/verdict.json`。
- Runtime/hygiene: `native/`。

失敗・timeout・not_run・abort・再試行を削除していない。`aborted-*.txt` と `kani-canonical-aborted-*` は製品反例ではなく、solver/memory budget の設計根拠である。操作ミスで一度 group 指定なしの全件 lane を開始した記録も `actions.jsonl` に残す。
