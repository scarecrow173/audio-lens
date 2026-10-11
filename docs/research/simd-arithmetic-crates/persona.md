# Persona / Intent Model

- User objective: AudioLens 独自算術クレートの基盤として wide / pulp / thermite を比較し採用判断する
- Audience: AudioLens 開発者(ユーザー本人)
- Must-cover aspects: stable/nightly 要件、f64 サポート、命令セットカバレッジ(SSE/AVX/AVX2/AVX-512/NEON)、実行時 dispatch、メンテナンス状況、ライセンス、DSP 用途向け API(超越関数・aligned load・gather 等)
- Nice-to-have aspects: パフォーマンス実績、他クレート(faer/rustfft)との親和性、MSRV
- Recency requirement: 現在の crates.io 最新版の状態(2026-10 時点)
- Source quality bar: docs.rs / crates.io / GitHub リポジトリを primary とする
- Known constraints: AudioLens は f64 主体・stable Rust・依存追加は承認制・決定性が重要
- Unknowns: thermite の現メンテナンス状況、各クレートの AVX-512 対応度
- Steering preference: 最終的な採用推奨を含めて提示する
