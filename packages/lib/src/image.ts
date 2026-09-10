import { clamp } from 'remeda';

export const fitImageSize = ({
  width,
  height,
  maxWidth,
  maxHeight = Infinity,
  proportion = 1,
}: {
  width: number;
  height: number;
  maxWidth: number;
  maxHeight?: number;
  proportion?: number;
}): { width: number; height: number } => {
  const scale = Math.min(1, maxWidth / width, maxHeight / height) * clamp(proportion, { min: 0.1, max: 1 });
  return { width: width * scale, height: height * scale };
};
