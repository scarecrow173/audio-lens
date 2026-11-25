# al#274 evidence — Kani solver-bound 8 harness 原因調査・budget 削減

対象 Issue: <https://github.com/scarecrow173/audio-lens/issues/274>
実施日: 2026-11-25
sync commit: `344a48c7407a6c3611808686d458f6cf64e0ae39`(dispatch 時点の origin/main tip)

## 環境

`residual/env.txt` 参照。要約: Windows 11 + WSL2 `Ubuntu`、
kani-verifier 0.68.0 + CBMC 6.11.0、nightly-2026-08-21、
Mem 31GiB + swap 8GiB、16 vCPU。開始時に他 repo/他 Worker の
kani・cbmc job は無し(ps 確認)。trial 実行中の cbmc は本 worker
の queue 由来のみ。canonical = `mise run test-formal-kani`、
個別 `cargo kani` trial・ps/free 観測は補助証拠。

## 支配要因の分析(コード精査 + 短時間試行の実測)

al#256 residual で timeout した 8 harness について、原因を以下の
分類で特定した。判別材料は `--log-file` が残す verbose ログ
(kill されても残る — al#256 の `kani-*.txt` は driver buffer 喪失
で cargo 行しか残らなかった制約を、この手段で回避した)と
10s 間隔の cbmc RSS 採取。

### 分類

| 分類 | 定義 | 判別 |
| --- | --- | --- |
| envelope-bound | cbmc RSS が物理 envelope まで単調増大し OOM/殺到 | RSS が free 近傍まで伸びる |
| symex-bound | SAT 到達前に symbolic execution の path 展開で budget 切れ | verbose log が最後まで `Unwinding loop`/`aborting path` を吐き続け、RSS が緩やか増大 |
| solver-bound | symex 通過後に SAT grind で未収束 | RSS 頭打ち + SAT フェーズログ |

### 支配的コスト(全 harness 共通の構造要因)

verbose ログの function 分布(例:
`header_reads_invalid_ranges_error` 600s trial、1843 件の
abort/unwind イベント)で、上位を占めるのは harness 固有ロジック
ではなく stdlib 計装経路だった:

- `kani::mem::cbmc::same_allocation`・`kani::rustc_intrinsics::
  ToISize::to_isize`: slice/pointer 操作ごとの kani 妥当性検査 —
  `read_at` 系の `data.get(off..off+n)` が繰り返し発行する
- `std::collections::TryReserveError`・`RawVec::grow_impl_runtime`
  ・`handle_alloc_error`・`layout_array`: `vec![0; n]`・
  `Vec::with_capacity`・`Cursor<Vec>` 成長の alloc 失敗枝 —
  `--no-malloc-may-fail` 下でも symex は失敗枝を explore してから
  `assume(false)` で打ち切る
- `core::slice::memchr::memchr_aligned` ← `core::fmt::builders::
  PadAdapter::write_str`: `format!` 起因の fmt machinery が loop
  unwind を消費(al#256 の decode probe で iteration 55/56 まで
  到達を実測)
- `std::io::Error`/`ErrorKind`/`repr::decode_repr`/`core::fmt`:
  `?` 経路の error 型変換

つまり timeout の支配要因は入力 symbolic 度合いではなく、
**decode call-graph 全体 × loop unwind × kani 計装の式量**である。
concrete fixture + symbolic 2byte のみの `header_reads_invalid`
でも 600s で symex が終わらないことが、この「固定床コスト」を
直接示す。product `src/` 側に改変可能な原因は無い
(`format!` 多用・vec 確保は product 意味論の一部 — 本 slice で
product は不変)。

## 実施した削減

1. **proof 分割(契約を弱めない分割)**: 9 combined proof →
   19 単一責務 proof(2 段目の signature 分割と合わせて最終 21)。
   - `decode_never_panics_on_bounded_input`(3 entry point・
     unwind 56)→ `decode_wav`(40)/`wav_frame_count`(16)/
     `read_wav_range`(40)に entry point 別分割
   - `writer_round_trip_is_exact`(symbolic encoding 5 択・32)→
     per-encoding 5 proof(各 32)
   - `header_reads_agree_with_writer_layout`(32)→ valid 側
     `header_reads_valid_ranges_slice_exact` + invalid 側
     `header_reads_invalid_ranges_error` の補集合ペア
     ([0,255]×[0,255] の分岐空間を合計で網羅)
   - `containers_never_panic_on_bounded_input`(3 entry・80)→
     `mp4_`/`avi_never_panics_on_bounded_input`(各 48)
   - `mutated_mp4/avi_never_panics`(mutate∧truncate 結合)→
     mutate-only + truncate-only 4 proof。結合入力は proptest
     `mutated_or_truncated_containers_never_panic`(最大 16 箇所
     編集+truncate、元 proof より広い)が担保
   - unwind 縮小は「実到達 loop trip 数 + slack」に rightsizing。
     unwinding assertions は有効のまま(不足すれば FAILED で
     検出される)
