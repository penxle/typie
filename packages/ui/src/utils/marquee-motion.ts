const PIXELS_PER_SECOND = 48;
const RAMP_DURATION_MS = 200;
const MAX_FRAME_DURATION_MS = 64;
const INTEGRATION_STEP_MS = 1;

type AdvanceMarqueeMotionOptions = {
  position: number;
  velocity: number;
  maximum: number;
  elapsed: number;
};

type MarqueeMotionState = {
  position: number;
  velocity: number;
};

export const advanceMarqueeMotion = ({
  position: currentPosition,
  velocity: currentVelocity,
  maximum: rawMaximum,
  elapsed: rawElapsed,
}: AdvanceMarqueeMotionOptions): MarqueeMotionState => {
  const maximum = Math.max(0, rawMaximum);
  let position = Math.max(0, Math.min(currentPosition, maximum));
  let velocity = Math.max(0, Math.min(currentVelocity, PIXELS_PER_SECOND / 1000));

  if (position >= maximum) return { position: maximum, velocity: 0 };

  const maximumVelocity = PIXELS_PER_SECOND / 1000;
  const acceleration = maximumVelocity / RAMP_DURATION_MS;
  let elapsed = Math.min(Math.max(0, rawElapsed), MAX_FRAME_DURATION_MS);

  while (elapsed > 0) {
    const step = Math.min(elapsed, INTEGRATION_STEP_MS);
    const remainingDistance = maximum - position;
    const brakingDistance = (velocity * velocity) / (2 * acceleration);
    const nextAcceleration = brakingDistance >= remainingDistance ? -acceleration : velocity < maximumVelocity ? acceleration : 0;
    const nextVelocity = Math.max(0, Math.min(maximumVelocity, velocity + nextAcceleration * step));
    const nextPosition = position + ((velocity + nextVelocity) / 2) * step;

    if (nextPosition >= maximum) return { position: maximum, velocity: 0 };

    position = nextPosition;
    velocity = nextVelocity;
    elapsed -= step;
  }

  return { position, velocity };
};
