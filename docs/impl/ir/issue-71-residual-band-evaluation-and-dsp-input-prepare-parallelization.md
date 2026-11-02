---
id: "IMPL-034QFVFaeVh3dJyR8EkmOP"
type: "impl"
status: "completed"
title: "Issue #71 residual band evaluation and DSP input prepare parallelization"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "code"
        path: "crates/al-locate/src/pool.rs"
      - type: "code"
        path: "crates/al-locate/src/prepared.rs"
      - type: "code"
        path: "crates/al-locate/src/stage.rs"
      - type: "doc"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-71-band-gain-shared-spectrum.md"
    - "docs/impl/ir/perf-reeval-2026-10.md"
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
    adopted:
      - "(input, channel) 粒度の DSP 入力準備一括並列化 prepare_many(純粋計算の pool 化 + spec 順の逐次規則で挿入・計測)"
    rejected:
      - "locate band_ncc の一段 NCC/帯域合成への再構成(corr0 逐次蓄積と丸め順が変わり bit-exact を満たせない)"
      - "band_gain 窓 NCC のローカル相互相関+prefix-sum バッチ化(window_energy の累積起点が変わり last-ulp 差分)"
      - "OverlapCorrPlan の流用(等長窓・縮小 overlap 正規化で意味論が違う)"
      - "コスト順(LPT 系)タスク再順序化(静的順序と実コストのずれで逆効果を確認)"
      - "無音窓の早期打切り(9 run 全てで silence 窓 0 件、発火しない)"
---

# Issue #71 residual band evaluation and DSP input prepare parallelization

## Summary

Issue #71 の残件調査として (a) locate `band_ncc` の帯域フィルタ+
帯域ごと全長 NCC 構造、(b) `band_gain` の窓 NCC ループ(~16.9s/pair)
を再評価し、両者とも現行が既に bit-exact を保ったまま取れる最適な
演算経路であることを計測とコード対照で確認した。(a) について前提と
なっていた ~174s busy の見積もりは ruviform 移行前の古い値で、現行
postspec 計測では 24.3s/16 calls に縮退済みだった。調査で見つけた
bit-exact を保てる唯一の実質的な余地として、DSP 入力準備
(`InputCache` のフィルタ+間引き)を (input, channel) 粒度で
ワーカープールに並列化する `prepare_many` を実装し、capture+sources
一括・residual チャンネル一括・children 一括の 3 箇所でバッチ化した。
2 冪変換長の過剰パディング(~44〜47%)という ruviform 側の構造的
余地は上流 issue として scarecrow173/ruviform#255 に起票した。

## Context

- Issue #71: 帯域相関のコスト削減。PR #129 までに `band_gain` の
  共有 spectrum(BandSpectrum/BandScratch)で ~1.94x・e2e
  198.3→175.0s・golden bit-exact を達成済み
  ([issue-71-band-gain-shared-spectrum](issue-71-band-gain-shared-spectrum.md))。
- 本記録の対象は残件 2 件の評価と、計測インパクトとリスクで選んだ
  実装範囲の記録。事前計測は
  [perf-reeval-2026-10](perf-reeval-2026-10/) の post71/postspec
  キャプチャと `bench_reeval` の形状別計測。

## Implementation

### Changes

- `crates/al-locate/src/pool.rs`: `stage.rs` にあった `run_pool`
  (ワーカープール・順序保証スロット収集)をクレート共有の
  `pub(crate)` 関数として移設。振る舞いは同一。
- `crates/al-locate/src/prepared.rs`:
  - `PrepareSpec`(id/channel/rate/version/samples の組)を追加。
  - `prepare_with_bands` の計算部を純粋関数 `filter_channel` と
    キー構築 `input_key` に分離(内容は不変)。
  - `InputCache::prepare_many` を追加。キー確定→miss 収集
    (重複キーは pending で共有)→`run_pool` で miss 分を並列計算
    →spec 順に逐次 `prepare` と同じ規則で hit/miss を計上して
    挿入・返却。`locate::prepare_channels` span を付与。
- `crates/al-locate/src/stage.rs`:
  - `prepare_channels` を `prepare_many` 経由に変更(署名不変)。
  - capture ごとのループで、capture+有効 source 全チャンネルの
    spec を一括 `prepare_many` に渡し、既存の `prepare_channels`
    呼出はキャッシュ hit で解決する。mark で弾かれる source
    (チャンネル無し/全空)は従来どおり対象外。
  - residual 経路: `residual_cache` のチャンネル準備を
    `prepare_many` に、全 children×チャンネルを 1 バッチの
    `prepare_many` に変更し、(child, channel) 順で regroup。
- `crates/al-locate/src/prepared.rs` tests: バッチ一括と逐次
  `prepare` の帯域/coarse 内容一致・統計一致・バッチ内重複
  dedup・再呼出 all-hit の回帰テストを追加。逐次 vs 一括の壁時計を
  比較する `#[ignore]` 計測テスト
  `bench_prepare_many_parallel_vs_sequential` も併設した。

### Decisions During Implementation

- **(a) locate `band_ncc` は変更不要と判断**: 現行は
  `bands::bandpass_many`(共有 forward FFT+帯域ごと逆 FFT)済みで、
  prepared 経路では `PreparedChannel` キャッシュが再フィルタも
  排除している。帯域ごとの NCC は `CorrelationPool` 経由の
  `CorrelationPlan::new_ncc` で、full-rate 形状(2.4M/241 lags)は
  コストモデルが直算(`NccDirect`: axpy 蓄積・prefix-sum 分母・
  template エネルギー外だし)を選ぶ最適形。発行時点の ~174s busy
  の前提は移行前の古い値で、postspec 計測の現値は 24.3s/16 calls。
  帯域合成による一段 NCC 等の再構成は corr0 の逐次蓄積と丸め順を
  変え得るため bit-exact を満たせず採用しない。
