# Audit

## Coverage
- [x] 3 クレートの stable/nightly 要件 — 全て確認(全て stable で動作、pulp AVX-512 のみ nightly)
- [x] f64 サポート — 3 者とも f64xN 型あり
- [x] ISA カバレッジ — 確認済み(AVX-512 は thermite=未実装 / pulp=nightly / wide=型のみ)
- [x] 実行時 dispatch — pulp=yes, thermite=yes, wide=no(自前)
- [x] メンテナンス状況 — wide=活発(2026-09), pulp=活発, thermite=復活直後(2026-08)
- [x] ライセンス — 全て permissive(MIT/Apache/Zlib)
- [x] DSP 用途適合 — transcendentals(thermite), c64(pulp), 構造簡素さ(wide)

## Evidence
- [x] 事実関係は docs.rs / GitHub / lib.rs の primary ソースのみで構成
- [x] 最新情報は 2026 年のリリースで確認
- [x] マイナス面(thermite 採用実績、pulp ドキュメント率、wide の dispatch 欠落)も記録

## Contradictions
- thermite のドキュメント内に SPIR-V バックエンドの自己言及的な破損警告あり → 現在は CPU 用途のみと解釈

## Search Quality
- [x] 公式ソース優先
- [x] 実績情報(lib.rs の download 数・被依存数)を併記
- [x] crates.io API の失敗は負の証拠として記録(影響なし)

## User Alignment
- [x] AudioLens の制約(stable、f64 主体、決定性、依存承認制)に沿った評価軸で比較
- [x] 採用推奨を含める
