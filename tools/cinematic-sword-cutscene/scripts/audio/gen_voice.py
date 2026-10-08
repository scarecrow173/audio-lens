# /// script
# requires-python = ">=3.10,<3.13"
# dependencies = ["chatterbox-tts==0.1.7"]
# ///
"""Generate Japanese dialogue lines with Chatterbox Multilingual (MIT).

Run:  uv run scripts/audio/gen_voice.py

Raw model output (24kHz mono) lands in
assets/audio/generated/voice/raw/<id>.wav — character processing is a
separate deterministic DSP step (postprocess-voice.ts).

Chatterbox TTS is MIT licensed (Resemble AI); model weights are fetched
from HuggingFace on first run and cached locally. No cloud API calls.
"""
import sys
from pathlib import Path

import torch
import torchaudio
import perth
from chatterbox.mtl_tts import ChatterboxMultilingualTTS

# resemble-perth's implicit watermarker is optional and fails to import on
# some platforms (None instead of a class). These fixture clips don't need
# watermark embedding — fall back to the dummy watermarker.
if perth.PerthImplicitWatermarker is None:
    perth.PerthImplicitWatermarker = perth.DummyWatermarker

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets" / "audio" / "generated" / "voice" / "raw"
OUT.mkdir(parents=True, exist_ok=True)

# id, text, seed, generation params
LINES = [
    # KAEL — calm, restrained delivery
    ("kael_01", "……まだ終わっていない。", 7001,
     dict(exaggeration=0.35, cfg_weight=0.5, temperature=0.6)),
    ("kael_02", "お前をここで止める。", 7003,
     dict(exaggeration=0.4, cfg_weight=0.5, temperature=0.6)),
    # VAREN — heavier, more pressured delivery
    ("varen_01", "終わっていない？いや、もう決して戻れない。", 7002,
     dict(exaggeration=0.55, cfg_weight=0.55, temperature=0.7)),
    ("varen_02", "なら見せてみろ。", 7004,
     dict(exaggeration=0.5, cfg_weight=0.55, temperature=0.7)),
    ("varen_03", "その程度か。", 7005,
     dict(exaggeration=0.55, cfg_weight=0.55, temperature=0.7)),
]


def main() -> None:
    device = "cuda" if torch.cuda.is_available() else "cpu"
    model = ChatterboxMultilingualTTS.from_pretrained(device=device)
    for vid, text, seed, params in LINES:
        torch.manual_seed(seed)
        wav = model.generate(text, language_id="ja", **params)
        path = OUT / f"{vid}.wav"
        torchaudio.save(str(path), wav.cpu(), model.sr)
        print(f"wrote {path}", flush=True)


if __name__ == "__main__":
    sys.exit(main())
