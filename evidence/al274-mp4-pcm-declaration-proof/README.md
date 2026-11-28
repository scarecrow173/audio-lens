# evidence: MP4 PCM declaration/capability selective exact proof

Issue #274 の partial slice `feature/274-mp4-pcm-declaration-proof` の証跡。対象は MP4-PCM-DECL-006 の fixed scalar sound-entry declaration domain における PCM declaration/capability classification と sample-width supportである。MP4 sample table、PCM extraction/conversion、parser integration、AVI、WAV conversion/writer は対象外。

## 読む順番

1. [boundary-analysis.md](boundary-analysis.md): 実装前に確定した入力領域、exactness、production seam、assume/stub 方針、除外範囲。
2. [verification.md](verification.md): Runtime/Formal Verification、mutation、byte-for-byte revert、post-revert lane、NotRun/NotApplicable。
3. [ledger.json](ledger.json): property、verdict、proof、cost、mutation、residual scope の機械可読 ledger。
4. [cost-comparison.md](cost-comparison.md): 関連 legacy proof と MP4 scan/boundary proof との限定 cost 比較。
5. [actions.jsonl](actions.jsonl): baseline、実装、失敗した初回 fmt、mutation、revert、最終 hygiene の時系列。
6. [environment.txt](environment.txt): OS/toolchain/Kani/CBMC 環境。

## 主な raw evidence

- Required proof: `kani/verdict.json`、`kani/kani-mp4_pcm_*.txt`。
- Mutation: `mutation/lane.txt`、`mutation/kani/verdict.json`、`mutation/kani/kani-mp4_pcm_*.txt`。
- Byte-for-byte revert: `mutation/pre-mutation.sha256`、`mutation/post-revert.sha256`、`mutation/revert-check.txt`。
- Post-revert proof: `kani-post-revert/verdict.json`。
- Regression: `kani-mp4-scan-regression/verdict.json`、`kani-mp4-avi-boundary-regression/verdict.json`。
- Runtime/hygiene: `check.pre.txt`、`lint.pre.txt`、`test.pre.txt`、`test-formal.pre.txt`、`fmt.final.txt`、`node-check.final.txt`、`prek.final.txt`。

失敗・timeout・not_run・abort・再試行を削除していない。初回 `mise run fmt` の失敗は `fmt.pre.txt` に残し、最終 Pass は `fmt.final.txt` に記録する。
