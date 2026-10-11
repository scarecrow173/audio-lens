| ID | Status | Priority | Domain | Task | Evidence target | Dependency | Provenance |
|---|---|---|---|---|---|---|---|
| T1 | completed | 9 | docs | wide の最新版・機能・安定性 | crates.io + docs.rs | - | initial_query |
| T2 | completed | 9 | docs | pulp の最新版・機能・dispatch モデル | crates.io + docs.rs | - | initial_query |
| T3 | completed | 9 | docs | thermite の最新版・機能・メンテナンス状況 | crates.io + GitHub | - | initial_query |
| T4 | completed | 8 | github | 各クレートのリポジトリ活動(最終コミット・issue) | GitHub | T1-T3 | knowledge_gap |
| T5 | completed | 8 | docs | f64/AVX-512/実行時 dispatch の対応差を確認 | docs.rs API | T1-T3 | knowledge_gap |
| T6 | completed | 7 | general_web | AudioLens 用途(FFT/相関/bandpass/resample)との適合評価 | 内部知識+ドキュメント | T1-T5 | knowledge_gap |
