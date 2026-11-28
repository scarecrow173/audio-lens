# 検証結果: MP4 PCM declaration/capability

基点は `f187ab33927359dc927dd32b39e4a65bd8cfb704`（作業開始時の `origin/main`）。PR #300 merge commit `0f1a1aac86336bff98df8bab1162e23bf5c36672` が ancestor であることを `git merge-base --is-ancestor` exit 0 で確認した。検証対象 SHA は `5c3a6f5921c791d53485e8fab5600c0b76450b1b`。この後の evidence metadata 更新は製品コード・検証対象ドキュメントを変えない記録同期である。

[境界分析](boundary-analysis.md) を実装前に確定した。`.devin/dispatch-specs/al274-next-after-pr300.explore.json` はこの worktree に存在しないため、Dispatch Spec に転記された recommended slice を operative input とした。ローカル検証は変更範囲の検証であり、統合検証は merge 後の対象 main SHA に対する CI 証拠で判定する。

## Runtime Verification

Windows x64 / MINGW64、rustc 1.99.1、mise 2026.11.2 で実行した。生ログは `check.pre.txt`、`lint.pre.txt`、`test.pre.txt`、`test-formal.pre.txt`、`fmt.final.txt`、`node-check.final.txt`、`prek.final.txt` にある。

| コマンド | exit | 結果 |
| --- | ---: | --- |
| `mise run fmt` | 0 | Pass（final） |
| `mise run check al-media` | 0 | Pass |
| `mise run lint al-media` | 0 | Pass |
| `mise run test al-media` | 0 | Pass。lib 125 件を含む全対象テスト |
| `mise run test-formal` | 0 | Pass。`formal_wav` 7 件、`formal_containers` 7 件 |
| `mise exec -- node --check tools/run-formal-kani.mjs` | 0 | Pass |
| `mise exec -- prek run --all-files` | 0 | Pass。docs index 未同期は Coordinator-owned の許容済み事項 |

初回 `mise run fmt` は未整形差分で exit 1 だった。`mise exec -- cargo fmt --all` を補助的に適用し、最終 canonical `mise run fmt` は exit 0 である。`cargo` 直接実行は canonical 結果として使っていない。

## Formal Verification

Ubuntu / WSL2、Kani Rust Verifier 0.68.0、CBMC 6.11.0 で実行した。runner は WSL mirror 内で実行し、Windows repo の target-dir 設定や `CARGO_TARGET_DIR` を変更していない。`AL_KANI_TIMEOUT_MS=600000`、unwinding assertions を無効化する runner flag はなく、新規 proof には `#[kani::unwind]`、loop bound、`kani::assume`、fmt/alloc/IO stub がない。

### Required proof group

コマンド:

```bash
AL_KANI_HARNESS_GROUP=mp4-pcm-declaration AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-mp4-pcm-declaration-proof/kani mise run test-formal-kani
```

exit 0、lane verdict `pass`。各 raw log に `VERIFICATION:- SUCCESSFUL` がある。

| harness | verdict | elapsed | max RSS |
| --- | --- | ---: | ---: |
| `mp4_pcm_declaration_classification_exact` | SUCCESSFUL | 15.733 s | 336772 KiB |
| `mp4_pcm_capability_width_exact` | SUCCESSFUL | 5.750 s | 316184 KiB |

合計 21.483 s、最大観測 RSS 336772 KiB。設計予算の 1 proof 600 秒以内・4 GiB 未満を満たす。

### Regression proof groups

| group | exit / verdict | harness | elapsed | max RSS |
| --- | --- | --- | ---: | ---: |
| `mp4-scan` | 0 / pass | `mp4_box_scan_progress_bounded_exact` | 24.864 s | 317600 KiB |
| `mp4-avi-boundary` | 0 / pass | `mp4_box_header_step_exact` | 11.153 s | 315168 KiB |
| `mp4-avi-boundary` | 0 / pass | `avi_chunk_step_exact` | 8.399 s | 312648 KiB |

raw log と verdict は `kani-mp4-scan-regression/`、`kani-mp4-avi-boundary-regression/` にある。

## Mutation detection と byte-for-byte revert

semantic mutation として、production `pcm_sample_width_value` の integer 24-bit width を 3 byte から 2 byte へ変更した。対象は sample-width support であり、diagnostic-only mutation ではない。

- mutation lane: exit 1、lane verdict `fail`。
- `mp4_pcm_capability_width_exact`: `VERIFICATION:- FAILED`。
- `mp4_pcm_declaration_classification_exact`: `VERIFICATION:- SUCCESSFUL`（width mutation は declaration proof の対象外）。
- mutation raw log: `mutation/kani/`。
- 復元前後 SHA-256: `e86f5123ce1307317940a996a816cbaa37c2e02fad56dad7acbc82ac2aeb9a9c`。
- `cmp` exit 0。byte-for-byte copy `mutation/mp4.rs.before-mutation` で復元した。

post-revert canonical lane:

```bash
AL_KANI_HARNESS_GROUP=mp4-pcm-declaration AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-mp4-pcm-declaration-proof/kani-post-revert mise run test-formal-kani
```

exit 0、lane verdict `pass`。`mp4_pcm_declaration_classification_exact` は 7.976 s / 312888 KiB、`mp4_pcm_capability_width_exact` は 11.397 s / 315984 KiB、両方 `VERIFICATION:- SUCCESSFUL`。

## Production seam と保証範囲

proof harness は production `pcm_layout`、`pcm_sample_width_value`、`pcm_supported_width` を直接呼ぶ。`pcm_tracks` と first PCM extraction selection は production `first_pcm_declaration` を共有する。proof-only implementation で本物の判定を迂回していない。`pcm_sample_width` の公開 error shell は error 文言を保ったまま allocation/fmt を持たない matrix helperへ委ねる。

証明範囲は fixed scalar sound-entry declaration domain の PCM declaration/capability classification、ISO `pcmC` presence/endianness/sample size、`lpcm` flags/default、unknown/compressed complement、zero channel/rate gate、sample-width matrixに限る。parser integration gap、sample table、PCM extraction/endian conversion/payload decode semantics、diagnostics、I/O、allocation、writer round-trip、AVI、WAV conversion/writer、公開 API 全体 no-panic は証明していない。

## Cost comparison

限定比較は [cost-comparison.md](cost-comparison.md) に記録した。今回の 2 proof は初回合計 21.483 s、post-revert 合計 19.373 s、最大 RSS 336772 KiB。旧 whole-call/container proof の timeout cost と直接の速度比は計測方法と保証範囲が違うため計算しない。

## NotRun / Unsupported / NotApplicable

- 未選択 Kani group、legacy timeout proof を含む全件 lane: NotRun。この slice の必須検証ではない。
- branch CI: NotApplicable/NotRun。`gh run list --branch feature/274-mp4-pcm-declaration-proof --workflow ci.yml --limit 10` は exit 0・空出力だった。`ci.yml` は `workflow_dispatch` のみで branch push では起動しない。manual dispatch は起動していない。
- 統合 CI: NotRun。merge 後の main SHA に対する CI 証拠は Coordinator が確認する。
- GUI/hardware/audio-device gate: NotApplicable。
- Windows `os error 206`: 発生せず。
- `/usr/bin/time -v`: WSL Ubuntu で利用可能であり、全 Kani proof で max RSS を測定した。
