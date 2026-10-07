# Samples

動作確認用の合成メディア。音声はすべて 48 kHz / 16-bit PCM、
`cargo run -p al-cli --example gen_samples -- samples/` で
シード固定の決定的信号として再生成できる(実マテリアル非含有)。

`*.wav` / `*.avi` 等は `.gitattributes` により拡張子単位で Git LFS
管理。clone/pull には `git lfs` が必要。

ファイルはユースケース(検証シナリオ)ごとのサブディレクトリに
整理する。各ディレクトリは、そのシナリオに必要な入力だけを持つ。

## tone/

| ファイル | パターン | 期待される検出 |
| --- | --- | --- |
| `capture-tone-440hz.wav` | 1.0 s クリーン 440 Hz 正弦波(capture 単独) | イベントなし・ベースライン特徴 |

## clipping/

| ファイル | パターン | 期待される検出 |
| --- | --- | --- |
| `capture-clipped.wav` | フルスケール超過 1 kHz 正弦波(i16 飽和) | `clipping` イベント |

## dropout/

| ファイル | パターン | 期待される検出 |
| --- | --- | --- |
| `capture-dropout.wav` | 500 Hz 正弦波の 0.5-0.8 s を無音化 | `silence_dropout_candidate` イベント |

## locate-mono/

モノラル source + capture の位置特定ペア。

| ファイル | パターン | 期待される検出 |
| --- | --- | --- |
| `source-noise-burst.wav` + `capture-embedded.wav` | 4096 frame のノイズ source を capture の 9600 frame 目(0.2 s)に 0.5 ゲインで埋め込み + 背景ノイズ | locate 受理(capture_range.start≈9600)、reference/expensive 解析、残差生成 |

## locate-stereo/

ステレオ混合の位置特定ペア。

| ファイル | パターン | 期待される検出 |
| --- | --- | --- |
| `source-stereo-pair.wav` + `capture-stereo-mixed.wav` | ステレオ source を 3 s capture の 1.0 s 目に L=0.6 / R=-0.3(極性反転)で混入 | locate 受理 + 符号付き混合行列 |

## video/

ゲームプレイ解析のユースケースを模した動画一式(非圧縮 AVI +
PCM 音声)。映像は BGR24 非圧縮(縦グラデーション + 掃引バー +
フラッシュフレーム)、音声は PCM s16 で BGM アセットを内蔵する。
3 s / 10 fps / 128×96。同一ファイル群で 3 つのケース(A/B/C)を
実行できる。

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
セクションに記録する。各エントリは導出済みの per-packet
タイムスタンプ列(`start_tick` = `dwStart` 由来の初期オフセット +
`movi_order_timestamps`)も保持し、metadata だけから各パケット /
フレームのストリーム時刻をアドレスできる。レガシー AVI は明示的な
PTS を持たないため、これは mux 順由来の一次観測であり CFR も
仮定しない。音声・映像が複数ある場合は先頭ストリームのみを
採用する。MP4/H.264 等の圧縮
形式は未対応で、`decode/input unavailable` として明示的に失敗
する。

## 動作確認コマンド

```bash
# 再生成
cargo run -p al-cli --example gen_samples -- samples/

# capture 単独(cheap → イベント → dataset → 可視化 → export)
target/release/al run --base-dir samples/_runs \
  --input capture_bug:samples/tone/capture-tone-440hz.wav \
  --stages ingest,cheap_analysis,expensive_analysis,dataset,visualization,export

# source + capture フルパイプライン(locate 含む)
target/release/al run --base-dir samples/_runs \
  --input capture_bug:samples/locate-mono/capture-embedded.wav,source_asset:samples/locate-mono/source-noise-burst.wav \
  --stages ingest,locate,cheap_analysis,reference_analysis,expensive_analysis,dataset,visualization,export
```

### 動画ユースケース

