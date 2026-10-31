# Pre-migration Baseline (Issue #98)

Issue #95 の Ruviform 移行において、移行前後を客観比較するための現行
AudioLens の golden・実メディア・性能 baseline。既存アルゴリズムの正当性を
主張するものではなく、**観測された挙動をそのまま固定する**記録である。

## 前提(重要)

- **旧データ再読込互換は要求しない**(Pre-release 方針)。形式や内部表現が
  変わった場合は本 baseline の手順で成果物を**再生成**して比較する。
- 値は全て実測。測れなかった項目は `coverage.json` と本ファイルで
  **「実測未実施」** と明記しており、捏造していない。
- bit-exact が成立するのは **同一 commit・同一 toolchain・同一 CPU SIMD
  レベル** のときだけ。al-math は pulp 実行時 dispatch のため、CPU が
  変われば浮動小数の演算順序が変わり、tolerance 面でも差が出うる。

## レイアウト

```
baseline/
  environment.json   commit SHA / rustc / OS / CPU / ffmpeg / dirty 状態
  inputs.json        入力ファイルの SHA-256 と派生 AVI の生成コマンド
  config.json        run 行列定義(入力・役割・固定 input-ids・stage)
  contracts.json     数値契約: bit-exact / tolerance / record-only の分類
  coverage.json      Issue #98 列挙項目 → e2e/既存テスト/ガード/未実施 の台帳
  bench/results.json alloc 計測と既存ベンチの記録
  runs/<name>/
    manifest.json           生 manifest(record-only 証跡)
    manifest.normalized.json ID/絶対パスを正規化した比較面
    artifacts.index.json    全成果物の path/size/sha256/クラス索引
    golden/                 normalized JSON・table row・tensor/payload 等
      payloads/             tolerance 面の decode 値または fingerprint
    performance.json        e2e wall / CPU / peak RSS / stage wall / 生成物 bytes
    determinism.json        aa run 等の決定性記録
    inspect.json            al inspect 出力(record-only 証跡)
```

run 行列(`config.json` 参照): `canonical`, `variant-desync/-clipping/
-reverb/-filter/-clicks`, `small`, `resume`, `aa`。

## 比較契約(contracts.json)

- **bit-exact**: 整数・enum・決定・PCM payload・path 集合・主要 artifact
  メタデータ。path/size/SHA-256 が一致すること。
- **tolerance**: f64 解析値(STFT/CSD/coherence/band_gain/lag/offset/残差等)。
  `|a-b| <= max(abs_tol, rel_tol * max(|a|,|b|))`。非有限値は
  `{"$nonfinite":"NaN"|"Infinity"|"-Infinity"}` センチネルで区別し、
  `null` に潰さない。小 payload は全値、大 payload は
  `{count,min,max,sum}` fingerprint で保持する。
- **record-only**: 環境依存の詳細・可視化成果物・ログ・video_thumbs 等。
  値は観測のため保持するが、比較では `recordOnlyFields` 指定の範囲のみ
  確認し、値そのものは合否に使わない。

## 再現手順

環境は `environment.json` の commit/toolchain を合わせる。

```bash
mise install          # toolchain 復元(rust/node/biome/...)
mise run setup        # apm 依存 + git lfs install/pull(実メディア実体化)
mise exec -- cargo build --release -p al-cli   # al CLI 構築
```

1. 派生 AVI を生成する。`config.json` の `derivedInputs` に従い、各
   `<source>.mp4` から `samples/_runs/decoded/<name>.avi` を作る:

   ```bash
   # 例: ffmpeg -y -i <source>.mp4 -r:v 10 -s 160x90 -c:v rawvideo \
   #   -pix_fmt bgr24 -c:a pcm_s16le -ar 48000 <out>.avi
   node tools/baseline/capture.mjs --derive-only   # 自動生成
   ```

   ffmpeg は repo toolchain 外(派生 AVI 生成にのみ使用・比較契約外)。

2. run 行列を実行して baseline を収集する:

   ```bash
   node tools/baseline/capture.mjs --matrix        # 全 run を実行+収集
   node tools/baseline/capture.mjs --run canonical # 1 件だけ
   ```

   `resume` run は全 stage plan で起動し `locate` 完走境界で kill、
   `al resume --job-id` で完走させる(部分 stage plan + resume は不可)。

3. 比較(移行後の検証):

   ```bash
   node tools/baseline/compare.mjs <baseline_dir> <candidate_dir> \
     --contracts baseline/contracts.json
   node tools/baseline/compare.mjs --self-test     # 比較器自身の検証
   ```

4. alloc ベンチ:

   ```bash
   mise exec -- cargo run --release -p al-analysis --example bench_alloc
   ```

## 測定範囲と「実測未実施」

測定したもの:
- run 単位 e2e wall time(実測)、CPU(user/kernel, 1s ポーリング)、peak RSS
- stage 別 wall time(manifest `duration_ms` 由来)
- 生成物サイズ(artifact index の bytes 集計 → I/O 書き込み量の推定)
- al-analysis 主経路の alloc 回数/bytes(`bench_alloc.rs`)

**実測未実施**(理由は `coverage.json` / `bench/results.json` 参照):
- stage 別 CPU/RSS/IO: 外部ポーリングでは stage 帰属ができず計装が必要
- 実 I/O カウンタ: 生成物 bytes で推定のみ
- criterion ベンチ全件の本環境実測値: harness は温存・再実行方法を記録

## ガードテスト

`crates/al-math/tests/edge_inputs.rs`(本 Issue で追加)は、NaN 伝播・無音・
空/短入力に対する al-math 公開 API の**観測された挙動**を固定する。正しさの
主張ではなく、移行で挙動が変わったことを検知するための観測契約である。
