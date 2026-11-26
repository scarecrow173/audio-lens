---
id: "EVID-tL6qRwXzPbNsYjKfVmHd"
type: "evidence"
status: "approved"
title: "検証記録: Ruviform 移行・pin 更新・契約 API・消費者 oracle"
created: "2026-11-19"
updated: "2026-11-26"
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
  implements: []
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
---

# 検証記録: Ruviform 移行・pin 更新・契約 API・消費者 oracle

Ruviform 依存・pin 更新・契約 API 採用・consumer oracle に関する検証の事実記録。

## AL#101 f64 カーネル置換

- Golden 比較(`tools/baseline/compare.mjs`、rel 1e-9/abs 1e-12): 9 run を #98 凍結 golden と比較。差分 kind は `tolerance`/`value` のみ、missing/added/type/行数差 0。最大相対 ≈1.9e-8(`transfer_h_t` fingerprint `sum`)、`values/*` 要素は最大 ~2e-7・典型 ~1e-9、`candidates.rows` `measured` は ~1e-17 最終桁差。原因は scalar vs SIMD の累積順序差。
- baseline 差分ファイル(`issue-101-baseline/<run>-diff.txt`): canonical/aa/resume 各 45、small 1149、variant-desync 20、variant-clipping 42、variant-reverb 31、variant-filter 37、variant-clicks 51。この件数セットが後続の「#101 既知」分類列に完全一致で流用。
- 性能(small run): e2e ネット -1.7%(124612→122529ms)。microbench: analyze_pair +33%、stft +25%、corr0 sweep 4.0x、NCC plan +23%、conv +45%。e2e ネット中立だがカーネル単位退行は scalar 化の既知トレードオフ。
- 検証: fmt/`cargo test --workspace`(golden 19 件)・clippy・`cargo build --release`・`check-cli-headless`(al-math 除去)・`verify-toolchain` 全 pass。

## AL#102 責務分離

- 全 9 run の全差分が 2 クラスに機械分類され unexplained 0: (1) #101 既知クラス(surface/path/kind 完全一致)、(2) #100 decoder provenance(`*/params/decoder/*`、各 run 130-160 件)。baseline 差分ファイル `issue-102-baseline/*-diff.txt`(canonical/aa/resume 各 175=45+130、small 1309=1149+160)。
- `cargo test -p al-analysis -p al-locate` 全 pass(bit-exact 特性テスト `a7_freq_lr_coherence_is_bit_exact` 等、golden 19+16 件)。

## AL#103 回帰検証

- Live Job(`ingest,locate,cheap_analysis,reference_analysis,expensive_analysis,dataset,export` を Visualization なしで実行): 全 stage completed、`analysis/visualization/` 未生成のまま `export/analysis/`+`export_manifest.json`+`golden/snapshot.json`+`report.html`/`report.data.js` 生成。
- resume: locate 完了時点で kill → `al resume` で完走 → `resume` vs `canonical` **0 diffs**。`aa` vs `canonical` 0 diffs(実行間決定性)。
- headless reader 検証: table `events/candidates` 36 rows、`locate/summary` 2 rows、tensor `features/cheap|expensive|reference`、audio `inputs/*.wav`、json `locate/<src>/<cap>.json`。
- Provenance 確認: decoder attempts(backend=`ruviform`、version=`e467ccb…`、outcome、reason)、`settings.ruviform`{io=[ruviform-io-symphonia, ruviform-io-avi], rev, video_path=native_dib}、AAC-in-MP4→unscaled_interleaved_f64le 変換履歴。
- 検証: `mise run fmt`/`check`/`lint`/`test`/`build` 全 pass(全 41 セクション、新規契約テスト含む)。

## AL#104 最終撤去

