# cinematic-sword-cutscene

`samples/` の「ゲームキャプチャ風」fixtureを生成するプロジェクト。
Remotion + Three.js(React Three Fiber)で KayKit Character Pack:
Adventurers(CC0)のキャラクター / アニメーションを駆動し、輝度変化・
フレーム差分・明確なシーンチェンジを持つ剣戟カットシーンを
50 s / 640x360 / 30 fps でレンダリングする。

正常系(`GameCaptureNormal`)がマスタータイムライン
(`src/data/masterTimeline.ts`)をそのまま再生する。異常系は2系統:

- 映像バグ: `BugProfile` のフレームリマップ / ポストプロセスとして
  非破壊注入(`src/bugs/bugs.ts`)
- 音声バグ: プリミックスしたサウンドトラックへの deterministic
  DSP変換(`AudioBugProfile`, `src/bugs/audioBugs.ts`)

出力は `samples/<scenario>/` に集約される(バグ種別ごとの
サブディレクトリ + `samples/reference/` + `samples/metadata/`)。
`gen-audio.ts` は動画内で鳴っている音素材(source_asset)も
`samples/reference/source-*.wav` として同梱する
(イベント単位の個別素材)。locate で素材の出現位置を特定し、
capture 内の音が素材からどう変化したかを解析する用途。

## 再生成

```bash
cd tools/cinematic-sword-cutscene
npm install
npm run audio:gen          # SE/Ambience/BGM + バグミックスを procedural 生成
npm run audio:voice        # Voice を Chatterbox で生成(要 uv/python)
npm run render:variants    # test / normal / 音声バグ6種を順にレンダリング
```

デバッグ表示付きプレビューは `GameCaptureBugPreview` composition
(`debug` prop)。任意のバグプロファイルは `--props '{"bug":{...}}'`
で `remotion render` に渡せる。

## 音声パイプライン

配置は `src/data/audioTimeline.ts` が唯一の参照元で、
`src/audio/AudioTrack.tsx` が `<Sequence>/<Audio>` へ展開する。

音声バグ系は映像を正常系のまま、サウンドトラックのみを差し替える。
全イベントを deterministic にプリミックスした `mix-normal.wav` を
基準に、`gen-bug-audio.ts` が各バグをDSP変換として適用した
`mix-<variant>.wav` を生成する(クリッピング等の非線形処理は
イベント単位では再現できないためプリミックス方式)。バリアントは
`AUDIO_BUG_VARIANTS` で宣言的に定義される:

| バリアント | samples dir | 異常 |
| --- | --- | --- |
| `audio-dropout` | `dropout/` | 32.0-33.0 s 全音声が無音に落ちる |
| `audio-desync` | `desync/` | 音声全体が +0.25 s 遅れる |
| `audio-clip` | `clipping/` | 全体 +14 dB デジタルクリッピング |
| `audio-reverb` | `reverb/` | マスターバスに意図しない残響 |
| `audio-filter` | `filter/` | 全体に 700 Hz ローパス(籠り) |
| `audio-clicks` | `clicks/` | 8-50 s にバッファアンダーラン風のプチノイズ |

| 種別 | 生成 | 出力 |
| --- | --- | --- |
| Voice | Chatterbox Multilingual (`gen_voice.py`) + DSP ポスト処理 | `assets/audio/generated/voice/` |
| SE | procedural DSP (`scripts/audio/gen-sfx.ts`) | `assets/audio/generated/sfx/` |
| Ambience | procedural DSP(24 s シームレスループ) | `assets/audio/generated/ambience/` |
| BGM | procedural synth 50 s (`gen-bgm.ts`) | `assets/audio/generated/bgm/` |
| バグミックス | `gen-bug-audio.ts`(プリミックス + DSP変換) | `assets/audio/generated/master/` |

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
startFrame を `samples/metadata/audio.json` に記録する。

KayKit アセットは CC0 (KayKit Character Pack: Adventurers)。
