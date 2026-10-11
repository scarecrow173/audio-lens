# Query Log

| Query ID | Task | Query/Source | Tool | Rationale |
|---|---|---|---|---|
| Q1 | T1 | docs.rs/wide/latest | webfetch | 公式 API・型一覧・依存・ライセンス |
| Q2 | T2 | docs.rs/pulp/latest | webfetch | 同上 + dispatch モデル確認 |
| Q3 | T3 | docs.rs/thermite/latest | webfetch | 同上 + バックエンド表 |
| Q4 | T2 | docs.rs/crate/pulp/latest/features | webfetch | AVX-512/nightly 要件の確認 |
| Q5 | T3 | github.com/novacrazy/thermite | webfetch | リポジトリ確認 → 404(改名を検出) |
| Q6 | T3 | docs.rs/crate/thermite/latest | webfetch | 正規リポジトリ raygon-renderer/thermite、バージョン履歴を確認 |
| Q7 | T3 | lib.rs/crates/thermite | webfetch | 採用実績・SLoC・edition 要件 |
| Q8 | T1 | github.com/Lokathor/wide | webfetch | 活動状況(465 commits, 558 stars) |
| Q9 | T2 | github.com/sarah-quinones/pulp | webfetch | 活動状況(250 commits, 369 stars) |
| Q10 | - | crates.io API (curl) | shell | 失敗 — curl が JSON を返さず webfetch に切替(負の証拠として記録) |