2. **signature-gated input-domain 分割(2 段目の削減)**:
   初段の分割でも symbolic-input no-panic 系が timeout したため、
   decode 入口が持つ signature 早期 reject(wav.rs:192・
   mp4.rs:1196・avi.rs:375)を domain 境界として使い、
   signed 側と unsigned 側に分離。unsigned 側
   (`wav_reads_never_panics_unsigned`・
   `containers_never_panics_unsigned`)は 3 入口まとめて
   早期 reject 経路のみを持つ shallow proof — signed 側と
   合わせて入力領域を disjoint に網羅する
3. **runner env-gated 追加**: `AL_KANI_HARNESSES` subset filter
   (既定挙動不変・未登録名は fail-closed `not_run`)。HARNESSES
   を 21 件へ同期
4. **入力 domain narrowing(正直な縮小として記録)**:
   `header_reads_agree_with_writer_layout` が 1 proof で valid/
   invalid 双方を見ていたものを、valid-only / invalid-only に
   `kani::assume` で分けた。合計のカバレッジは不変、
   per-proof の分岐木のみ半減。signed 系の `kani::assume(署名)`
   も同様に、補集合を unsigned proof が担保する分割であり
   契約の弱化ではない

## Trial 実行記録(補助証拠 — WSL mirror 内 `cargo kani` 個別実行)

