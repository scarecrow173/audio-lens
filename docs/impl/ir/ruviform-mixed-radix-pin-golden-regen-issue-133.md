---
id: "IMPL-sVG_mgkzHmo21LucC9WoX"
type: "impl"
status: "completed"
title: "Ruviform mixed-radix FFT pin bump + golden regeneration + perf measurement (Issue #133)"
created: "2026-11-03"
updated: "2026-11-03"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "doc"
        path: "docs/impl/ir/ruviform-mixed-radix-pin-golden-regen-issue-133.md"
      - type: "data"
        path: "docs/impl/ir/perf-reeval-2026-10/bench-reeval-issue133.txt"
      - type: "data"
        path: "docs/impl/ir/perf-reeval-2026-10/golden-diff-133.json"
      - type: "data"
        path: "docs/impl/ir/perf-reeval-2026-10/canonical-pre133.json"
      - type: "data"
        path: "docs/impl/ir/perf-reeval-2026-10/canonical-pre133b.json"
      - type: "data"
        path: "docs/impl/ir/perf-reeval-2026-10/canonical-post133.json"
      - type: "data"
        path: "docs/impl/ir/perf-reeval-2026-10/small-pre133.json"
      - type: "data"
        path: "docs/impl/ir/perf-reeval-2026-10/small-pre133b.json"
      - type: "data"
        path: "docs/impl/ir/perf-reeval-2026-10/small-post133.json"
    modified:
      - type: "code"
        path: "Cargo.toml"
      - type: "code"
        path: "Cargo.lock"
      - type: "code"
        path: "crates/al-locate/tests/locate.rs"
      - type: "baseline"
        path: "baseline/runs/canonical"
      - type: "baseline"
        path: "baseline/runs/small"
      - type: "baseline"
        path: "baseline/runs/aa"
      - type: "baseline"
        path: "baseline/runs/resume"
      - type: "baseline"
        path: "baseline/runs/variant-desync"
      - type: "baseline"
        path: "baseline/environment.json"
      - type: "doc"
        path: "docs/impl/ir/README.md"
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
  related:
    - "docs/designs/ruviform-dependency-ci.md"
    - "docs/impl/ir/ruviform-rev-bump-consumer-regression-verification-issue-127.md"
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

# Ruviform mixed-radix FFT pin bump + golden regeneration + perf measurement (Issue #133)

## Summary

