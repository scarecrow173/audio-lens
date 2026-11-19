---
id: "EVID-svK3m4d0HkX4z3V1v9jQ"
type: "evidence"
status: "approved"
title: "検証記録: DSP/数学カーネル — 等価性・性能・計測方法"
created: "2026-11-19"
updated: "2026-11-19"
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

# 検証記録: DSP/数学カーネル

`al-math` 系の数値カーネル・プリミティブ・plan/workspace 分離に関する等価性検証と性能計測の事実記録。

## vec プリミティブ等価性(T4)

- 8 プリミティブ(dot/sum_sq/corr0/rms/scale/axpy/sub_scaled/mag_sq)の dispatch↔Scalar 等価性を `|a-b| <= 1e-12 * max(1,|a|,|b|)` で検証。サイズ行列 `0..=33`(空・1 要素・lane 端数)+1k/64k/1M、`mixed_inputs` 決定的入力(負値・混合桁)。
- `cargo test -p al-math` vec.rs 14 件 pass、`cargo test --workspace` 500 passed/0 failed。`al_locate::dsp` 公開再 export で外部消費を確認。
- criterion ベンチ `vec_primitives`(GiB/s、median): n=32 は call オーバーヘッド支配(5–13ns/call)、1M でメモリ帯域 ~20–34 GiB/s に飽和。dot 1M 641.6µs/24.4、sum_sq 254.4µs/30.7、corr0 666.1µs/23.5、sub_scaled 706.9µs/33.2、mag_sq 1.167ms/20.1。
- 置換前後比: 全行が ±5% 内か改善(最大 −10.3%: GccPhat 367.8→329.8ms)。Instant bench は 3 rep 単発で +20% 級の run 間変動あり、閾値近傍は再計測で逆転し得る。
- residual 再構成: A/B 交互計測(REPS=15)で legacy→reorg が −15〜−18%(2ch×1src)/−7〜−1%(2ch×2src)。採否ゲート(既存テスト green+同一 fixture predicate+回帰±5% 内)を満たして採用。sequential 計測では drift で偽回帰(+15〜+37%)が出たため交互計測に変更した経緯がある。

## NCC/PHAT plan 化(#52)の計測

- `bench_ncc` の全 5 パターンが −27〜−75% で改善:`bench_ncc/10x4x10` 16.99µs→4.35µs(−74.4%)、`/10x4096x10000` 51.6→37.5µs(−27.2%)、`/100x4096x10000` 518.3→365.9µs(−29.4%)、`/1x4800x8640` 52.7→37.9µs(−28.1%)、`/1x192000x480000` 4.76→1.90ms(−60.0%)。
- 計測 binary: before `b1cdcd7e4ad4c39f…`、after `9ce2ffbbbd84ac0a…`(両方 tracked source archive から再現)。
- `CorrelationPool` 効果: `bench_locate_prepared` で plan builds が呼出数に対して 1–2 に収束(呼出し数の 1/1,800–1/6,000 規模)。full_ncc 1.99→1.82ms(−8%)、band_ncc 11.6→3.8ms(−67.6%)、gcc_phat 2.06→1.71ms(−17%)、spec env 1.23→1.13ms(−8%)。
- OverlapCorr FFT 置換:`bench_overlap_corr` で窓あたり 6.5x(既定 8192/lag512、直接法 corr0 走査対比)。交差は ~L=256 付近で、dispatch 閾値は `3·(2·max_lag+1)·L < 15·N·log2(N)` の仕事量モデル。
- local_lag 自体はステージ全体で軽微(~585 窓×~1.8ms≈~1s/pair)で、支配項は STFT/CSD/FIR 系。

## 形状適応 dispatch(#71 P3)の計測

- コストモデル `L·(K+1) ≤ C·n·log2(n)` が 6 代表形状で実測最速側と一致。NccDirect plan の ncc が NccValid plan と全有効ラグ一致(argmax・tie 含む、誤差 ~1e-13 未満)。
- 実サンプル run:`locate::band_ncc` busy −84%、`full_ncc` −90%。

## BandFilter streaming(#71 P2)の計測

- counting allocator テストで帯域同時生存が 2 信号以内(streaming max_live 16.8MB vs batch 276.8MB、約 16 分の 1)。実サンプル run で reference ステージ中 peak WS 2186MB→937MB(−57%)、pair wall は同等。
- `#[inline(always)]` 化による修正効果:`bandpass_many` 26 帯域 8.8s→~2.0s、`analyze_pair` 52s→~23s。

## 等価性で注意が必要な数値差分

- split re/im の `cmul` 再合成は pulp `mul_c64s` との FMA 融合差で ~1ulp(性能優先で意図的採用)。
- `normalize_gain` の `v/rms` → `v*r.recip()` は ulp 級差(許容済み)。
- envelope NCC の `corr0` 化は周期入力で全 lag 厳密同点(≡1.0)になり argmax が lag 0 に倒れる —— 残存許容として逐次式を維持していた(のち曖昧性構造検出により解消、`locate-verification.md` 参照)。
- residual 累算順の変更(一括 dot→逐次 sub_scaled)で `ResidualEnergy.after` 等が ~ulp 揺れ(ADR 許容)。

## 未検証・制約

- bench の Instant 計測は 3 rep 単発で +20% 級の run 間変動、閾値近傍は将来再計測で逆転し得る。
- `bench_residual` 代表形状は 2ch×{1,2}src で多 ch source では cap 再走査が線形増。
- n≥16384 で使用する thread_local workspace の out-of-place 経路は、caller 共有バッファへの書き戻しが必要。
- criterion の smoke bench は公開 dispatch 経路の最小サイズ(8192/1024)で、大規模形状の代表ではない。
