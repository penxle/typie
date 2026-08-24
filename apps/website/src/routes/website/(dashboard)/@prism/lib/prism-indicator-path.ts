export type PrismIndicatorPoint = Readonly<{
  x: number;
  y: number;
}>;

export type PrismIndicatorPath = Readonly<{
  p0: PrismIndicatorPoint;
  p3: PrismIndicatorPoint;
  arcHeight: number;
}>;

const easeOutTravel = (progress: number) => Math.sin((progress * Math.PI) / 2);

const requirePoint = (point: PrismIndicatorPoint) => {
  if (!Number.isFinite(point.x) || !Number.isFinite(point.y)) {
    throw new RangeError('Prism indicator path points must be finite.');
  }
};

export const createPrismIndicatorPath = (source: PrismIndicatorPoint, destination: PrismIndicatorPoint): PrismIndicatorPath => {
  requirePoint(source);
  requirePoint(destination);
  if (destination.x >= source.x || destination.y >= source.y) {
    throw new RangeError('Prism indicator destination must be above and left of its source.');
  }

  const distance = Math.hypot(source.x - destination.x, source.y - destination.y);
  const verticalDistance = source.y - destination.y;
  return {
    p0: { ...source },
    p3: { ...destination },
    arcHeight: Math.min(distance * 0.1, verticalDistance * 0.3, 24),
  };
};

export const samplePrismIndicatorPath = (path: PrismIndicatorPath, elapsedProgress: number): PrismIndicatorPoint => {
  if (!Number.isFinite(elapsedProgress)) throw new RangeError('Prism indicator path progress must be finite.');
  if (elapsedProgress <= 0) return path.p0;
  if (elapsedProgress >= 1) return path.p3;
  const progress = easeOutTravel(elapsedProgress);
  const lift = path.arcHeight * Math.sin(Math.PI * progress);
  return {
    x: path.p0.x + (path.p3.x - path.p0.x) * progress,
    y: path.p0.y + (path.p3.y - path.p0.y) * progress - lift,
  };
};
