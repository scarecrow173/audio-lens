# al#274 stub-spike evidence — `#[kani::stub]` による solver 固定床の実測

対象 Issue: <https://github.com/scarecrow173/audio-lens/issues/274>
実施日: 2026-11-25
sync commit: `7fd87add13e24858f6677d9c2dea135bf581ebe6`(作業開始時 `origin/main`)

## 目的と範囲

al#274 の 21 単一責務 proof 構成でも 20/21 proof が 1500s budget で
symex 未収束だった固定床に対し、test-only の `#[kani::stub]` モデル化が
wall-clock・peak RSS・symex 到達深度をどこまで改善するかを代表 3 proofで
実測した。product `crates/al-media/src/` は変更していない。proptest 側の
property 意味も変更していない。

代表 proof:

- `wav_reads_never_panics_unsigned`: unsigned shallow WAV probe
- `read_wav_range_never_panics_signed`: signed deep WAV range probe
- `mp4_writer_output_decodes`: MP4 writer/container round-trip probe

本 README の ablation は WSL mirror 内で `cargo kani` を直接実行した
**補助証拠**である。正規 lane verdict は `mise run test-formal-kani` の
canonical 結果として `canonical/` に分離する。

## 環境と実行方法

環境は `env.txt` に記録した。要約: Windows 11 + WSL2 Ubuntu、
kani-verifier 0.68.0、CBMC 6.11.0、Kani 側 rustc nightly-2026-08-21、
31GiB RAM + 8GiB swap、16 vCPU。queue 開始時に競合する cbmc/kani-driver
process は無かった。観測した CBMC command line には従来 lane と同じく
`--no-malloc-may-fail` が含まれる。

ablation matrix は `run-ablation.sh` で実行した。proof body・入力 bound・
unwind・mirror source を同一化するため、既存 `evidence/al274/trials` は
baseline として使わず、no-stub control も同じ shared body から同じ 600s
cap で再測した。12 job(baseline/fmt-only/alloc-only/combined × 代表 3)を
concurrency 3 で実行した。

再現性に関する補正: 当初の 12 job 実行時点では、WAV/mp4 の canonical
harness 自身がまだ no-stub だったため、その名前を baseline control に
使った。最終 head では canonical WAV 2 harness が fmt stub を採用済みなので、
現在の `run-ablation.sh` は同じ proof body/unwind を持つ明示的な
`spike_wav_unsigned_baseline`・`spike_wav_range_baseline`・
`spike_mp4_writer_baseline` を control に使う。したがって保存済み table の
baseline 行は historical job 名(`wav_reads_never_panics_unsigned` 等)を
残し、再実行時の output file 名は `spike_*_baseline` になる。両者は同じ
shared body と入力条件を指すが、byte-for-byte 同じ harness 名ではない。

計測の操作化:

- wall-clock: `run-ablation.sh` が harness job 開始から timeout/後処理
  完了までに計測した実経過秒。cargo compile 時間を含み、600s cap の
  job は概ね 662–663s となる
- peak RSS: harness 名を含む cbmc + kani-driver の RSS 合算を 10s 間隔で
  採取した peak (`*.rss.txt`)
- verbose/event lines: `--log-file` の行数と、symex イベント行
  (`Unwinding`/`Not unwinding`/`aborting path`/`entering`)数
