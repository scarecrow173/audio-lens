# evidence/al274-boundary

Issue #274 の verification boundary/ledger 設計 slice の証跡である。本ディレクトリは設計と照合だけを扱い、Kani の再計測や proof 実装の証拠ではない。

## ファイル

| ファイル | 内容 |
| --- | --- |
| `actions.jsonl` | 本 slice で実行した照合・生成・検証コマンドの記録。失敗・NotRun・Unsupported は理由と再現条件を残し、Pass に読み替えない |
| `proof-inventory.json` | canonical 21 proof、一時 spike 12 proof、proptest 13 件の machine-readable inventory。runner/test code を正本とする |
| `proof-inventory.md` | 同 inventory の人間向け表。旧契約、現行の取得保証、必要保証、入力/状態領域、Bound、assume/Stub、cost verdict、proptest overlap、新検証、未証明領域/残留リスクを示す |
| `classification.json` | canonical proof の disposition、要求状態、接続条件、target verification structure の machine-readable ledger |
| `classification.md` | 同 ledger の人間向け表。WAV/MP4/AVI の Property 単位の必要保証・粒度・境界・手法・実行予算を示す |

## 正本と解釈

- baseline は `origin/main` `c91b8ce20c90733fa21098451cb768a74425884d` であり、inventory の行番号はこの HEAD に対応する。
- canonical proof の正本は `tools/run-formal-kani.mjs` の HARNESSES と `crates/al-media/tests/formal_wav.rs` / `formal_containers.rs` である。`docs/specs/formal-verification.md` の記述は正本にしない。
- `spike_` harness は stub ablation 再現用の一時資産であり、canonical HARNESSES に登録しない。実行・集計で canonical と混ぜない。
- proptest は統計的 Property Tested、Kani は宣言した bound 内の検証である。両者を同じ保証種別として扱わない。
- Timeout、環境不足、exit 0、CI green を Proof Pass とみなさない。要求状態 Required/Deferred/NotApplicable と verdict Pass/Fail/NotRun/Unsupported は別軸で管理する。
- 既存定量根拠 `evidence/al274/` は read-only 引用である。canonical 1500s/proof と ablation 600s cap の条件差を混同しない。

## この slice で行っていないこと

- `crates/`、`tools/`、`mise.toml`、runner 意味論、proof、lazy diagnostic の変更。
- Kani の再計測、Required 検証の実走、mutation detection の新規実行。
- audio-lens#278 の最終文書再構成と docs index 整備。
