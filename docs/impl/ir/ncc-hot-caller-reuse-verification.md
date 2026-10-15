---
id: "IMPL-034FRBaMddoJNJ1MS8Oskb"
type: "impl"
status: "completed"
title: "NCC hot caller reuse verification"
created: "2026-10-14"
updated: "2026-10-14"
owners: ["scarecrow173"]
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
    - "docs/tasks/ncc-hot-caller-reuse-verification.md"
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# NCC hot caller reuse verification

## Summary

band_gain::windowsとcheap IPD_lagsの2siteだけをloop外plan/workspaceへ移行した。
large firstcold/所有/drop ACは基盤IR、micro観測とactual全caller確保削減は本IRで対応する。
借用curveをそのiterationで消費し、FFT/相関bufferを再構築しない。完成Judge ACCEPT、status/証拠を含む論理commit後にGraph終了監査する。

## Context

数学基盤commit e997b7f661e6e4a13513e57e8a5a26e4acba8090からのslice3。
IPD callerfix f12e920は先行独立commit。旧microfree f12e920、newmathfree e997b7f、reuseを区別する。

## Implementation

短入力guard後、band(window,window+2max)、IPD(block,block+2max)のexactshapeplan/wsを
各呼出のloop外で1回構築。signed原点/tie/polarity/gain/guard/100ms/firstpairの処理は維持する。

al-analysisはal-math directdepを持たず既存al_locate::dsp経由で数学を消費する。
直接importのE0433を新依存で解決せず、dsp.rsの既存相関pub useへ公開2型を1行追加した。
coordinatorは既存API責務境界の最小拡張と確認。al-locate callerの移行/数学実装移動はない。
pulp/backend型は公開せず、Cargo.toml/Cargo.lockを変更しない。

## Related Experiments

採用判定は独立数値/所有検証と実caller/microの観測。追加性能gateを作らない。

## Validation

- `mise exec -- cargo test -p al-analysis --lib`: 99passed（band gain、IPD6、他producer契約）。
- `mise exec -- cargo test -p al-analysis --test golden`: 19passed。
- `mise exec -- cargo clippy -p al-analysis -p al-locate --all-targets -- -D warnings`: exit0。
- `mise exec -- cargo fmt --all -- --check`: exit0。

math基盤の独立NCC/DFT PHAT oracle/firstcold/共有/shape/state/drop55件はslice2で実施済み。
本sliceでその検証を済ませたと再主張せず、基盤IRのscopeと時点を参照する。

### Actual production owner paths

10秒stereo48kHz、noise seed0x52・lag -3/+5、30iterations、band198windows/IPD99blocks。
measuredscopeは実際のprivatewindowsとipd_lags（band filtering/cheap全pipelineは除外）。
サンプルはproduction al-analysis unit binaryへ一時ignored testを追加して採取した。
body-copy surrogateは採用していない。

beforeはe997b7f fulltrackedsourcearchiveをrepo内targetへ隔離展開し、同harnessを付け
`cargo test --release --locked --manifest-path ... -p al-analysis --lib ncc_caller_measure` を実行。
afterはliveproduction2hotの同harness/flagsで実行。両measuredbinaryを直後保存しSHAを記録。
sourcearchive/helper/fixture/raw30samples/UTCinterval/PID/phaseRSSを両variantで保持する。

| Actual path | before p50/p95 ms | after p50/p95 ms | observed p50 ratio | entire caller alloc calls | allocated bytes |
| --- | --- | --- | --- | --- | --- |
| band windows198 | 64.868 / 119.8128 | 53.5591 / 58.5154 | 1.211x | 7921→40 | 260642448→1323560 |
| IPD blocks99 | 31.1737 / 33.2636 | 26.3239 / 27.9605 | 1.184x | 3961→40 | 130012344→1310960 |

wholecallerの確保は40残る（plan/ws/output等）。global allocation zeroを主張しない。
loop内plan/FFTbuffer再確保は除去し、基盤NCCcall0allocと呼出配置/全caller計数で確認する。
30回のrawdataで各iteration countが同じことも確認した。

timed scope外で全Window fields（floatbits含む）198行と全IPDlagbits99行をserializeし、
before/after bytesが完全一致。hash:

- Window: 41e905ed6692ea54b6d064fa839b0ab28c99c5bae6af15c53311a5eaa0b97a2a
- IPD: 7a8cca726f34d384e0178a05bc3a2aedddf7b8c1a255d4681b088cd1af0aa9d2

