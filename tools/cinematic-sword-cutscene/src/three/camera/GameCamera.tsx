import { useFrame, useThree } from "@react-three/fiber";
import type React from "react";
import { shotAt, type Timeline } from "../../data/shots";
import { useGameTime } from "../../utils/frameTime";
import { cameraAt } from "./CinematicCameraRig";

// Applies the master-timeline camera every rendered frame.
export const GameCamera: React.FC<{ timeline: Timeline }> = ({ timeline }) => {
  const camera = useThree((s) => s.camera);
  const t = useGameTime();

  useFrame(() => {
    const shot = shotAt(timeline, t);
    if (!shot) return;
    const pose = cameraAt(shot, t, timeline.impacts);
    camera.position.set(...pose.pos);
    camera.lookAt(...pose.look);
    const persp = camera as { fov?: number; updateProjectionMatrix?: () => void };
    if (persp.fov !== undefined && persp.fov !== pose.fov) {
      persp.fov = pose.fov;
      persp.updateProjectionMatrix?.();
    }
  });

  return null;
};
