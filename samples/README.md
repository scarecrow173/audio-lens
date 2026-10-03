# Samples

動作確認用の合成 WAV。すべて 48 kHz / 16-bit PCM、
`cargo run -p al-cli --example gen_samples -- samples/` で
シード固定の決定的信号として再生成できる(実マテリアル非含有)。

`.wav` は `.gitattributes` により Git LFS 管理。clone/pull には
`git lfs` が必要。

## ファイル

| ファイル | パターン | 期待される検出 |
| --- | --- | --- |
| `capture-tone-440hz.wav` | 1.0 s クリーン 440 Hz 正弦波(capture 単独) | イベントなし・ベースライン特徴 |
| `source-noise-burst.wav` + `capture-embedded.wav` | 4096 frame のノイズ source を capture の 9600 frame 目(0.2 s)に 0.5 ゲインで埋め込み + 背景ノイズ | locate 受理(capture_range.start≈9600)、reference/expensive 解析、残差生成 |
| `capture-clipped.wav` | フルスケール超過 1 kHz 正弦波(i16 飽和) | `clipping` イベント |
| `capture-dropout.wav` | 500 Hz 正弦波の 0.5-0.8 s を無音化 | `silence_dropout_candidate` イベント |
| `source-stereo-pair.wav` + `capture-stereo-mixed.wav` | ステレオ source を 3 s capture の 1.0 s 目に L=0.6 / R=-0.3(極性反転)で混入 | locate 受理 + 符号付き混合行列 |

## 動作確認コマンド

```bash
# 再生成
cargo run -p al-cli --example gen_samples -- samples/

# capture 単独(cheap → イベント → dataset → 可視化 → export)
target/debug/al run --base-dir samples/_runs \
  --input capture_bug:samples/capture-tone-440hz.wav \
  --stages ingest,cheap_analysis,expensive_analysis,dataset,visualization,export

# source + capture フルパイプライン(locate 含む)
target/debug/al run --base-dir samples/_runs \
  --input capture_bug:samples/capture-embedded.wav,source_asset:samples/source-noise-burst.wav \
  --stages ingest,locate,cheap_analysis,reference_analysis,expensive_analysis,dataset,visualization,export
```

`samples/_runs/` は gitignore 済み。各ジョブは
`samples/_runs/<JOB-*>/{manifest.json, analysis/, export/}` に出力される。
