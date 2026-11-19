---
id: "KNOW-gR9kXnPwTmVbYsHjQfLd"
type: "knowledge"
status: "approved"
title: "Ruviform コラボレーション: 依存解決・契約 API・消費者検証"
created: "2026-11-19"
updated: "2026-11-19"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Ruviform コラボレーション: 依存解決・契約 API・消費者検証

AudioLens は Ruviform の consumer であり、依存解決・契約 API の採用・pin 管理・消費者側検証の一連の運用パターンを確立した。本書は、Ruviform との境界を越える作業に関する知見をまとめる。

## Git 依存・pin 運用

- Ruviform は Git 依存 + `rev` pin で導入(AL#99 で `73190d7d…` を初回導入)。pin 更新は `node tools/update-ruviform-rev.mjs <full-sha>` で `cargo update --precise` を適用し、`mise run sync-ui-deps` で `ui/vendor/ruviform` を同期する。`check-ruviform-lock` で lockfile 単一 SHA を検証、`check-ruviform-rev` で manifest/lock/vendor 一致を検証する。
- lockfile 正本化(AL#140)で resolved revision の正本を `Cargo.toml` の `rev` 列挙から `Cargo.lock` の `source` fragment(`git+<url>#<sha>`)へ移行。14 件の ruviform git 依存から `rev` を除去し、URL と feature は保持。
- private アクセスは `net.git-fetch-with-cli` + `GIT_LFS_SKIP_SMUDGE` で system git へ委譲(GCM / CI Deploy Key で共通)。

## consumer oracle と契約 API 採用

`ruviform-audio-test` の deterministic fixture/oracle を独立 consumer oracle として `al-analysis` に実装(AL#127/RF#628)。期待値は upstream oracle 出力の f64 widen 統計量と解析的参照の 2 系統で、`pcm_assert_eq` は fixture 入力の妥当性確認限定で Pass 判定には使わない。

Ruviform 側の契約 API(RF#167-170)を採用して AL 側残留二重実装を置換した(AL#117):
- csolve→`linalg::complex_solve`(RF#167、複素 Gaussian 消去・部分 pivot・条件数 proxy を同一評価順、`to_bits()` bit-exact 検証)。
- 直算 corr0 sweep→`overlap_ncc_direct`(RF#168)。AL の「第1引数シフト」正 lag 意味論を `(ys,xs)` 順呼出で再現し、負 lag は `corr0` のまま。
- スペクトル統計→`ruviform_numeric::spec::*`(RF#169、centroid/bandwidth/rolloff/flatness/entropy/flux/ols_slope/median/argmax、`total>0`/`total<=0` 非対称 guard・NaN 処理・`ln(n)` 正規化・`total_cmp` argmax を逐語移植)。
- ResamplePlan 永続化→`ALGORITHM_ID`+`stable_id` 正準表現(RF#170)。旧内部係数フィールドは永続契約から撤去し、`NormalizedParams::validate()` は rate から plan 再構成して stable_id 一致検証。
- #117 stamp: 全 FeatureRecord.params に `{"rev": al_core::RUVIFORM_REV, "precision": "f64", "crates": [...]}` を永続化境界(`stage.rs` の `write_json` 直前)で一括適用。`al_core::RUVIFORM_REV` は `build.rs`+`rustc-env` で Cargo.lock 導出(AL#104)。

## 上流委譲と責務境界

- ruviform 側に責務がある問題を AL 側の恒久 workaround で隠さない —— 一時的な回避策には理由・解消条件・関連 Issue を明記する(AGENTS.md の規約)。
- 「意味論境界」の判定: 別用途ツールがそのまま使える処理→Ruviform、AL の解析判断を含む処理→AL。移行不能の理由を bit-exact 条件違反・API 不在・意味論差異に分類して契約提案(R1-R4→RF#167-170)へ接続する。
- 上流に API が無い規則は複製+テストで上流実出力に固定し、上流 Issue で差し替え条件を記録する運用(RF#443 の `spectrogram_columns` 複製は ruviform#443 OPEN のため維持)。
- 他リポジトリ API の引数順契約差異は「呼出引数の入替」で吸収し、対応をソースと突合する。
- 「plan 系 NCC は正規化方式が違うため直算 API に寄せられない」— 正規化実装(prefix-sum vs 逐次)の違いが bit-exact を壊す具体例。

## 設計上の教訓

- 「書き出し直前の一括 stamp」は params を途中層が上書きする場合の authoritative 適用点の確立パターン。
- rev ハードコードを `build.rs`+`rustc-env` で Cargo.lock 導出に置き換え、複数 rev 混在を build 失敗で検出する provenance 正本化。
- 「facade が担っていた Result→panic 写像」は、検証済み形状では panic しない契約への移行で撤去できる段階的 adapter 退役の型。
- 「正本を manifest 属性から lockfile fragment へ移す」際は契約ツールと manifest 変更を同一コミットにする。
- `float_roundtrip` 無効の serde_json 既定パーサは f64 を 1-ULP 丸める罠 —— 永続化値を生 f64 と直接比較するテストは失敗し得る。比較両辺を同一テキスト経路に揃えるか `float_roundtrip` 有効化で対処する。

関連 Issue/PR: AL#99、#100、#101、#102、#104、#106、#115、#117、#127、#131、#133、#138、#140、RF#150-157、RF#167-170、RF#219、RF#255、RF#256、RF#294、RF#334、RF#335、RF#443、RF#628、RF#631。
