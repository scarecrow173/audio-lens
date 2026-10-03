# Samples

動作確認用の合成メディア。音声はすべて 48 kHz / 16-bit PCM、
`cargo run -p al-cli --example gen_samples -- samples/` で
シード固定の決定的信号として再生成できる(実マテリアル非含有)。

`*.wav` / `*.avi` 等は `.gitattributes` により拡張子単位で Git LFS
管理。clone/pull には `git lfs` が必要。

## ファイル

### WAV フィクスチャ

| ファイル | パターン | 期待される検出 |
| --- | --- | --- |
| `capture-tone-440hz.wav` | 1.0 s クリーン 440 Hz 正弦波(capture 単独) | イベントなし・ベースライン特徴 |
| `source-noise-burst.wav` + `capture-embedded.wav` | 4096 frame のノイズ source を capture の 9600 frame 目(0.2 s)に 0.5 ゲインで埋め込み + 背景ノイズ | locate 受理(capture_range.start≈9600)、reference/expensive 解析、残差生成 |
| `capture-clipped.wav` | フルスケール超過 1 kHz 正弦波(i16 飽和) | `clipping` イベント |
| `capture-dropout.wav` | 500 Hz 正弦波の 0.5-0.8 s を無音化 | `silence_dropout_candidate` イベント |
| `source-stereo-pair.wav` + `capture-stereo-mixed.wav` | ステレオ source を 3 s capture の 1.0 s 目に L=0.6 / R=-0.3(極性反転)で混入 | locate 受理 + 符号付き混合行列 |

### 動画フィクスチャ(非圧縮 AVI + PCM 音声)

ゲームプレイ解析のユースケースを模した 3 s / 10 fps / 128×96 の
合成動画。映像は BGR24 非圧縮(縦グラデーション + 掃引バー +
フラッシュフレーム)、音声は PCM s16 で BGM アセットを内蔵する。

| ファイル | ロール | 内容 | 期待される検出 |
| --- | --- | --- | --- |
| `source-bgm.wav` | `source_asset` | 元音源 BGM(2 s 和音フレーズ) | — |
| `video-gameplay-normal.avi` | `capture_reference` | 正常動画。BGM を 0.4 s 目から 0.5 ゲインで内蔵 | locate 受理(start≈19200) |
| `video-gameplay-bug.avi` | `capture_bug` | 同一レシピ + 1.0-1.5 s ドロップアウト + 2.2-2.4 s クリップ | locate 受理 + `clipping` / `silence_dropout_candidate` 等のイベント |
| `video-gameplay-run2.avi` | `capture_reference` 等 | 同一セッションの別テイク。BGM 位置(0.6 s)・ゲイン(0.45)・テンポ(約 +1%)・ノイズ床・映像掃引速度がすべて微妙に異なる | 元 BGM との locate は low confidence で reject(テンポ差により完全一致しないためアライメントを強制しない) |

動画コンテナのサポート範囲は **レガシー AVI のみ**(PCM 音声 +
宣言メタデータとしての非圧縮映像)。映像フレームの内容は解析せず、
コーデック・解像度・フレーム数・公称 fps を `metadata.json` の
`video` セクションに記録する。加えて、`movi` のパケット出現順と
`strh` クロックから導出したストリーム別タイミング観測(音声
パケット位置・映像フレーム位置・間隔・不連続候補)を `av_timing`
セクションに記録する。レガシー AVI は明示的な PTS を持たないため、
これは mux 順由来の一次観測であり CFR も仮定しない。音声・映像が
複数ある場合は先頭ストリームのみを採用する。MP4/H.264 等の圧縮
形式は未対応で、`decode/input unavailable` として明示的に失敗
する。

## 動作確認コマンド

```bash
# 再生成
cargo run -p al-cli --example gen_samples -- samples/

# capture 単独(cheap → イベント → dataset → 可視化 → export)
target/release/al run --base-dir samples/_runs \
  --input capture_bug:samples/capture-tone-440hz.wav \
  --stages ingest,cheap_analysis,expensive_analysis,dataset,visualization,export

# source + capture フルパイプライン(locate 含む)
target/release/al run --base-dir samples/_runs \
  --input capture_bug:samples/capture-embedded.wav,source_asset:samples/source-noise-burst.wav \
  --stages ingest,locate,cheap_analysis,reference_analysis,expensive_analysis,dataset,visualization,export
```

### 動画ユースケース

```bash
# ケース A: バグ動画 vs 正常動画 + 元音源(フルパイプライン)
target/release/al run --base-dir samples/_runs \
  --input capture_bug:samples/video-gameplay-bug.avi,\
capture_reference:samples/video-gameplay-normal.avi,\
source_asset:samples/source-bgm.wav

# ケース B: バグ動画 vs 非同一リプレイ動画 + 元音源
#          → run2 の locate はテンポ差で reject(low confidence)される
target/release/al run --base-dir samples/_runs \
  --input capture_bug:samples/video-gameplay-bug.avi,\
capture_reference:samples/video-gameplay-run2.avi,\
source_asset:samples/source-bgm.wav

# ケース C: バグ動画単独(ソースなし、異常検出のみ)
target/release/al run --base-dir samples/_runs \
  --input capture_bug:samples/video-gameplay-bug.avi
```

`samples/_runs/` は gitignore 済み。各ジョブは
`samples/_runs/<dir>/<JOB-*>/{manifest.json, analysis/, export/}` に
出力される。
