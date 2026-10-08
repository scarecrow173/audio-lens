# Samples

動作確認用fixtureメディア。ユースケース(検証シナリオ)ごとの
サブディレクトリに整理し、各ディレクトリはそのシナリオに必要な
入力だけを持つ。

`*.wav` / `*.mp4` 等は `.gitattributes` により拡張子単位で Git LFS
管理。clone/pull には `git lfs` が必要。

以下のfixtureはすべて `tools/cinematic-sword-cutscene/` が生成する
決定論的な「ゲームキャプチャ風」映像(50 s / 640x360 / 30 fps /
AAC 音声の剣戟カットシーン)。映像は全バリアントで正常系と同一で、
異常はサウンドトラックのみに注入される。

## reference/

| ファイル | ロール | 内容 |
| --- | --- | --- |
| `game-capture-normal.mp4` | `capture_reference` | 正常系 50 s 本編 |
| `source-<name>.wav` | `source_asset` | 動画内で実際に鳴っている各音素材(voice 5 + sfx 13 + ambience + bgm)。出現位置・音量は `metadata/audio.json` の各イベント(`startFrame` / `volume`)がground truth |

## dropout/

| ファイル | ロール | 内容 |
| --- | --- | --- |
| `game-capture-audio-dropout.mp4` | `capture_bug` | 32.0-33.0 s 全音声が無音に落ちる |

## desync/

| ファイル | ロール | 内容 |
| --- | --- | --- |
| `game-capture-audio-desync.mp4` | `capture_bug` | 音声全体が +0.25 s 遅れる(AV同期ずれ) |

## clipping/

| ファイル | ロール | 内容 |
| --- | --- | --- |
| `game-capture-audio-clip.mp4` | `capture_bug` | 全体 +14 dB デジタルクリッピング |

## reverb/

| ファイル | ロール | 内容 |
| --- | --- | --- |
| `game-capture-audio-reverb.mp4` | `capture_bug` | マスターバスに意図しない残響 |

## filter/

| ファイル | ロール | 内容 |
| --- | --- | --- |
| `game-capture-audio-filter.mp4` | `capture_bug` | 全体に 700 Hz ローパス(籠り) |

## clicks/

| ファイル | ロール | 内容 |
| --- | --- | --- |
| `game-capture-audio-clicks.mp4` | `capture_bug` | 8-50 s にバッファアンダーラン風のプチノイズ |

## test/

| ファイル | 内容 |
| --- | --- |
| `game-capture-test.mp4` | 10 s のアニメーション / VFX / 音声 技術テスト |

## metadata/

バリエーション・バグ注入位置・音声イベントの機械可読メタデータ
(`<variant>.json` と音声イベント一覧 `audio.json`)。

## 動作確認コマンド

```bash
# バグ動画 vs 正常動画(例: dropout)
target/release/al run --base-dir samples/_runs \
  --input capture_bug:samples/dropout/game-capture-audio-dropout.mp4,\
capture_reference:samples/reference/game-capture-normal.mp4,\
source_asset:samples/reference/source-bgm.wav
```

`samples/_runs/` は gitignore 済み。各ジョブは
`samples/_runs/<dir>/<JOB-*>/{manifest.json, analysis/, export/}` に
出力される。