```bash
# ケース A: バグ動画 vs 正常動画 + 元音源(フルパイプライン)
target/release/al run --base-dir samples/_runs \
  --input capture_bug:samples/video/video-gameplay-bug.avi,\
capture_reference:samples/video/video-gameplay-normal.avi,\
source_asset:samples/video/source-bgm.wav

# ケース B: バグ動画 vs 非同一リプレイ動画 + 元音源
#          → run2 の locate はテンポ差で reject(low confidence)される
target/release/al run --base-dir samples/_runs \
  --input capture_bug:samples/video/video-gameplay-bug.avi,\
capture_reference:samples/video/video-gameplay-run2.avi,\
source_asset:samples/video/source-bgm.wav

# ケース C: バグ動画単独(ソースなし、異常検出のみ)
target/release/al run --base-dir samples/_runs \
  --input capture_bug:samples/video/video-gameplay-bug.avi
```

`samples/_runs/` は gitignore 済み。各ジョブは
`samples/_runs/<dir>/<JOB-*>/{manifest.json, analysis/, export/}` に
出力される。

## cinematic-sword-cutscene/

Remotion + Three.js(React Three Fiber)で生成する、決定論的な
「ゲームキャプチャ風」映像フィクスチャ群。KayKit Character Pack:
Adventurers(CC0)のキャラクター / アニメーションを使用し、
輝度変化・フレーム差分・明確なシーンチェンジを持つ 50 s /
640x360 / 30 fps の剣戟カットシーンをレンダリングする。

正常系(`GameCaptureNormal`)がマスタータイムライン
(`src/data/masterTimeline.ts`)をそのまま再生し、異常系は
`BugProfile` のフレームリマップ / ポストプロセスとして非破壊的に
注入されるため、バグ区間以外のピクセルは正常系と一致する。

| ファイル | ロール | 内容 |
| --- | --- | --- |
| `out/normal/game-capture-normal.mp4` | `capture_reference` | 正常系 50 s 本編 |
| `out/test/game-capture-test.mp4` | 技術テスト | 10 s のアニメーション / VFX 検証 |
| `out/metadata/*.json` | — | バリエーション・バグ注入位置の機械可読メタデータ |

再生成:

```bash
cd samples/cinematic-sword-cutscene
npm install
npm run audio:gen          # SE/Ambience/BGM を procedural 生成
npm run audio:voice        # Voice を Chatterbox で生成(要 uv/python)
npm run render:variants    # test / normal を順にレンダリング
```

デバッグ表示付きプレビューは `GameCaptureBugPreview` composition
(`debug` prop)。任意のバグプロファイルは `--props '{"bug":{...}}'`
で `remotion render` に渡せる。

### 音声パイプライン

配置は `src/data/audioTimeline.ts` が唯一の参照元で、
`src/audio/AudioTrack.tsx` が `<Sequence>/<Audio>` へ展開する。
バグプロファイルは映像フレームのみ remap するため、音声トラックは
全バリアントで同一(音声は正常のまま維持される)設計。

| 種別 | 生成 | 出力 |
| --- | --- | --- |
| Voice | Chatterbox Multilingual (`gen_voice.py`) + DSP ポスト処理 | `assets/audio/generated/voice/` |
| SE | procedural DSP (`scripts/audio/gen-sfx.ts`) | `assets/audio/generated/sfx/` |
| Ambience | procedural DSP(24 s シームレスループ) | `assets/audio/generated/ambience/` |
| BGM | procedural synth 50 s (`gen-bgm.ts`) | `assets/audio/generated/bgm/` |

Voice は Resemble AI **Chatterbox Multilingual TTS**(`chatterbox-tts`
0.1.7, **MIT License**)を `language_id="ja"` でローカル実行。
モデルウェイトは初回実行時に HuggingFace から取得されローカルに
キャッシュされる(実行時のクラウドAPI依存はない)。Perth 透かしの
オプションモジュールが環境により import できない場合は
DummyWatermarker へフォールバックする(fixture用途のため透かし不要)。
KAEL は +0.5st相当の明るめ・ドライ、VAREN は -2.8st相当の低音強化・
小部屋リバーブで声色を分離(いずれも特定人物の声の模倣ではなく
モデル標準音声+DSPチェーン)。SE/Ambience/BGM は生成AIを使わず
seeded oscillator/noise/filter のみで合成し、seed・generator・
startFrame を `out/metadata/audio.json` に記録する。
