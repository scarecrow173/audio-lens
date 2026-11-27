# 検証結果: AVI `strh` presence

実装と proof の Sync Commit は `957d4f90ee4ac014a94641f3b7baa2fb21cb0720`。
基点の `main` は `bd7bda931a9080b0e6233005016358f919922b11`。
[実行環境](environment.txt) は Windows x64 build `10.0.26300.9974`、
mise 2026.11.2、rustc 1.99.1。Kani は Ubuntu 24.04.4 LTS / WSL2
Linux `6.6.87.2-microsoft-standard-WSL2`、Kani 0.68.0、CBMC 6.11.0 を使う。
ローカル検証は変更範囲が対象であり、統合検証は merge 後の対象 main SHA
に対する Daily CI 証拠で判定する。

## 証明と実パーサーテストの分担

Required の局所証明は、三つの entry に対する presence または最初の
不正 entry の位置を検査する。[境界分析](boundary-analysis.md) のとおり、
実パーサーテストは `avi_layout` の stream table と malformed の理由を
検査する。`strf` と `strH` だけでは stream を追加せず、正常 `strh` が
二件あっても一件だけ追加する。また短い `strh` は先行する正常 `strh` の
有無を問わず既存のエラーとなり、後続する構造エラーより先に返る。

## Runtime Verification

Windows native で変異復元後に実行した。[実行ログ](final-native.txt) に
コマンド、対象 SHA と exit を記録した。

| コマンド | exit | 結果 |
| --- | ---: | --- |
| `mise run fmt` | 0 | Pass |
| `mise run check al-media` | 0 | Pass |
| `mise run lint al-media` | 0 | Pass |
| `mise run test al-media` | 0 | Pass、unit 120 件、全体 208 件 |
| `mise run test-formal` | 0 | Pass、WAV 7 件、containers 7 件 |
| `mise exec -- prek run --all-files` | 0 | Pass、[実行ログ](prek.txt) |
| `mise exec -- node --check tools/run-formal-kani.mjs` | 0 | Pass（補助検証） |

[`prek` 初回](prek-initial.txt) は raw log の行末空白と改行、
verdict の JSON 書式を整形した。比較で log の内容と JSON の値が
変化していないことを確認して再実行した。

## Unwind bound の確認

初回 `fbf9e1478fc73a1ca1246dfc5a8289265c3ab7b6` の unwind bound 4 では、
4 byte の tag 一致比較に到達した `memcmp.unwind.0` が失敗した。
これは製品契約への反例ではなく、tag 比較ループの展開が足りないことを
示す。unwinding assertions は無効化せず、bound を 5 に変更した。
入力領域と oracle は変えていない。

| 実行 | exit / verdict | wall ms | harness ms | peak RSS KiB |
| --- | --- | ---: | ---: | ---: |
| [bound 4](kani-initial-lane.txt) | 1 / `fail`（unwinding） | 23972 | 21523 | 465508 |
| [bound 5・変異前](before-mutation-lane.txt) | 0 / `pass` | 35653 | 25351 | 392548 |

## 変異の検出と復元

正常な `strh` が先行した場合に限り、`parse_strh` の失敗を無視する
変異を入れた。[変異実行](mutation-lane.txt) は exit 1 / `fail` で、
[raw log](kani-mutation/kani-avi_strh_presence_bounded_exact.txt) の
唯一の失敗 assertion は `strh presence or first invalid entry`。
wall 18933 ms、harness 16589 ms、peak RSS 416280 KiB だった。
無変異の `avi.rs` blob `5a46409b4ce5e76b97f74dcc2f0c622e38433543` に戻し、
最終実行の冒頭で元の blob と一致することを確認した。

## Formal Verification

[最終実行](final-formal.txt) は変異復元後に行った。4 group を逐次実行し、
exit 0 に加えて各 verdict の `pass` と全 6 proof の
`VERIFICATION:- SUCCESSFUL` を照合した。unwinding assertions は有効。

| `AL_KANI_HARNESS_GROUP` | exit / verdict | proofs | wall ms | harness ms | peak RSS KiB |
| --- | --- | ---: | ---: | ---: | ---: |
| [`avi-strh-presence`](avi-strh-presence/) | 0 / `pass` | 1/1 | 15973 | 13402 | 401280 |
| [`avi-strh-declaration`](avi-strh-declaration/) | 0 / `pass` | 1/1 | 10883 | 8189 | 307976 |
| [`avi-strf-capture`](avi-strf-capture/) | 0 / `pass` | 3/3 | 37087 | 31756 | 436408 |
| [`avi-wb-concatenation`](avi-wb-concatenation/) | 0 / `pass` | 1/1 | 43075 | 40371 | 823084 |

新規 proof は設計予算 120 秒、4 GiB 内。4 group の合計は wall
107018 ms、harness 93718 ms。ピークは和ではなく最大の 823084 KiB。
wall は WSL の probe と source mirror を含むコマンド全体、harness
は runner の計測区間、peak RSS は GNU time `-v` の測定である。
新規 proof の無変異 2 回は wall 15973〜35653 ms、peak RSS
392548〜401280 KiB だった。既存 mirror を使う再実行の観測値であり、
cold build や保証条件を揃えた性能比較ではない。

## 旧構成との比較

旧 canonical の `avi_writer_output_decodes`、`mutated_avi_never_panics`、
`truncated_avi_never_panics` は whole-call の有限入力に対する
round-trip または no-panic を問う。[当時の canonical verdict](../al274/residual/verdict.json)
は各 1500 秒で timeout、個別の canonical peak RSS は未記録。
[別の trial](../al274/verification.md) では下表の中断値を得た。

| 旧 trial | timeout / wall | symex 中の RSS |
| --- | --- | ---: |
| `avi_writer_output_decodes` | 1200 s / 1267 s | 3.9 GiB |
| `mutated_avi_never_panics` | 1200 s / 1266 s | 2.9 GiB |
| `truncated_avi_never_panics` | 900 s / 948 s | 2.9 GiB |

旧 proof を今回の局所 proof に置換していない。旧 trial と新 lane は
入力領域・契約・実行条件が異なるため、順位や速度比の比較ではない。
局所契約を一つの group で選択し直せることと、その再実行コストを示す。

## 未実行と適用外

- 未選択 Kani group と legacy 全件 lane は NotRun。旧 registry と証跡は
  残し、今回の結果を任意長の AVI 走査や公開 API の証明に使わない。
- macOS/Linux ネイティブの回帰と統合検証は NotRun。統合時は、merge
  後の main SHA の Daily CI を確認する。手動 CI は起動していない。
- pstack の cloud owner / cloud swarm は Devin のローカル Dispatch
  では利用できないため NotRun。代わりに上記の canonical 検証を実走した。
- GUI-L2、EXT-L3 は今回の局所 parser 変更には NotApplicable。
  AVI 固有の処理であり、Ruviform への還元対象はない。
- docs index は Coordinator が統合時に整備する。