formalbefore manifest SHA00782701c8b24b6dc1c766c548380b5ef14a426a44bc7314d109fa57e26daff0、
formalafter SHA6af404ea9390228cbf7586c974667ed899372350b23fc86cbd5479205d55a05a。
artifactroot `target/ncc-reuse-evidence/production-before-e997b7f/` と `actual-after-e997b7f/`。

### Micro three arms

旧freebaseline f12e920とnewfree/reuse e997b7fを同fixture/各3000calls/30×100で比較。
6fixture bytehash一致、mode別cold/steady/p50/p95/RSS/binary/env/hashを保持。
postmicro manifest b87ed700a7a4747e5b22c33d69ed6940aee018f86f3dfa64c37d9e57963f0a40。

| shape/mode | newfree p50 ms | reuse p50 ms | observed newfree/reuse ratio |
| --- | --- | --- | --- |
| band NCC / PHAT | .3168 / .3202 | .2546 / .2553 | 1.244 / 1.254x |
| IPD NCC / PHAT | .3167 / .3187 | .2564 / .2573 | 1.235 / 1.239x |
| large NCC / PHAT | 10.5893 / 10.8158 | 6.2419 / 5.9098 | 1.696 / 1.830x |

reuse冷初回はconstructor+firstcall、steadyはborrowed curve copyなし。freeのownedout dropは旧baseline同様
timer停止後。旧free/reuse比は1.26/1.26、1.25/1.24、1.67/1.82xで、mathfree変更とreuseを混同しない。

### Setup/provenance failures and selection

一時harness extern空白compile error、cratecwdに相対出力先指定の保存failure、archiveRustsubsetの
dictionary include欠落を記録。いずれも製品の数値failureではなく修復後結果を採用する。

最初のactualbeforeはmeasuredbinary SHA保存漏れがあり非採用。正式4caseでprovenanceを修復し、
中央値も上表の正式比較へ更新した。旧before raw/source/summary/UTCは保持。
旧after summary/binary/初期intervalは保持したが、v1rawはv2同名出力で上書きされ未保持。
旧v1を再現済み/正式採用として扱わない。恣意的な速度選択の再測定はしていない。

microとactual性能実行は単独でUTCintervalを保存。harness準備compile時に待機を調整し、
perf並列重複なし。actualbefore/afterは時間的に分離し、OS背景負荷未統制、各variant1run。
特にbeforebandp95は高く、統計的有意性や一般的speedup保証を主張しない。
RSSはphasecurrentとprocesslifetimepeakを区別、RSS低下をowneddeallocation証拠としない。

## Risks

今回observedmedian改善と確保削減を報告するが、全job/全inputでの速度保証ではない。
恒久性能harnessは増やさず、一時helper/module/tailは自分の追加した部分だけ除去した。
raw/binary/sourcearchiveはignored targetのローカル保存でありcargo clean等で失われうる。
長期のcanonical証拠は本IRの条件・結果・hash要約（今回commit対象）で、rawをGitへ保存しない。
正式beforeはtarget内isolatedsourceでありproductionfilesを全面revertしていない。

## Follow-ups

完成差分を独立reviewし、Blockerなし＋修正可能な非Blocker対応後にcaller/doc/evidenceをcommit。
Graph task/plan/IR statusを同期しstrictcommit gate/exit監査を完了する。

## 2026-10-15 Completion Review

A/B/Validator追加0、23artifactSHA一致、production全出力byte同値、基盤loop無確保と
caller再利用・観測採用条件を確認してJudge ACCEPT、Blocker0。
統計的有意性/全job改善は承認対象ではなく、本IRの観測条件と限界を維持する。

## 2026-10-15 Graph Closure

caller論理commit227c095後、git status --porcelain空を確認。保持したimplementation baselineに
新dirtyがなくstrictcommit gate通過。全3task done、plan/IR completed、implementation-verified。
宣言route implementation-to-followup-triage（IR74/task120監査finding0）、
followup-triage-terminal（all253文書blocking0/finding0）、exit-audit-to-completeを実行し、
complete nodeのterminal/blockers0を確認。#33 focusedchainもcomplete terminalを再確認した。
#52のrunはtaskBudgetCount3/maxHops12、完了した辺をこの順に追跡。
元worktree F:/repositry/audio-lens はmain76c90fb、既存ロードマップdirty1件を保持。
依存manifest/lock差分なし、push/PR/mergeなし。