- symex 到達深度: 厳密な完了率は CBMC verbose から取れないため、
  `function al_media::...` / `function al_testkit::...` を含む
  イベント数(al#274 で支配因と区別した product/testkit 側到達の proxy)、
  product loop iteration、最大 unwind iteration、末尾非空行を併記する。
  この proxy は進行量の単位ではなく、stdlib 計装内で停滞しているか
  product 側へ到達したかを比較する定性指標として扱う
- 抽出は `summarize_ablation.py` が `ablation/summary.json` と
  `ablation/table.md` に再生成する

## compile smoke: stub が実際に replacement へ解決すること

効果測定前に、`#[kani::stub]` が実際に適用されることを compile smoke で
確認した。

- `spike_wav_unsigned_fmt`: verbose header に
  `Stub: std::fmt::format -> crate::kani_stub_models::fmt_format_empty`
  が出力され、fmt 側 verbose log には `memchr`/`PadAdapter` イベントが
  残らなかった。nightly-2026-08-21 では `format!` は `alloc::fmt::format`
  へ lowering されるが、`std::fmt::format` 指定で同一 function として
  解決された
- `spike_wav_unsigned_alloc`: verbose header に
  `Stub: std::alloc::handle_alloc_error ->
  crate::kani_stub_models::handle_alloc_error_infallible`
  が出力され、replacement 内の `kani::assume(false)` に対応する早期
  path-abort が確認できた
- したがって `std::fmt::write` fallback は本 spike では測定不要と判断した。
  `format` が解決しない場合の fallback であり、主系が有効と確認できた
  ためである

raw log: `ablation/spike_wav_unsigned_fmt.compile-smoke.verbose.log`、
`ablation/spike_wav_unsigned_alloc.compile-smoke.verbose.log`、
lowering probe は `ablation/fmt-lowering.probe.txt`。

最終 head で追加した no-stub control 3 harness も compile smoke 済みである。
`spike_wav_unsigned_baseline`・`spike_wav_range_baseline`・
`spike_mp4_writer_baseline` はいずれも verbose log に `Checking harness ...`
を出力して CBMC へ到達し、120s timeout で exit 124 となった。timeout は
verification 未完了を意味し、compile 失敗ではない。これらは no-stub
control なので `Stub:` 行は出ない。raw log は
`ablation/spike_*_baseline.compile-smoke.*`。

## ablation 結果(補助証拠)

`ablation/table.md` の生成表を転記する。全 job は 600s cap で、
`timeout` は verification 未完了を意味する。mp4 の fmt-only/combined は
contract-equivalent ではない control であり、verdict や性能を canonical
proof の証拠としては使わない(理由は後述)。

| proof | variant | wall-clock s | peak RSS MiB | verbose/event lines | max unwind | product/testkit events | product loop iter | verdict | memchr | alloc_rt_error |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: |
| wav_reads_never_panics_unsigned | baseline-control | 662 | 5818 | 8175/8165 | 17 | 0 | 0 | timeout | 1124 | 254 |
| spike_wav_unsigned_fmt | fmt-only | 662 | 7883 | 1948/1937 | 16 | 32 | 16 | timeout | 0 | 32 |
| spike_wav_unsigned_alloc | alloc-only | 663 | 6198 | 7974/7963 | 17 | 0 | 0 | timeout | 1124 | 0 |
| spike_wav_unsigned_both | combined | 662 | 7869 | 1917/1905 | 16 | 32 | 16 | timeout | 0 | 0 |
| read_wav_range_never_panics_signed | baseline-control | 662 | 8471 | 17705/17695 | 41 | 0 | 0 | timeout | 5279 | 20 |
| spike_wav_range_fmt | fmt-only | 663 | 6744 | 2419/2408 | 19 | 38 | 19 | timeout | 0 | 40 |
| spike_wav_range_alloc | alloc-only | 663 | 6321 | 13183/13172 | 41 | 0 | 0 | timeout | 3958 | 0 |
| spike_wav_range_both | combined | 663 | 6701 | 2367/2355 | 19 | 38 | 19 | timeout | 0 | 0 |
| mp4_writer_output_decodes | baseline-control | 662 | 4356 | 2557/2547 | 48 | 118 | 0 | timeout | 0 | 124 |
| spike_mp4_writer_fmt | fmt-only | 663 | 4298 | 2506/2495 | 48 | 112 | 0 | timeout | 0 | 124 |
| spike_mp4_writer_alloc | alloc-only | 663 | 4298 | 2389/2378 | 48 | 113 | 0 | timeout | 0 | 0 |
| spike_mp4_writer_both | combined | 663 | 4303 | 2395/2383 | 48 | 113 | 0 | timeout | 0 | 0 |

### 観測

1. **fmt stub は target machinery を完全に除去し、WAV では product 側へ進んだ**:
   no-stub control は 600s 時点で product/testkit function event が 0 件で、
   verbose の末尾まで fmt/memchr・alloc・pointer 計装内にいた。fmt-only と
   combined では `memchr`/`PadAdapter` が 0 件になり、unsigned では
   `wav_layout` iteration 16、range では iteration 19 まで到達した。
   verbose/event lines も unsigned 8165→1937、range 17695→2408 へ大幅に
   減った。これは noise では説明しにくい symex 到達深度の改善である。
2. **ただし verdict と wall-clock は 600s cap では改善しなかった**:
   全 variant が timeout で、wall-clock は compile/後処理込みでほぼ同一。
   unsigned では peak RSS が 5818→7883MiB と増えた。range では
   8471→6744MiB に下がったが、 verdict を得るには不十分だった。
   fmt machinery を取り除くと、それまで stdlib 診断経路で消費されていた
   探索が product の parse/range 分岐と memory-safety 計装へ進むため、
   「遅いまま」ではなく「次の支配因へ進んだ」結果と解釈する。
3. **alloc stub は failure endpoint の rt_error を消すが床は下げなかった**:
   `handle_alloc_error::rt_error` event は alloc-only/combined で 0 になった。
   しかし unsigned/range では product 到達が 0 のまま(no-fmt)か fmt-only と
   ほぼ同じ(combined)で、mp4 でも peak RSS は 4356→4298MiB、event lines
   2547→2378、product/testkit events 118→113 と誤差〜小改善に留まった。
   capacity overflow・Layout/TryReserve branch・RawVec grow と pointer
   validity 計装が残るため、failure endpoint だけの剪定は支配因ではない。
4. **mp4 fmt 行は contract control であり効果不適格**:
   fmt-only/combined の数値が baseline とほぼ同じなのは、この proof では
   そもそも fmt/memchr 比率が低いことと一致する。ただし codec 名 assertion が
   `format!` 出力へ依存するため、これらの行は semantic control であり、
   採用候補の performance evidence ではない。

支配的な残存 function の上位表は `ablation/residual-top-functions.md`
に残した。fmt 適用後の WAV では `to_isize`・`same_allocation`・
`split_at`・`slice_index_fail`・parser の Option/Result plumbing が上位で、
mp4 では `same_allocation`・`to_isize`・RawVec/Layout/capacity 系が上位で
ある。これらは検証対象の memory-safety / parse / allocation-capacity 意味論
に属し、等価 stub の候補ではない。

## stub ごとの契約等価性レビュー

### `fmt-format-empty`(`std::fmt::format` -> 空文字列)

採用対象 property は panic しないこと、decode metadata/sample/range の一致
であり、人間向け `MediaError::reason` 文字列の内容を検査しない。WAV の
2 representative proof はいずれも decode/read 結果を `let _ =` で捨て、
error variant や reason 文字列を分岐・assertion に使わない。したがって
診断文字列を空にしても、これらの proof が検証する契約には影響しない。

一方で検証外になるものは次である。

- error message の内容
- formatting 自体の panic/alloc 挙動
- message 長に依存する allocation 挙動

これらは product 契約ではなく診断表現の挙動であり、本 lane が対象とする
memory-safety/parse/write 契約とは分離する。`docs/specs/formal-verification.md`
の assumptions にも同じ境界を記録した。

`mp4_writer_output_decodes` では decode 結果 `a.codec` が
`format!` で作られる `pcm_s16le` 文字列であり、proof はそれを直接 assert
する。したがってこの proof では fmt stub は contract-equivalent ではなく、
ablation の fmt-only/combined は mechanism control に過ぎない。canonical
harness には適用しない。

### `alloc-error-infallible`(`std::alloc::handle_alloc_error` -> infeasible)

観測した CBMC command line には `--no-malloc-may-fail` が既に含まれる。
したがって allocator failure は現行 lane で既に「起きない」と仮定された
環境であり、この stub は失敗枝へ到達した後の `rt_error` 付近まで symex が
進んでから剪定される経路を、failure boundary で早期に infeasible へ
剪定するだけである。capacity overflow・Layout 計算・`TryReserveError`
の分岐は stub していないため、入力依存の overflow/panic は残る。

実メモリ不足時の製品挙動は従来 lane でも検証対象外である。この意味では
等価性の筋は強いが、採用可否は semantic 適格性だけでなく ablation の
効果量で判断した。

## 採用判定と canonical harness への反映

- **採用: `fmt-format-empty` を WAV の 2 representative harness に適用**。
  `wav_reads_never_panics_unsigned` と `read_wav_range_never_panics_signed` は
  error text を property に使わず、compile smoke と ablation で target 経路除去・
  product 側到達が確認できた。canonical harness には同じ per-harness attribute を
  反映済みである。
- **不採用: `alloc-error-infallible`**。semantic 上は `--no-malloc-may-fail`
  と整合するが、支配的な RawVec/Layout/capacity/pointer 計装は残り、
  代表 3 proof で wall-clock・peak RSS・product 到達の有意な改善がなかった。
  したがって canonical harness には追加しない。
- **不採用: mp4 の fmt stub**。`a.codec == "pcm_s16le"` が `format!` 出力を
  検証するため contract-equivalent ではない。
- **不採用: `fmt-write-noop`**。`std::fmt::format` が実際に解決したため
  fallback 測定は不要と判断し、helper には残さなかった。
- 一時 `spike_` harness は ablation 再現のため残す。canonical runner の
  `HARNESSES` には登録していないため、既定の 21 件全走には混ざらない。
  no-stub control も `spike_*_baseline` として明示し、fmt stub 採用後の
  canonical WAV harness を control に誤用しないようにした。ただし mirror 内で
  `cargo kani --tests` を harness 指定なしに直接実行すると spike も
  compile/verification 対象になるため、再現は `run-ablation.sh` の個別
  `--harness` 指定経由に限る。特に mp4 fmt/both は contract-equivalent でない
  control なので、それらの結果を product の反例として扱ってはいけない。

## canonical 代表 subset(正規 lane)

実行コマンド(repo ルート、正規 lane):

```sh
AL_KANI_HARNESSES=wav_reads_never_panics_unsigned,read_wav_range_never_panics_signed,mp4_writer_output_decodes AL_KANI_TIMEOUT_MS=1500000 AL_KANI_LOG_DIR=evidence/al274/stub-spike/canonical mise run test-formal-kani
```

raw 出力と lane verdict は `canonical/` に保存する。canonical lane は
sequential 実行で、ablation の 3 並列 trial とは実行条件が異なる。
`canonical/rss-samples.txt` は途中開始の補助 RSS 採取であり、全行程の
peak を保証しない(欠測がある旨を結果表に明記する)。

| harness | adopted stub | verdict | peak RSS MiB | memchr/PadAdapter | product/testkit 到達 | raw log |
| --- | --- | --- | ---: | --- | --- | --- |
| read_wav_range_never_panics_signed | fmt | timeout(1500s) | 10324* | 0/0 | `wav_layout` iteration 29 | `canonical/kani-read_wav_range_never_panics_signed.txt` |
| wav_reads_never_panics_unsigned | fmt | timeout(1500s) | 13762 | 0/0 | `wav_layout` iteration 16、`decode_samples` まで到達 | `canonical/kani-wav_reads_never_panics_unsigned.txt` |
| mp4_writer_output_decodes | none | timeout(1500s) | 7150 | 0/0 | `write_mp4` 到達、product/testkit event 285 | `canonical/kani-mp4_writer_output_decodes.txt` |

`*` range の RSS 採取は 09:16UTC に開始した。sequential 実行と timeout
境界から range harness はおおよそ 08:56–09:21UTC と推定されるため、
開始後の約 20 分を欠測している。採取開始時点で既に 9566MiB、その後は
終了直前まで増加して 10324MiB だった。したがって表中の値は観測された
peak であり、欠測区間を含む全行程 peak を厳密に保証するものではない。
`VERIFICATION:- FAILED` は 3 件の log に無く、kill 時点はいずれも
symex の path exploration だった。fmt stub は両 log の header に
`Stub: std :: fmt :: format -> crate :: kani_stub_models ::
fmt_format_empty` として出力され、memchr/PadAdapter が canonical raw
log でも 0 件であることを確認した。

## 展開見通し / 支配因の再分析

fmt stub は床を完全に取り除くものではないが、WAV no-panic proof では
診断 formatting 由来の人工負荷を確実に取り除き、symex を product code
まで進める。canonical 1500s でも `memchr`/`PadAdapter` は 0 件、range は
`wav_layout` iteration 29、unsigned は `decode_samples` まで到達した。
一方で両者とも verdict は得られず、unsigned の peak RSS は 13.7GiB に達した。
残る支配因は pointer/slice validity (`to_isize`/`same_allocation`)、slice
bounds panic machinery、parser branch、RawVec/Layout/capacity 計算であり、
いずれも memory-safety または入力依存 panic の意味論に近いため、等価 stub
では消せない。

したがって次のように判定する。

1. **fmt stub の限定的な展開価値はある**。`MediaError::reason` の文字列や
   `format!` 生成物を property が使わない WAV read/no-panic family では、
   診断経路のノイズ除去と到達深度確認に有効。usage review と compile smoke
   (`Stub:` 解決)を各 harness 単位で必須にする。
2. **verdict 改善を直接期待して全 proof に展開する根拠はない**。代表 2 件が
   1500s でも timeout のため、fmt だけで 25 分/proof 内に収まるとは言えない。
   少なくとも代表 proof では `>1500s/proof` が残り、21 件を同 budget で走らせる
   worst-case 見積もりは依然 8.75 時間級である。fmt が新たな成功を示さない
   限り、全走時間の短縮量を定量主張できない。
3. **alloc-error stub は展開しない**。event 削減より RSS/壁時間/深度への効果が
   小さく、canonical でも allocation failure 末端より RawVec 分岐や pointer
   instrumentation が残った。
4. **mp4/container round-trip に fmt stub は適用しない**。`codec` など
   `format!` 生成文字列を metadata assertion が使うため等価でない。
   writer/container family の支配因は allocation/Vec/RawVec と codec/tag 文字列
   の分岐なので、本 spike の安全な stub では床を下げられない。

推奨する次手は、fmt を有効と確認できた WAV family にだけ限定的に展開して
探索深度を改善しつつ、verdict を取る目的では runner/budget/input-domain
分割を別途扱うことである。memory validity instrumentation の stub 化や
memory-safety check の無効化は検証弱化なので採用しない。product 側の恒久策
(lazy diagnostic 化など)は本 slice 外だが、Kani 以外にも保守性利益があるかを
別途設計判断すべき候補である。

## pstack Verification

本 spike で実行した検証を pstack の区分に沿って記録する。ここでの
Runtime Verification は実バイナリ/通常 test lane、Formal Verification は
proptest/Kani lane、review は pstack review panel を指す。

| 種別 | 結果 | 根拠 |
| --- | --- | --- |
| Runtime Verification | Pass | `mise run test al-media` が Windows host で exit 0。fmt/check/lint は別途 static/hygiene check として exit 0 |
| Formal/property lane(proptest) | Pass | `mise run test-formal` が Windows host で exit 0。proptest は bounded property test であり proof 成功とは区別する。property の意味は変更していない |
| Formal Verification(Kani canonical) | Executed / lane verdict `not_run` | 代表 3 harness を 1500s/proof で実行したが全て timeout。`SUCCESSFUL`/`FAILED` はなく、proof verdict 未取得であることを成功とは記録しない |
| pstack review panel | NotRun | 本 slice は測定 spike で review panel を実行しなかった。代わりに raw evidence・compile smoke・canonical/mise lane・prek/biome hygiene を残し、review panel 実施を後続の統合判断に委ねる |

## Honest notes

- ablation は compile 時間を含み、3 job 同時実行である。canonical lane は
  sequential なため、wall-clock/RSS を直接同一視しない
- product/testkit function event 数は product 側到達の proxy であり、正確な
  探索完了率ではない。timeout した proof の厳密な symex 深度は CBMC
  verbose から単一数値では再構成できなかった
- fmt-only/combined の mp4 行は contract violation を含む control であり、
  採用候補としての性能証拠ではない
- `*.log` は repository の gitignore 規則に該当するが、証拠必須のため
  既存 `evidence/al274/trials` と同様に明示的に tracked 化する