- **(b) 窓 NCC ループも変更不要と判断**: 窓形状(4,800/8,640)は
  `new_ncc` の縮退 FFT(16,384)経路で 0.35ms/窓、直算は 15.77ms
  (~45x 遅)。ローカル相互相関+prefix-sum バッチ化は分母の
  window_energy の累積起点が変わり last-ulp 差分を生じるため
  bit-exact を満たせない。`OverlapCorrPlan` は等長窓・縮小する
  overlap 正規化で `band_gain` の全窓 NCC と意味論が違い流用不可。
- **実装対象として DSP 入力準備の並列化を採用**: canonical-locate
  計測で `prepare_channels` が 3.96s busy/6 calls(逐次)に対し、
  (input, channel) 粒度で一括化すると全 miss がワーカープールに
  載る。計算は純粋関数で結果・統計・dedup 規則が逐次呼出と同一の
  ため、制約(bit-exact・証拠契約維持)を満たしたまま取れる最大の
  残存レバーと判断した。
- **非2冪変換長は上流へ**: band_gain 窓形状は signal 8,640 に対し
  変換長 16,384(最小の ~47% 過剰)、アンカー full 形状は
  2,400,240 に対し 4,194,304(~30〜40% 過剰)。radix-3/5 が要る
  FFT 基盤の拡張であり AudioLens 側の恒久重複は作らないため
  ruviform#255 として起票(効果は変換長 ~30〜47% 削減、consumer
  は golden 再生成が必要)。

## Related Experiments

### Adopted

なし(実験ログは作成していない。選択肢の検討は上記 Decisions に記録)。

### Rejected

- コスト順(LPT 系)のメソッド/アンカータスク再順序化: 静的順序と
  実コストがずれると逆効果になる合成ワークロードを確認し、canonical
  改善も再現しないため不採用。
- 無音窓の早期打切り: 9 本の golden run 全てで silence 窓が 0 件
  (実データで発火しない経路)のため YAGNI として棄却。

## Validation

- `mise run` 系: `fmt` / `lint` / `test`(workspace 全体)/
  `verify-toolchain` / `check-cli-headless` 全て pass
  (biome の al-gui style 警告は既存・非ブロッキング)。
- `cargo test -p al-locate`: 58 lib tests(新規 `prepare_many`
  等価テスト含む)+ integration(golden 16、locate 12、prepared 7、
  argmax 1、ruviform_deps 2)全て pass。
- `cargo clippy -p al-locate --all-targets -- -D warnings`: pass。
- Golden 比較(`tools/baseline/compare.mjs`): canonical / small /
  aa / resume / variant-desync の 5 run 全て **0 diff**(bit-exact)。
  コミット済み baseline は ruviform provenance key 導入前の stale
  状態のため、同一セッションで pre-change バイナリと post-change
  バイナリの capture を取り直して対比した(provenance drift 由来の
  偽差分を排除するため)。
- 制御計測(`bench_prepare_many_parallel_vs_sequential`、release):
  6 spec×2M サンプルで sequential 2,341ms → `prepare_many`
  1,120ms、**2.09x**(メモリ帯域律速でワーカー数未満に頭打ち)。
- 本番計測(`tools/perf-reeval/capture.mjs`):
  - `locate::prepare_channels`(canonical): 5.35s busy/10 calls。
    実計算バッチは 4-spec で 1.34〜1.49s、2-spec で 1.22〜1.30s、
    残りは all-hit で µs 級。バッチ壁時計は spec 合計ではなく
    最遅 spec+pool 起動に漸近する構造になった。
  - 同一セッションの canonical e2e: 153.3s → 143.6s、locate stage
    95.5s → 94.1s。postspec 計測(175.0s/101.6s)は別セッション・
    別負荷条件のため直接差分には使わない。
  - small e2e: pre 91.7s → preppar 78.4s、locate 79.5s → 68.2s。
  - 留意: baseline 用 post-capture の一部は `cargo test` 同時実行下で
    採取されており、同バイナリでも e2e が最大 ±40s 振れる計測雑音を
    確認した(resume pre 180.6s → post 140.7s は本変更では説明
    できない負荷由来の差)。内容比較(golden 0 diff)と span busy /
    制御ベンチが本変更の確実な根拠で、e2e/locate の差分は方向の
    参考値とする。

## Risks

- `prepare_many` は miss 分のみ並列計算する。hit 済み spec や
  空バッチは従来どおり逐次で、ワーカー起動コストは miss 件数に
  比例する(小規模バッチでは効果が薄い)。
- `locate::prepare_channels` span を新設した。perf-reeval 系の
  ログ収集では新規 span 名が増えるのみで、成果物には出ない。
- 並列計算中の panic(bandpass_many の内部 expect)は scope 経由で
  伝播する。逐次版も同じ expect を持つため挙動は同型。

## Follow-ups

- ruviform#255: 非2冪変換長対応が入れば band_gain 窓 NCC と
  アンカー full/widen で ~30〜47% の変換量削減が見込める。
  採用時は AudioLens 側 golden の再生成が必要。
- locate の残存支配コストは `gcc_phat`(postspec 39.6s busy)と
  アンカー経路(~72.5s busy)で、`band_ncc` 自体ではない。
  `spectral_envelope`(22.5s)も同量級。issue #71 の枠を超える
  別件として扱う。
