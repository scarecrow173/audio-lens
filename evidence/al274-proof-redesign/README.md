# al274 WAV-RANGE-008 exact valid-range proof redesign evidence

この evidence は Issue #274 のうち、PR #281 で Required NotRun だった
WAV-RANGE-008 exact valid-range proof を再設計した slice の記録である。
Issue 全体の完了証拠ではない。

## 正本

- `boundary-analysis.md`: exact input domain、証明性質、assume 分類、
  decomposition 候補の採否、coverage、runner 戦略。
- `verification.md`: canonical 検証コマンド、exit code、lane verdict、
  mutation/revert、cost 比較。
- `ledger.json`: 機械可読な assurance ledger。
- `actions.jsonl`: 実行・判断・失敗・再試行・mutation/revert の時系列。
- `kani/`: mutation revert 後の最終 canonical wav-range lane raw log と
  `verdict.json`。
- `mutation-kani/`: 一時 arithmetic mutation に対する意図的失敗の raw log
  と verdict。
- `kani-valid-constructive/`・`kani-invalid-initial/`: 設計変更の初期実走
  証跡。最終判定は `kani/` を使う。

## 最終結果

```text
AL_KANI_HARNESS_GROUP="wav-range" AL_KANI_TIMEOUT_MS=300000 \
  AL_KANI_LOG_DIR=evidence/al274-proof-redesign/kani \
  mise run test-formal-kani
```

Windows host + WSL2 Ubuntu、Kani 0.68.0 / CBMC 6.11.0、default CaDiCaL。
lane verdict は `pass`、exit code は 0。raw log では 2 proof とも
`VERIFICATION:- SUCCESSFUL`。

| proof | verdict | elapsed | max RSS |
| --- | --- | ---: | ---: |
| `wav_range_plan_valid_ranges_exact` | SUCCESSFUL | 10899 ms | 279020 KiB |
| `wav_range_plan_invalid_ranges_error` | SUCCESSFUL | 4490 ms | 276720 KiB |

max RSS は runner が `/usr/bin/time -v` で包んだ `cargo kani` 起動単位の
GNU time 計測値であり、個別 solver process だけの cgroup 計測ではない。
この条件付き計測でも 4 GiB を大きく下回る。

## 保証しないもの

この local proof は seek/read/short read、allocation、PCM decode、
`std::fmt` diagnostic formatting、公開 API 全体 no-panic を証明しない。
証明対象は production pure helper の range arithmetic と、その helper を
`read_wav_range` が使う接続条件までである。
