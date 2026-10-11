# Running Summary

## Confirmed findings

- **wide 1.7.1** (2026-09, Lokathor): 固定幅 SIMD 型(f64x2〜f64x8 等)。safe_arch 経由で x86/x86_64/aarch64 NEON/wasm32 を intrinsic 化、その他は LLVM オートベクタリゼーション頼み。**実行時 dispatch 機構なし**(コンパイル時 target_feature に従う; `multiversion` 等と組み合わせが前提)。超越関数なし。最成熟(465 commits、558 stars、ドキュメント率100%)。
- **pulp 0.22.3** (sarah-quinones): `Arch::new().dispatch(WithSimd)` で **実行時 dispatch**。`Simd` トレイトに対してジェネリックに書く。c64xN 複素ベクトル型込み。AVX-512 は `x86-v4` feature = bytemuck/avx512_simd = **nightly 必須**。stable は x86-v3(AVX2+FMA)まで。faer の基盤として実績あり。MIT。
- **thermite 0.2.1** (2026-08, novacrazy → raygon-renderer): 2026-08 に9年ぶり全面書き換え。trait ラダー + `#[thermite::dispatch]` proc macro + `dispatch_dyn!` 実行時 dispatch。**ポリシー設定可能な超越関数ライブラリ**(exp/log 等、UltraPerformance〜Reference)。compress/expand、gather/scatter、interleave、fp16/bf16/fp8 パック、Dual(自動微分)/Compensated(double-double)。バックエンド: scalar/SSE2/SSE4.2/AVX2+FMA/NEON 完成、**AVX-512 未実装**、SPIR-V は壊れてると明記。採用実績ほぼなし(36 DL/月、被依存4クレート)。

## Contradictions / cautions

- thermite: 機能は最も豊富だが 0.2 系は2ヶ月前のリライトで実戦実績がほぼ無い
- pulp: API ドキュメント率 21% — 学習コストが高い可能性
- wide: dispatch は自力で組む必要がある(multiversion か自前 detect)。「f64x8 は AVX-512 専用幅だが、無ければ分割実行される」点に注意

## Remaining gaps

- 各クレートの厳密な MSRV(thermite は Rust 2024 edition なので比較的新しい toolchain が必要)
- thermite の安定版での AVX-512 状況(現在は未実装と明記)
- 実ベンチマーク差(未測定 — 必要ならプロトタイプで測る)

## Next

- 採用推奨の作成 → final-report.md
