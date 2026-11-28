# 検証結果: AVI ストリーム開始 tick

対象の Sync Commit は `44acc115b3ce622e59051a51c7f51d3c88119530`。
基点は `b14746b7a34f790572ec1bb64f596b571c624559`（`origin/main`）。
[境界分析](boundary-analysis.md) に実走前の必要保証と予算を、
[実行環境](environment.txt) に OS・ツール版を記録した。
ローカル検証は変更範囲に限り、統合検証は merge 後の対象 main SHA の
必須 CI 証拠で判定する。

## Runtime Verification

Windows x64 で次を実行した。[実行ログ](native.txt) にはコマンドと exit、
[復元後の個別ログ](final-packet-ticks.txt) には二つの新しい例の再実行を記録した。

| コマンド | exit | 結果 |
| --- | ---: | --- |
| `mise run fmt` | 0 | Pass |
| `mise run check al-media` | 0 | Pass |
| `mise run lint al-media` | 0 | Pass |
| `mise run test al-media` | 0 | Pass、unit 122 件・全体 210 件 |
| `mise run test-formal` | 0 | Pass、WAV 7 件と containers 7 件 |
| `mise exec -- prek run --all-files` | 0 | Pass、[ログ](prek.txt) |
| `mise run test al-media --lib packet_ticks_` | 0 | Pass、全変異の復元後に 3 件 |

上限に収まる例では `2_281_422_937 × 4_042_815_511 = i64::MAX` が
最初の packet に付く。overflow の例では、packet を持たない他種 stream
でも `u32::MAX × u32::MAX` を拒否し、`Malformed` の理由は
`stream start offset overflows i64` のまま。回帰テストを
`packet_ticks` の接続・エラー契約の証拠とし、parser の全域証明とはしない。

## Formal Verification

Ubuntu 24.04.4 x86_64 / WSL2、Kani 0.68.0 / CBMC 6.11.0 で実行した。
`AL_KANI_TIMEOUT_MS=600000`、`AL_KANI_LOG_DIR=<証跡ディレクトリ>` と
各行の `AL_KANI_HARNESS_GROUP` を設定し、`mise run test-formal-kani` を実行する。
[復元後の実行ログ](final-formal.txt) と各ディレクトリの raw log で、
exit 0 に加えて lane `pass`、対象 harness の `VERIFICATION:- SUCCESSFUL`、
unwinding assertions を無効化していないことを確認した。

| group | exit / 結果 | harness 経過時間 | ピーク RSS (KiB) |
| --- | --- | ---: | ---: |
| [`avi-clock-start`](avi-clock-start/verdict.json) | 0 / Required Pass | 13.097 s | 255816 |
| [`avi-strh-presence`](avi-strh-presence/verdict.json) | 0 / 回帰 Pass | 15.129 s | 468132 |
| [`avi-strh-declaration`](avi-strh-declaration/verdict.json) | 0 / 回帰 Pass | 6.550 s | 308384 |
| [`avi-stream-kind`](avi-stream-kind/verdict.json) | 0 / 回帰 Pass | 20.954 s | 500020 |

開始 tick の全 `u32` 入力対を、仮定も stub も置かずに検証した。
失敗した check は 33 件中 0 件。保証は表現可能な積の正確さと
表現不能な積の拒否に限る。remainder、packet 増分・累積、stream 間の分離、
PCM、parser 全体はこの proof では未証明。

## 境界変異と復元

- **上限の超過を誤って受理:** helper の `checked_mul` を
  `Some(i64::from(start).wrapping_mul(i64::from(scale)))` に置き換えた。
  [Kani の実行](mutation-overflow-lane.txt)は exit 1、
  [raw log](mutation-overflow/kani-avi_stream_start_tick_exact.txt) は
  `avi_stream_start_tick(start, scale) == expected` の assertion failure
  （4.115 s、270952 KiB）。[Windows の個別回帰](mutation-overflow-native.txt)
  も overflow の例で失敗した（exit 101）。
- **上限ちょうどを誤って拒否:** `checked_mul` の結果に
  `.filter(|&tick| tick < i64::MAX)` を足した。
  [Windows の個別回帰](mutation-native.txt)は上限ちょうどの例で失敗した
  （exit 101）。[Kani](mutation/kani-avi_stream_start_tick_exact.txt) は
  147.851 s で[中断](mutation-endpoint-observation.txt)した。
  runner は exit 1 / `fail` を返したが、CBMC の終了理由は SIGTERM
  であり、assertion failure ではない。この試行は証明による検出に数えない。

変異を除いた `avi.rs` の blob は初回の `29053e3ac74a1db7fc225b839ee12c7e04c69af0`
に戻り、上表の開始 tick proof と Windows の個別回帰が再び Pass した。

## 保証範囲と再実行コストの比較

[旧実測](../al274/verification.md)の `avi_writer_output_decodes` は固定の
映像と象徴的な PCM サンプルを持つ writer fixture の往復を試み、
`dwStart` と `dwScale` の全域を検査しない。1200 s の設定で timeout
（観測 1267 s、ピーク 3.9 GiB）。旧 `mutated_avi_never_panics` は
1266 s / 2.9 GiB、`truncated_avi_never_panics` は 948 s / 2.9 GiB で
それぞれ timeout した。旧 RSS は CBMC と Kani driver の合算サンプル。
今回の RSS は `/usr/bin/time -v` の最大値で、計測法と保証範囲が違うため
速度比・メモリ削減率は比較しない。旧 proof の登録は維持する。

新規 group は [初回](initial/verdict.json) 29.529 s / 301688 KiB、
復元後 13.097 s / 255816 KiB。どちらも 600 s 以内・4 GiB 未満だった。
復元後の 4 group の harness 時間を合計すると 55.730 s、最大 RSS は
500020 KiB。時間には Cargo と Kani を含むが、環境検出と WSL mirror
準備は含まない。回帰時は変更に関連する group だけを選択できる。

## 未実行と適用外

- 未選択 Kani group・全件 lane、macOS/Linux のネイティブ OS-L1、および
  統合 CI は NotRun。手動 CI は起動していない。
- pstack の cloud owner / swarm は、ローカル Orca Dispatch では
  NotRun。上記の canonical Runtime / Formal Verification を実走した。
- GUI-L2、EXT-L3 は NotApplicable。AVI 固有の算術なので Ruviform への
  還元は不要。docs index は Coordinator が統合時に整備する。
