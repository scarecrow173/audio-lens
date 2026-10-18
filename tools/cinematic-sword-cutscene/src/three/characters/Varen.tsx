import { forwardRef } from "react";
import { KAYKIT } from "../../data/kaykitManifest";
import type { CharacterTrack } from "../../data/shots";
import { type CharacterHandle, KayKitCharacter } from "./KayKitCharacter";

// VAREN — Knight body, heavy 2H sword, red identity.
export const Varen = forwardRef<CharacterHandle, { track: CharacterTrack }>(({ track }, ref) => (
  <KayKitCharacter
    ref={ref}
    manifest={KAYKIT.knight}
    track={track}
    showNodes={["2H_Sword"]}
    embeddedTipY={1.85}
    rimColor="#ff2b3a"
  />
));