Ruviform pin を `9d267fd` から `258df5a`(PR #256 merge、ruviform
#255 mixed-radix FFT 採用)へ更新した。CorrelationPlan の変換長が
`nextpow2` から {2,3,5}-smooth 高速長選択へ変わり、tpl≪sig の
NCC 形状で変換長が -42〜-49% 削減された。golden 5 run で差分は
rev スタンプと変換長・計算順序由来の浮動小数/整数候補差のみで、
構造差・最終決定差は 0。pre-release 方針に従い baseline を
再生成した。効果実測では変換長削減に反して AudioLens e2e は
改善せず、canonical で **+10%(locate +17%)の回帰**、small で
±2% 程度の誤差範囲だった。原因は tpl≈sig メソッド形状(実経路の
支配コスト)で `prefers_direct` 誤判定による direct→FFT 反転と
Full mode の変換長効果不足(mixed-radix の per-element コスト増
> 長縮小効果)で、ruviform 側課題として報告する。PR #130
(prepare_many 並列化)は pin の影響を受けず非回帰を確認した。

## Context

Issue #133: ruviform PR #256(merge `258df5a`)で CorrelationPlan
の変換長選択が {2,3,5,7}-smooth 系へ拡張された(選択は
{2,3,5}-smooth 偶数、radix-7 は FFT 層で受理するが相関では
計測上遅いため非選択)。AudioLens 側の pin 更新、golden 再検証・
再生成、変換長削減と locate/e2e 時間の効果実測、PR #130
(prepare_many 並列化)との複合効果確認が目的。

旧 pin `9d267fd` と新 pin `258df5a` の間には ruviform Issue
#249(EOF 修正)系の差分 1 件も含まれるが、AudioLens 消費面の
API 契約は不変。

## Implementation

### Changes

- `Cargo.toml` `[workspace.dependencies]` の ruviform 14 entry の
  `rev` を `9d267fd…` → `258df5ae…` に一括置換(規約どおり全
  entry 同一 SHA)
- `Cargo.lock`: `cargo update -p ruviform-media-core` で再解決。
  15 ruviform source が単一 `?rev=258df5ae…#258df5ae…` に統一、
  旧 rev 参照 0 件を確認
- `crates/al-locate/tests/locate.rs`:
  `long_source_persists_coarse_proposals_and_global_evidence` の
  candidates 比較を `serde_json::to_value(生値)` との直接比較から、
  永続化と同じテキスト経路(serialize→parse)を通す比較へ修正
  (後述「テスト失敗の原因分析」)
- `baseline/runs/{canonical,small,aa,resume,variant-desync}`:
  post-pin で再 capture し golden を再生成。`determinism.json`
  (aa/resume) も再生成済み。`baseline/environment.json` に再生成
  由来を `commit_note` で記録
- AudioLens ソースの call-site 変更: **なし**(全消費 API は
  signature・契約不変)

### Resolved revision

- before: `9d267fdf7aa2b3f1cd250672f32b54813ada8e1d`
- after:  `258df5ae255f812f54461eb1aef30fce608e8c6d`
  (`git ls-remote` / ローカル fetch の `refs/heads/main` HEAD と一致)
- 主要差分: `b5044b2` (numeric: mixed-radix FFT for correlation plans、
  radix-2/3/4/5 専用段 + generic radix-7、Full mode bound を
  `S+2L-2` へ修正)、ruviform Issue #249 系 EOF 修正

### Impact analysis(AudioLens 消費面)

- `CorrelationPlan` 消費: `al-locate/src/pool.rs`(PoolMode::Ncc=
  `new_ncc`、PoolMode::Phat=`new`)、`al-analysis/src/cheap.rs`、
  `al-analysis/src/band_gain.rs`。公開 API・返却型は不変、
  call-site 修正不要
- 変換長の振る舞い変化:
  - `new_ncc`: `n = 最小 {2,3,5}-smooth 偶数 ≥ signal_len`
    (旧: `nextpow2(signal_len)`)
  - `new`(Full): `n = smooth ≥ signal_len + 2·template_len - 2`
    (旧: `nextpow2(S+L-1)`。bound 修正で旧実装は trailing lag の
    必要長が不足していた — pow2 オーバーシュートで隠れていた)
  - `prefers_direct`(k ≤ 2048 かつ `T·k ≤ 8·n·log2 n`): n が
    縮小したことで閾値が下がり、tpl≈sig 形状で direct→FFT 反転
    (後述の測定で実害を確認)

## Golden 差分分析(pre-pin vs post-pin、同一セッション)

`tools/baseline/compare.mjs` で 5 run を比較し、差分を分類した。
要約 JSON: `docs/impl/ir/perf-reeval-2026-10/golden-diff-133.json`

| run | total | rev-stamp | non-rev | tolerance | int | structural | decision-path |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| canonical | 1257 | 998 | 259 | 242 | 17 | 0 | 0 |
| small | 1741 | 1205 | 536 | 516 | 20 | 0 | 0 |
| aa | 1257 | 998 | 259 | 242 | 17 | 0 | 0 |
| resume | 1257 | 998 | 259 | 242 | 17 | 0 | 0 |
| variant-desync | 1050 | 786 | 264 | 223 | 41 | 0 | 0 |

- rev-stamp 差分: `ruviform_rev` が `9d267fd…`→`258df5ae…` に更新
  される想定内の provenance 変更(decoder/analysis metadata)
- tolerance 差分: `score`/`peak_to_background`/`peak_ratio_12`/
  `target`/`scalar`/`ref.phat_peak` 等。変換長変更による FFT 加算順・
  PHAT bin グリッド変更に起因。canonical では `peak_to_background`
  最大 ~6.4%、`score` 最大 ~199%(低スコア領域の絶対差は微小)
- int 差分: `peak_width`/candidate 順位/`position` の境界変動
  (曲線の FP 微差が極値選択を反転)
- 構造差(artifact 配置・record 形状)と最終決定差(offset/lag/
  time_map/accepted)は全 run で 0
- **決定性**: post-pin 同士で `aa` vs `canonical` = 0 diff、
  `resume` vs `canonical` = 0 diff。同一 pin での再現性を確認

差分は全て変換長・計算経路変更由来であり、pre-release 方針により
互換層を設けず baseline を再生成した。旧 baseline は git 履歴に
残るため `*-pre133` 一時コピーは破棄(比較要約は上記 JSON に保存)。

## 効果実測

### 変換長削減(bench_reeval、同一セッション pre/post)

生値: `docs/impl/ir/perf-reeval-2026-10/bench-reeval-issue133.txt`

| shape | tpl | signal | fft 旧 | fft 新 | 削減率 | ncc 旧 | ncc 新 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| narrow_lo | 512 | 64,000 | 65,536 | 64,000 | -2.3% | 1.68ms | 1.95ms |
| narrow_hi | 512 | 132,000 | 262,144 | 135,000 | -48.5% | 7.40ms | 5.69ms |
| widen_mid | 512 | 580,000 | 1,048,576 | 583,200 | -44.4% | 45.51ms | 32.07ms |
| widen_hi | 512 | 1,050,000 | 2,097,152 | 1,062,882 | -49.3% | 101.49ms | 76.19ms |
| full | 512 | 2,400,240 | 4,194,304 | 2,430,000 | -42.1% | 231.97ms | 223.76ms |
| band_win | 4,800 | 8,640 | 16,384 | 8,640 | -47.3% | 0.35ms | 0.22ms |
| method | 2,400,000 | 2,400,240 | 0 (direct) | 2,430,000 | n/a | 134.38ms | 218.61ms |

- anchor full NCC(issue 指定形状): 4,194,304 → 2,430,000
  (**-42.1%**)、ncc 時間 -3.5%
- band_gain window: 16,384 → 8,640 (-47.3%)、ncc -37%
- widen/narrow 系: -44〜-49%、ncc -23〜-30%
- **method 形状回帰**: tpl≈sig(k=241)は旧 direct(134.38ms)→新
  FFT(218.61ms、**+63%**)へ反転 — `prefers_direct` 誤判定

### e2e / locate(perf-reeval capture、同一セッション)

pre/post 各 2 回計測。初回 pre 計測(`*-pre133.json`)は並行負荷で
異常に膨らんでいたため(span busy/call が歴代記録の 3-6 倍)、
アイドル状態で `*-pre133b.json` を再取得して採用。post 計測は
アイドル状態で取得。歴代記録(canonical post54/post71 =
141-157s、small post71 = 84.6s)とも整合。

| run | stage | pre-pin(採用値) | post-pin | 差 |
| --- | --- | ---: | ---: | ---: |
| canonical | locate | 82,378ms | 96,472ms | **+17.1% 回帰** |
| canonical | e2e | 129,288ms | 142,508ms | **+10.2% 回帰** |
| small | locate | 69,792ms | 68,389ms | -2.0%(誤差級) |
| small | e2e | 79,951ms | 78,678ms | -1.6%(誤差級) |

- canonical の `window_anchors_in` 2 件(514 windows、range
  2,400,240、各 ~93-94 full_ncc): 31,179+34,488ms → 29,879+32,489ms
  (**-4〜-6%** の改善)— 変換長半減の効果は実在するが小さい
- canonical の locate 主律は full-rate メソッド呼出(tpl≈sig)で、
  そちらの悪化(下表)が anchor 改善を上回り net 回帰
- 初回計測 `canonical-pre133.json`(208.2s)/`small-pre133.json`
  (223.1s)はいずれもコンテンションで膨らんでおり、初回比較で
  「canonical -32%」と読めるのは計測誤差だった — 証跡として
  両ファイルは残す

### メソッド呼出し(tpl≈sig)の変化 — canonical span busy/call

| method | busy/call 旧(歴代 360-583ms 整合) | busy/call 新 | 備考 |
| --- | ---: | ---: | --- |
| full_ncc | ~494ms | ~2,289ms | direct→FFT 反転(4.6x) |
| band_ncc | ~1,803ms | ~6,344ms | 同上(3.5x、帯域各 NCC も同形状) |
| gcc_phat | ~2,171ms | ~5,544ms | Full mode n: 8,388,608→7,290,000 (-13%)で per-element 増分が支配(2.6x) |
| spectral_envelope | ~1,183ms | ~1,860ms | 別 FFT 経路も低速化(1.6x) |
| prepare_channels | ~447ms | ~472ms | ほぼ不変(誤差級) |

busy は並列ワーカー下の実行時間でコンテンションを含む概算値。
隔離ベンチ(bench_reeval `method`)でも direct 134ms→FFT 219ms
(+63%) を確認しており、実経路では帯域幅競合でさらに増幅
される。逆に anchor NCC(tpl≪sig)は改善するが、canonical の
locate に占める割合が小さい。

### PR #130(prepare_many)との複合効果

- `prepare_channels` busy/call は pre/post でほぼ不変
  (canonical 447→472ms、small 215→227ms)。並列化の効果は新 pin
  で維持され、pin との相互作用による回帰は観測されない
- e2e の複合判定: canonical +10% は pin 起因のメソッド呼出
  悪化が主因で、prepare_many 並列化が原因ではない
  (prepare_channels に異常なし)

## テスト失敗の原因分析(修正込み)

`long_source_persists_coarse_proposals_and_global_evidence` が
post-pin で `score` 1-ULP 差により失敗した。調査の結果、原因は
製品・FFT ではなく **テストの比較経路** にあった:

- 永続化 JSON には `0.9491537490084663`(正しい f64 の最短表現)が
  書かれている
- しかし `serde_json::from_str` の既定パーサ(float_roundtrip
  無効)は同テキストを `0.9491537490084664` に近似丸めする
  (実測: `str::parse` = ...663 vs `serde_json::from_str` = ...664)
- 比較右辺は `serde_json::to_value(生 f64)` で正確値を保持 →
  パース済み left と生値 right で 1-ULP 差が出る
- stage 経路と直接呼出の `corr` 曲線は bit 完全一致を dump で検証済み
  (第一パス)。FFT・pool・prepare に起因する差分ではない

修正: 比較を `serde_json::to_string(生値) → from_str` の同一
テキスト経路に揃え、永続化と同じパース誤差を両辺に適用した。
旧 pin で合格だったのは、当時の値が誤丸め対象外だったため。

`serde_json = { features = ["float_roundtrip"] }` 有効化でこの
不変条件(浮動小数の正確な往復)をパーサ層で回復できるが、
依存 feature 変更は範囲外のため未実施 — 別途提案する価値あり。

## 検出した残課題(ruviform 側、起票候補)

AudioLens e2e で **canonical +10% の net 回帰**を観測しており、
原因は ruviform 側の変換長/mode 選択に起因する。pin は Issue
#133 の指示どおり採用するが、以下は ruviform Issue 起票を
推奨する実害(コーディネーター経由で起票判断を依頼):

1. **`prefers_direct` コストモデル誤判定**: `T·k ≤ 8·n·log2 n`
   の係数 8.0 は pow2 radix-4 前提。mixed-radix の per-element
   コスト増を織り込まないため、n 縮小で閾値が下がり本形状
   (tpl≈sig、k≤2048 小ラグ)で direct→FFT に反転。隔離計測
   +63%、実経路 busy/call で 3.5-4.6x 低速化。radix 経路の
   実効コストで校正するか、該当形状で direct を維持する調整が必要
2. **Full mode(gcc_phat 経路)の変換長効果不足**:
   `(2_400_000, 2_400_240)` で n は 8,388,608→7,290,000(-13%)に
   留まり、per-element コスト増で実時間は増加(busy/call
   2.6x)。tpl≈sig では Full mode の bound `S+2L-2` が大きく
   smooth 化の恩恵が薄い
3. AudioLens への net 影響は canonical e2e +10%/locate +17%、
   small ±2%。tpl≪sig NCC(anchor 経路)では改善しており、
   負け側を直せば全体で効く構成

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

実施済み(全て本 worktree・新 pin で実測):

- `git` で `refs/heads/main` = `258df5ae…` を pin と照合
- `mise exec -- cargo fetch --locked`: pass
- `mise exec -- cargo tree`: ruviform 15 source が単一 rev、旧 rev
  参照 0 件
- `mise run fmt`: pass
- `mise run check`: pass
- `mise run lint`: pass(biome は既存 JS/TSX warning/info のみ、
  新規指摘なし。clippy `-D warnings` clean)
- `mise run test`: pass、全 workspace テスト緑(修正した
  `long_source_persists…` を含む)
- golden: 5 run を post-pin で再 capture、pre-pin との compare
  分類(上表)、post-pin 間の determinism(aa/resume vs canonical)
  = 0 diff
- bench: `bench_reeval` pre/post 同一セッション計測(上表)
- perf-reeval: canonical/small を pre/post で capture(small pre は
  アイドル時の再計測を採用)
- pre-pin バイナリは `samples/_runs/al-pre133.exe` として保持
  (gitignore 対象のスクラッチ領域、検証用)

未実施:

- `mise run verify-ci`(CI profile 一式): dev profile の
  fmt/check/lint/test は実施済み。ci profile は未実行
- ruviform 側 `issue255_perf` example の本環境計測: al-locate 側
  bench_reeval で代表形状の plan 実測を代替として取得済み
