# Evidence Ledger

| Source ID | URL/Location | Title | Publisher | Date | Type | Reliability | Key evidence | Supports | Limitations |
|---|---|---|---|---|---|---|---|---|---|
| S1 | docs.rs/wide/latest | wide 1.7.1 | Lokathor | 2026-09-14 | official docs | primary | 固定幅型 f64x2/x4/x8 等、safe_arch ベース、bytemuck 依存、Zlib/Apache/MIT、ドキュメント率100%、runtime dispatch 機構なし、超越関数なし(sqrt は std feature) | T1,T5 | 実行時挙動はコンパイル時 target_feature 依存 — ソース未精読 |
| S2 | docs.rs/pulp/latest | pulp 0.22.3 | sarah-quinones | latest | official docs | primary | `Arch::new().dispatch` 実行時 dispatch、`Simd` トレイトに write-once、c64xN 複素型あり、scalar フォールバック、MIT | T2,T5 | ドキュメント率 21% — API 詳細はソース読みが必要 |
| S3 | docs.rs/crate/pulp/latest/features | pulp features | docs.rs | latest | official docs | primary | default = std+x86-v3(AVX2+FMA)+relaxed-simd。`x86-v4`(AVX-512)は bytemuck/avx512_simd 経由で **nightly 必須**、`nightly` feature も存在 | T5 | - |
| S4 | docs.rs/thermite/latest | thermite 0.2.1 | novacrazy | 2026-08-13 | official docs | primary | trait ラダー(GenericVector→NumericVector→FloatVector+math traits)、`#[dispatch]` proc macro、`dispatch_dyn!` runtime dispatch、ポリシー設定可能な超越関数(UltraPerformance..Reference)、compress/expand、gather/scatter、interleave、Dual(自動微分)/Compensated(double-double)、SPIR-V バックエンド(壊れていると明記) | T3 | 0.2.x は 2026-08 の全面書き換え — 新しい |
| S5 | docs.rs/crate/thermite/latest | thermite crate page | docs.rs | 2026-08-13 | official metadata | primary | リポジトリ=raygon-renderer/thermite、0.1.0(2016)→0.2.0(2026-08-11)の9年ぶり復活、ビルド時間 ~1m25s、ソース4MB | T3,T4 | - |
| S6 | lib.rs/crates/thermite | thermite on lib.rs | lib.rs | 2026-08 | package metadata | medium | 36 DL/月、依存クレート4、69K SLoC、Rust 2024 edition、MIT/Apache | T4 | 採用実績が非常に少ない |
| S7 | github.com/Lokathor/wide | wide repo | GitHub | current | source repo | primary | 558 stars, 465 commits, safe_arch/safe 実装、「near drop-in replacement for std::simd」、他アーキは LLVM 任せ | T1,T4 | - |
| S8 | github.com/sarah-quinones/pulp | pulp repo | GitHub | current | source repo | primary | 369 stars, 250 commits, README に dispatch/WithSimd/with_simd 例 | T2,T4 | - |
| S9 | thermite docs Backends 表 | S4 内 | novacrazy | 2026-08 | official docs | primary | scalar/SSE2/SSE4.2/AVX2+FMA/NEON=Complete、**AVX-512 未実装**(AVX2 で動作)、wasm=opt-in、spirv=incomplete | T3,T5 | - |