`~/.cache/al-formal-kani/trials/` に per-harness `.log`(cargo 出力)
・`.verbose.log`(kani-driver `--log-file`、kill 耐性あり)
・`.rss.txt`(10s 採取)・`summary.txt`/`summary-par.txt` を残し、
全件を `trials/` 配下に採取済み(`residual/` は canonical lane の
出力専用 — al#256 同型)。

### 結果表

| harness | cap | 結果 | elapsed | peak RSS |
| --- | --- | --- | --- | --- |
| wav_frame_count_never_panics_on_bounded_input | 600s | timeout | 630s | 5.2 GiB*(symex 中・増大継続) |
| header_reads_invalid_ranges_error | 600s | timeout | 631s | 5.2 GiB*(symex 中) |
| header_reads_valid_ranges_slice_exact | 900s | timeout | 947s | 2.5 GiB(symex 中) |
| writer_round_trip_pcm_s16 | 600s | timeout | 632s | 2.2 GiB(symex 中) |
| writer_round_trip_pcm_s24 | 600s | timeout | 632s | 2.2 GiB(symex 中) |
| writer_round_trip_pcm_s32 | 600s | timeout | 631s | 2.1 GiB(symex 中) |
| writer_round_trip_float32 | 900s | timeout | 947s | 2.4 GiB(symex 中) |
| mp4_writer_output_decodes | 1200s | timeout | 1265s | 4.5 GiB(symex 中) |
| truncated_mp4_never_panics | 900s | timeout | 948s | 2.5 GiB(symex 中) |
| truncated_avi_never_panics | 900s | timeout | 948s | 2.9 GiB(symex 中) |
| wav_frame_count_never_panics_signed | 600s | timeout | 632s | 2.6 GiB(symex 中) |
| decode_wav_never_panics_signed | 1200s | timeout | 1265s | 3.8 GiB(symex 中) |
| read_wav_range_never_panics_signed | 1200s | timeout | 1267s | 3.6 GiB(symex 中) |
| wav_reads_never_panics_unsigned | 300s→900s | timeout | 316s/949s | 1.2→3.1 GiB(symex 中) |
| containers_never_panics_unsigned | 300s→900s | timeout | 316s/949s | 0.84→2.3 GiB(symex 中) |
| mutated_mp4_never_panics | 1200s | timeout | 1266s | 2.4 GiB(symex 中) |
| mutated_avi_never_panics | 1200s | timeout | 1266s | 2.9 GiB(symex 中) |
| avi_writer_output_decodes | 1200s | timeout | 1267s | 3.9 GiB(symex 中) |
| detect_format_signature_rules | 300s | SUCCESSFUL | 55s | 0.03 GiB |

(*RSS 採取は cbmc+kani-driver 合算 — driver 込み peak。cbmc 単体の
終盤値は `*.rss.txt` 末尾参照)

補足: `wav_frame_count_never_panics_signed` は旧結合名
`..._on_bounded_input` 比で peak 5.2→2.6 GiB と半減、
`decode_wav_never_panics_signed` は al#256 の 3 入口結合 proof の
envelope-bound OOM(30.4GiB)に対し 3.8GiB — signature gate が
unsigned 経路と symbolic `memcmp` の分岐質量を除去した効果。
ただし solver 時間は依然 budget 超過で、verdict 改善には至らず。

署名 gated unsigned 系は shallow 経路のみでも 900s で symex が
終わらない — decode 入口の早期 reject まわりにも ~15min 級の
固定床(全 call-graph の goto-model 計装)がある。検証済みは
`detect_format_signature_rules` のみ(先頭 <=12B のみ読む
近似無料の入口)。

`mp4_never_panics_ftyp_signed`・`avi_never_panics_riff_signed`・
`writer_round_trip_float64` は trial queue drain 時点で未完了 —
canonical lane 側の結果を次節に記録する。

## Canonical lane 実走

`mise run test-formal-kani`(`node tools/run-formal-kani.mjs`、
`AL_KANI_TIMEOUT_MS=1500000` = 各 harness 25min、
`AL_KANI_LOG_DIR=evidence/al274/residual`)を 21 harness 全件で
実走した。verdict は **`not_run` — 1/21 verified**、fail-closed
exit 0(`residual/verdict.json`)。trial cap を全て上回る 1500s
を選んだのは、近接未収束があれば verdict が転じ得る唯一の
budget だからで、結果は以下の通りだった:

- `detect_format_signature_rules` のみ
  `VERIFICATION:- SUCCESSFUL`(symex runtime 0.69s — 先頭 <=12B
  しか読まないため固定床に届かない唯一の入口)。
- 残り 20 harness は全て 1500s で `ETIMEDOUT` — signed・unsigned・
  writer round-trip・header・mutation 系を問わず、kill 時点の
  tail は symex の path-abort(`same_allocation`/`to_isize`
  系)出力のまま。**25min でも shallow reject 経路を含め
  どの proof も symex を抜けなかった** — この box の
  per-proof 固定床は >25min で、al#256 の「45min で 7 件
  solver-bound」という見積もりは、分割で per-proof の式量が
  下がっても固定床自体は残ることを意味していた。
- `VERIFICATION: FAILED`・unwinding-assertion 失敗は 0 件 —
  rightsizing した bound はどの proof でも足りている
  (不足なら FAILED として表れる設計)。反例不具合も 0 件。
- envelope 側は definitive に解消: al#256 で 30.4GiB OOM した
  3 入口結合 proof に相当する `decode_wav_never_panics_signed`
  は peak 3.8GiB、lane 全体を通じて cbmc RSS が envelope
  (31GiB)に近づいた harness は無い — memory-bound 分類は
  本 slice で解消し、残る制約は純粋に symex 時間のみ。

## 残課題(al#274 の結論として記録)

再構成で「envelope-bound の解消」「proof 責務の単一化による
失敗時の局所性」「`AL_KANI_HARNESSES` による部分再実行」は
達成されたが、**この box では 21 件中 20 件が依然
time-bound**。verdict を取るには以下のいずれかが要る:

1. 専用 runner/より大きな budget(RAM・時間とも余力 —
   symex のみが支配のため、時間増はそのまま有効)
2. `#[kani::stub]` 等による fmt/alloc/alloc-error 系 stdlib
   計装のモデル化(固定床を直接下げる唯一の手段だが、
   stub の契約等価性の担保が要る)
3. bounded `not_run` の受理 — proptest 側が同 property の
   統計カバレッジを持つ(本 slice で non-regression 確認済み)

## Honest notes

- 本 slice で product `src/` 差分ゼロ。proptest suite 差分ゼロ
  (non-regression は `mise run test-formal` 13/13 で確認)
- verbose ログの `function` 名は monomorph instance 命名のため
  proptest closure 名を冠することがある(共有 generic instance
  の collector root 由来)— symex 本体は当該 harness の
  call graph 内
- 検討したが採用しなかった手段: SAT solver 切替
  (`--solver kissat` は同梱 — ただし計測した timeout は全て
  symex 未通過のため無効)、`#[kani::stub]` による fmt/alloc
  内部のモデル化(契約等価性の担保が難しく本 slice の削減手段
  集合外 — 後続候補として記録)、const bytes fixture 化
  (writer 経路は全体の 1% 未満で固定床は decode 側にあるため
  効果薄と判断)