- Golden 回帰(9 run): 全差分が既知 4 クラス(#101/#104 rev/#100/#106 期待)に帰属・unexplained 0。`golden/inputs/IN-#*/metadata.json` の `decoder.version`/`settings.ruviform.rev`/`attempts/N.version` が全件 Cargo.lock rev と一致。
- 性能(#98 凍結 `performance.json` 比較): e2e 8/9 run で中立〜改善(-8.2%〜+0%)、small は計測分散で +5.0%/-1.3%。locate -12.6%〜+0.7%、reference_analysis +14.6〜19.8%、expensive_analysis +33.9〜58.3%(絶対差 +1.6〜+3.4s)、export -36.1〜-38.4%(report.html 撤去由来)。
- bench: `bench_dsp` legacy_fft 4M 325.0ms→ruviform fft 184.6ms/plan 再利用 120.6ms/rfft 68.5ms。`bench_residual` sub_scaled 1src -19%・2src -53%。`bench_alloc` analyze_pair -2.7%/prepare_residual -35.8%。
- 検証: fmt/check/lint/test/build/verify-toolchain/check-cli-headless/`cargo build --release -p al-cli` 全 pass、`cargo test -p al-media -p al-locate -p al-analysis` 全 pass。

## AL#133 mixed-radix FFT pin bump

- Golden 差分(pre-pin vs post-pin): canonical/aa/resume 各 1257、small 1741、variant-desync 1050。structural・decision-path(offset/lag/time_map/accepted)は全 run 0。rev-stamp は想定内 provenance 差、tolerance は score/peak_to_background/peak_ratio_12 等(canonical `peak_to_background` 最大 ~6.4%)。
- **canonical e2e +10.2%/locate +17.1% の net 回帰を観測**(ruviform 側の変換長/mode 選択に起因、AL consumer 境界では回避策を取らず pin は Issue #133 の指示どおり採用)。変換長は narrow/widen/full で -2.3%〜-49.3% 削減するが tpl≈sig メソッドは direct→FFT 反転で悪化。
- 計測ノイズ管理: 並行負荷時は span busy/call が歴代の 3-6 倍に膨らむため初回を棄却しアイドル時再計測を採用。

## AL#117 + RF#167-170 契約 API 採用

- Golden 回帰(9 run): 全差分が機械分類に帰属・unexplained 0。分類表に `#117`=stamp `params/ruviform` key、`#170`=stable_id、`#104`=rev 一致の直接検証を追加。
- `complex_solve_matches_csolve_bit_exact`、records.rs stamp 形状・params 保持・rev 一致、cheap.rs IPD/self_sim `corr0` oracle `to_bits` 一致、normalize stable_id 改ざん検知を追加。
- `mise run fmt`/`check`/`lint`/`test`(46 suite 全 ok・0 failed)/`build`/`verify-toolchain`(1.99.0)/`check-cli-headless` 全 pass。

## AL#127 ruviform pin 87a7ba8f 追従

- candidate tuple: ruviform `87a7ba8ff3fbe6d4502bea27d152d7f74c105899` / AL pin bump commit `67cc975ec651e502ad863c24db1db104cfe025b3` / lockfile resolved `87a7ba8ff3fbe6d4502bea27d152d7f74c105899`。25 個の ruviform package が単一 SHA に解決され、manifest 非変更を維持。
- `Cargo.lock` 差分は 25 個の ruviform source fragment と `gpui` 0.2.2 の registry→`scarecrow173/gpui` tag `v0.2.2-rf2`（peeled `b3144f286b942fbb5f478da73db13a826cef054a`）source switch のみ。package 追加・削除や無関係 version bump はなし。
- 差分評価: `ruviform-audio` ProbeDeck capture push bugfix、`ruviform-plugin-core`/`plugin-host` の wire v5→v9・`Plugin::buses` Option 化、`ruviform-ui` Input/CommandParams、`ruviform-ui-gpui` IMM32/native surface 変更を分類。AL は plugin host/device boundary を消費せず、UI 新 API も未消費。詳細は `evidence/al127/87a7ba8f-verification.md`。
- 検証: pin guard、`cargo fetch --locked`、analysis/service/CLI/GUI check/test、UI typecheck/test、GPUI check/lint/test/build、real-headless blocked-probe、prek が全て exit 0。`test al-analysis` は rf628 oracle を含む 151 tests pass。`build-ui-gpui` は gpui fork 経由の `al-desktop` Native Build/Link 合格。
- upstream `87a7ba8f` 自体の covering CI run はなし。ancestor `ab6dc99f` の `ci` run 45749815004・`daily-ci` run 45749786810 は success で、`ab6dc99f..87a7ba8f` は production code 差分なし。interactive real GUI/pixel/IME、macOS/Linux、cross-consumer tuple、upstream main `0fb97d1b` は Deferred/後続。

## 未検証・制約

- canonical e2e +10%/locate +17% の net 回帰(#133)は未解消。ruviform 側の `prefers_direct` 係数 8.0 が pow2 radix-4 前提で誤判定すること、Full mode で tpl≈sig の smooth 化恩恵が薄いことを ruviform Issue として起票提案。
- AL#115: decoder 記録値 `e467ccb…` vs lockfile `f3f474b…` の乖離(rev が記録される契約自体は検証済み、#104 で build.rs 導出に解消)。
- 性能比較の #98 基準との定量追跡は tools/baseline 資産の対象。
