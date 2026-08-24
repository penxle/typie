import { projectPrismIconPose } from './prism-icon-morph.ts';
import { PRISM_RENDER_SCALE_REFERENCE_SIZE, resolvePrismObjectPoseQuaternion } from './prism-object.ts';

type Vector2 = [number, number];
type Vector3 = [number, number, number];
type Quaternion = [number, number, number, number];
type VelocitySample = { at: number; velocity: Vector3 };

export type PrismPointerInteractionFrame = {
  angularVelocity: Vector3;
  dispersion: number;
  hdrHeadroom: number;
  lightPhaseOffset: number;
  orientation: Quaternion;
  phase: number;
  scale: number;
  sheenStrength: number;
  sourceHalo: number;
  velocity: number;
};

export type PrismPointerInteractionKinematics = {
  angularVelocity: readonly [number, number, number];
  orientation: readonly [number, number, number, number];
  phase: number;
  velocity: number;
};

type Options = {
  lightPeriod: number;
  period: number;
  prismSize: number;
};

const DRAG_THRESHOLD_PX = 3;
const DRAG_RADIANS_PER_VIEWPORT = 1 / 0.38;
const FLING_SAMPLE_WINDOW_MS = 90;
const FULL_FLING_TRAVEL_RADIANS = 0.18;
const ORIENTATION_SPRING = 1.4;
const PRESS_SCALE_AMOUNT = 0.03;
const PRESS_SCALE_SPRING = 180;
const PRESS_SCALE_DAMPING = 26;
const CLICK_SPIN_INCREMENT = 0.34;
const MAX_CLICK_SPIN_SPEED = 1;
const CLICK_SPIN_HALF_LIFE_SECONDS = 1;
const HIT_SLOP_PX = 3;
const INTEREST_REACH_PX = 56;
const TAU = Math.PI * 2;

const clamp = (value: number, minimum = 0, maximum = 1) => Math.max(minimum, Math.min(maximum, value));

const damp = (current: number, target: number, rate: number, dt: number) => current + (target - current) * (1 - Math.exp(-rate * dt));

const smootherstep = (value: number) => {
  const progress = clamp(value);
  return progress * progress * progress * (progress * (progress * 6 - 15) + 10);
};

const axisAngle = (axis: Vector3, angle: number): Quaternion => {
  const sine = Math.sin(angle * 0.5);
  return [axis[0] * sine, axis[1] * sine, axis[2] * sine, Math.cos(angle * 0.5)];
};

const multiplyQuaternion = (left: Quaternion, right: Quaternion): Quaternion => [
  left[3] * right[0] + left[0] * right[3] + left[1] * right[2] - left[2] * right[1],
  left[3] * right[1] - left[0] * right[2] + left[1] * right[3] + left[2] * right[0],
  left[3] * right[2] + left[0] * right[1] - left[1] * right[0] + left[2] * right[3],
  left[3] * right[3] - left[0] * right[0] - left[1] * right[1] - left[2] * right[2],
];

const normalizedQuaternion = (quaternion: Quaternion): Quaternion => {
  const length = Math.hypot(...quaternion) || 1;
  return quaternion.map((value) => value / length) as Quaternion;
};

const conjugateQuaternion = (quaternion: Quaternion): Quaternion => [-quaternion[0], -quaternion[1], -quaternion[2], quaternion[3]];

const twoAxisValuatorRotation = (deltaX: number, deltaY: number, radiansPerPixel: number): Quaternion =>
  multiplyQuaternion(axisAngle([0, 1, 0], deltaX * radiansPerPixel), axisAngle([1, 0, 0], deltaY * radiansPerPixel));

const angularVelocityFromQuaternion = (quaternion: Quaternion, elapsed: number): Vector3 => {
  const vectorLength = Math.hypot(quaternion[0], quaternion[1], quaternion[2]);
  if (vectorLength < 0.00001) return [0, 0, 0];
  const angle = 2 * Math.atan2(vectorLength, Math.abs(quaternion[3]));
  const velocity = clamp(angle / elapsed, 0, 7.5);
  return [(quaternion[0] / vectorLength) * velocity, (quaternion[1] / vectorLength) * velocity, (quaternion[2] / vectorLength) * velocity];
};

const quaternionRotationVector = (quaternion: Quaternion): Vector3 => {
  let canonicalOrientation = normalizedQuaternion(quaternion);
  if (canonicalOrientation[3] < 0) canonicalOrientation = canonicalOrientation.map((value) => -value) as Quaternion;
  const vectorLength = Math.hypot(canonicalOrientation[0], canonicalOrientation[1], canonicalOrientation[2]);
  if (vectorLength < 0.000001) return [0, 0, 0];
  const angle = 2 * Math.atan2(vectorLength, canonicalOrientation[3]);
  return [
    (canonicalOrientation[0] / vectorLength) * angle,
    (canonicalOrientation[1] / vectorLength) * angle,
    (canonicalOrientation[2] / vectorLength) * angle,
  ];
};

const sampledFlingVelocity = (samples: readonly VelocitySample[], releasedAt: number): Vector3 => {
  let totalWeight = 0;
  const velocity: Vector3 = [0, 0, 0];
  for (const sample of samples) {
    const age = releasedAt - sample.at;
    if (age < 0 || age > FLING_SAMPLE_WINDOW_MS) continue;
    const weight = 0.15 + 0.85 * (1 - age / FLING_SAMPLE_WINDOW_MS);
    velocity[0] += sample.velocity[0] * weight;
    velocity[1] += sample.velocity[1] * weight;
    velocity[2] += sample.velocity[2] * weight;
    totalWeight += weight;
  }
  if (totalWeight === 0) return [0, 0, 0];
  return velocity.map((value) => value / totalWeight) as Vector3;
};

const convexHull = (points: readonly Vector2[]): Vector2[] => {
  const sorted = points.toSorted(([leftX, leftY], [rightX, rightY]) => leftX - rightX || leftY - rightY);
  const cross = (origin: Vector2, first: Vector2, second: Vector2) =>
    (first[0] - origin[0]) * (second[1] - origin[1]) - (first[1] - origin[1]) * (second[0] - origin[0]);
  const half = (vertices: Vector2[]) => {
    const result: Vector2[] = [];
    for (const point of vertices) {
      while (result.length >= 2) {
        const first = result.at(-2);
        const second = result.at(-1);
        if (!first || !second || cross(first, second, point) > 0) break;
        result.pop();
      }
      result.push(point);
    }
    return result;
  };
  const lower = half(sorted);
  const upper = half(sorted.toReversed());
  return [...lower.slice(0, -1), ...upper.slice(0, -1)];
};

const expandHull = (hull: readonly Vector2[], amount: number): Vector2[] => {
  const center = hull.reduce<Vector2>((sum, point) => [sum[0] + point[0], sum[1] + point[1]], [0, 0]);
  center[0] /= Math.max(hull.length, 1);
  center[1] /= Math.max(hull.length, 1);
  return hull.map(([x, y]) => {
    const offsetX = x - center[0];
    const offsetY = y - center[1];
    const distance = Math.max(Math.hypot(offsetX, offsetY), 0.001);
    return [x + (offsetX / distance) * amount, y + (offsetY / distance) * amount];
  });
};

const pointInsideConvexHull = (point: Vector2, hull: readonly Vector2[]) => {
  if (hull.length < 3) return false;
  let direction = 0;
  for (let index = 0; index < hull.length; index += 1) {
    const start = hull[index];
    const end = hull[(index + 1) % hull.length];
    if (!start || !end) continue;
    const cross = (end[0] - start[0]) * (point[1] - start[1]) - (end[1] - start[1]) * (point[0] - start[0]);
    if (Math.abs(cross) < 0.001) continue;
    const nextDirection = Math.sign(cross);
    if (direction !== 0 && nextDirection !== direction) return false;
    direction = nextDirection;
  }
  return true;
};

const distanceToSegment = (point: Vector2, start: Vector2, end: Vector2) => {
  const deltaX = end[0] - start[0];
  const deltaY = end[1] - start[1];
  const lengthSquared = deltaX * deltaX + deltaY * deltaY;
  const amount = lengthSquared > 0 ? clamp(((point[0] - start[0]) * deltaX + (point[1] - start[1]) * deltaY) / lengthSquared) : 0;
  return Math.hypot(point[0] - (start[0] + deltaX * amount), point[1] - (start[1] + deltaY * amount));
};

const distanceToHull = (point: Vector2, hull: readonly Vector2[]) => {
  if (pointInsideConvexHull(point, hull)) return 0;
  let distance = Infinity;
  for (let index = 0; index < hull.length; index += 1) {
    const start = hull[index];
    const end = hull[(index + 1) % hull.length];
    if (start && end) distance = Math.min(distance, distanceToSegment(point, start, end));
  }
  return distance;
};

export function createPrismPointerInteraction(element: HTMLElement, options: Options) {
  const document = element.ownerDocument;
  const baseSpeed = 1 / options.period;
  const baseScale = options.prismSize / PRISM_RENDER_SCALE_REFERENCE_SIZE;
  let inputEnabled = false;
  let pointerInside = false;
  let pointerDown = false;
  let pointerId = -1;
  let dragging = false;
  let pointerX = 0;
  let pointerY = 0;
  let pointerClientX = 0;
  let pointerClientY = 0;
  let pressPointerX = 0;
  let pressPointerY = 0;
  let lastDragPointerX = 0;
  let lastDragPointerY = 0;
  let lastPointerAt = 0;
  let dragVelocitySamples: VelocitySample[] = [];
  let dragTravelAngle = 0;
  let phase = 0;
  let spinVelocity = baseSpeed;
  let clickSpinImpulse = 0;
  let pressScaleProgress = 0;
  let pressScaleVelocity = 0;
  let interactionOrientation: Quaternion = [0, 0, 0, 1];
  let interactionAngularVelocity: Vector3 = [0, 0, 0];
  let previousSampleOrientation: Quaternion = [0, 0, 0, 1];
  let relativeLightPhase = 0;
  let currentPrismHull: Vector2[] = [];
  let tiltX = 0;
  let tiltZ = 0;
  let attentionX = 0;
  let attentionY = 0;
  let attentionVelocityX = 0;
  let attentionVelocityY = 0;
  let lastFrame = performance.now();

  const updatePointer = (event: PointerEvent) => {
    const rect = element.getBoundingClientRect();
    pointerClientX = event.clientX;
    pointerClientY = event.clientY;
    pointerX = clamp(((event.clientX - rect.left) / rect.width) * 2 - 1, -1, 1);
    pointerY = clamp(((event.clientY - rect.top) / rect.height) * 2 - 1, -1, 1);
  };

  const pointerHitsPrism = (clientX: number, clientY: number) =>
    pointInsideConvexHull([clientX, clientY], expandHull(currentPrismHull, HIT_SLOP_PX));

  const releaseCapture = (capturedPointerId: number) => {
    if (capturedPointerId < 0 || !element.hasPointerCapture(capturedPointerId)) return;
    element.releasePointerCapture(capturedPointerId);
  };

  const resetPointer = () => {
    const capturedPointerId = pointerId;
    pointerDown = false;
    pointerId = -1;
    dragging = false;
    dragVelocitySamples = [];
    dragTravelAngle = 0;
    releaseCapture(capturedPointerId);
  };

  const handlePointerEnter = (event: PointerEvent) => {
    if (!inputEnabled) return;
    pointerInside = true;
    updatePointer(event);
  };

  const handlePointerMove = (event: PointerEvent) => {
    if (!inputEnabled) return;
    updatePointer(event);
    if (!pointerDown || event.pointerId !== pointerId) return;
    const now = performance.now();
    if (!dragging && Math.hypot(event.clientX - pressPointerX, event.clientY - pressPointerY) < DRAG_THRESHOLD_PX) return;
    if (!dragging) {
      dragging = true;
      spinVelocity = 0;
      clickSpinImpulse = 0;
      lastPointerAt = now - 1000 / 60;
    }
    const elapsed = Math.max((now - lastPointerAt) / 1000, 1 / 240);
    const rect = element.getBoundingClientRect();
    const radiansPerPixel = DRAG_RADIANS_PER_VIEWPORT / Math.max(Math.min(rect.width, rect.height), 1);
    const deltaQuaternion = twoAxisValuatorRotation(event.clientX - lastDragPointerX, event.clientY - lastDragPointerY, radiansPerPixel);
    interactionOrientation = normalizedQuaternion(multiplyQuaternion(deltaQuaternion, interactionOrientation));
    interactionAngularVelocity = angularVelocityFromQuaternion(deltaQuaternion, elapsed);
    dragTravelAngle += Math.hypot(...interactionAngularVelocity) * elapsed;
    dragVelocitySamples.push({ at: now, velocity: interactionAngularVelocity });
    dragVelocitySamples = dragVelocitySamples.filter((sample) => now - sample.at <= FLING_SAMPLE_WINDOW_MS);
    lastDragPointerX = event.clientX;
    lastDragPointerY = event.clientY;
    lastPointerAt = now;
    if (event.cancelable) event.preventDefault();
  };

  const handlePointerLeave = () => {
    pointerInside = false;
    if (!pointerDown) {
      pointerX = 0;
      pointerY = 0;
    }
  };

  const handlePointerDown = (event: PointerEvent) => {
    if (!inputEnabled || event.button !== 0 || !event.isPrimary) return;
    updatePointer(event);
    if (!pointerHitsPrism(event.clientX, event.clientY)) return;
    pointerDown = true;
    pointerId = event.pointerId;
    dragging = false;
    pressPointerX = event.clientX;
    pressPointerY = event.clientY;
    lastDragPointerX = event.clientX;
    lastDragPointerY = event.clientY;
    lastPointerAt = performance.now();
    dragVelocitySamples = [];
    dragTravelAngle = 0;
    try {
      element.setPointerCapture(event.pointerId);
    } catch {
      resetPointer();
    }
  };

  const releasePointer = (event: PointerEvent, cancelled = false) => {
    if (!pointerDown || event.pointerId !== pointerId) return;
    const capturedPointerId = pointerId;
    pointerDown = false;
    pointerId = -1;
    releaseCapture(capturedPointerId);
    if (cancelled) {
      if (dragging) interactionAngularVelocity = [0, 0, 0];
    } else if (dragging) {
      const flingVelocity = sampledFlingVelocity(dragVelocitySamples, performance.now());
      const flingIntent = smootherstep(dragTravelAngle / FULL_FLING_TRAVEL_RADIANS);
      interactionAngularVelocity = flingVelocity.map((value) => value * flingIntent) as Vector3;
    } else {
      clickSpinImpulse = clamp(clickSpinImpulse + CLICK_SPIN_INCREMENT, 0, MAX_CLICK_SPIN_SPEED);
    }
    dragging = false;
    dragVelocitySamples = [];
    dragTravelAngle = 0;
  };

  const handlePointerCancel = (event: PointerEvent) => releasePointer(event, true);
  const handleLostPointerCapture = (event: PointerEvent) => {
    if (!pointerDown || event.pointerId !== pointerId) return;
    releasePointer(event, event.pointerType !== 'mouse' || event.buttons !== 0);
  };

  element.addEventListener('pointerenter', handlePointerEnter);
  element.addEventListener('pointermove', handlePointerMove);
  element.addEventListener('pointerleave', handlePointerLeave);
  element.addEventListener('pointerdown', handlePointerDown);
  element.addEventListener('lostpointercapture', handleLostPointerCapture);
  document.addEventListener('pointerup', releasePointer, { capture: true });
  document.addEventListener('pointercancel', handlePointerCancel, { capture: true });

  return {
    destroy() {
      inputEnabled = false;
      resetPointer();
      element.removeEventListener('pointerenter', handlePointerEnter);
      element.removeEventListener('pointermove', handlePointerMove);
      element.removeEventListener('pointerleave', handlePointerLeave);
      element.removeEventListener('pointerdown', handlePointerDown);
      element.removeEventListener('lostpointercapture', handleLostPointerCapture);
      document.removeEventListener('pointerup', releasePointer, { capture: true });
      document.removeEventListener('pointercancel', handlePointerCancel, { capture: true });
    },
    reset(kinematics: PrismPointerInteractionKinematics) {
      phase = kinematics.phase;
      spinVelocity = kinematics.velocity;
      const baseOrientation = resolvePrismObjectPoseQuaternion(phase, 1, 1) as Quaternion;
      interactionOrientation = normalizedQuaternion(
        multiplyQuaternion([...kinematics.orientation] as Quaternion, conjugateQuaternion(baseOrientation)),
      );
      interactionAngularVelocity = [
        kinematics.angularVelocity[0],
        kinematics.angularVelocity[1] - TAU * kinematics.velocity,
        kinematics.angularVelocity[2],
      ];
      previousSampleOrientation = [...kinematics.orientation];
      clickSpinImpulse = 0;
      pressScaleProgress = 0;
      pressScaleVelocity = 0;
      relativeLightPhase = 0;
      attentionX = 0;
      attentionY = 0;
      attentionVelocityX = 0;
      attentionVelocityY = 0;
      tiltX = 0;
      tiltZ = 0;
      lastFrame = performance.now();
      resetPointer();
    },
    sample(now: number): PrismPointerInteractionFrame {
      const dt = clamp((now - lastFrame) / 1000, 0, 0.05);
      lastFrame = now;
      const hullDistance = pointerInside ? distanceToHull([pointerClientX, pointerClientY], currentPrismHull) : Infinity;
      const linearCloseness = pointerInside ? clamp(1 - hullDistance / INTEREST_REACH_PX) : 0;
      const proximity = smootherstep(linearCloseness) ** 2;
      const autonomousX = Math.sin(now / 2800) * 0.22;
      const autonomousY = Math.sin(now / 3600 + 0.8) * 0.16;
      const targetX = autonomousX + (pointerX - autonomousX) * proximity;
      const targetY = autonomousY + (pointerY - autonomousY) * proximity;
      const spring = 2.8 + proximity * 5.4;
      const damping = 3.1 + proximity * 1.7;
      attentionVelocityX += (targetX - attentionX) * spring * dt;
      attentionVelocityY += (targetY - attentionY) * spring * dt;
      attentionVelocityX *= Math.exp(-damping * dt);
      attentionVelocityY *= Math.exp(-damping * dt);
      attentionX += attentionVelocityX * dt;
      attentionY += attentionVelocityY * dt;

      const pressScaleTarget = pointerDown ? 1 : 0;
      pressScaleVelocity += (pressScaleTarget - pressScaleProgress) * PRESS_SCALE_SPRING * dt;
      pressScaleVelocity *= Math.exp(-PRESS_SCALE_DAMPING * dt);
      pressScaleProgress += pressScaleVelocity * dt;
      if (Math.abs(pressScaleTarget - pressScaleProgress) < 0.0001 && Math.abs(pressScaleVelocity) < 0.001) {
        pressScaleProgress = pressScaleTarget;
        pressScaleVelocity = 0;
      }
      const pressedScale = smootherstep(pressScaleProgress);

      clickSpinImpulse *= Math.exp((-Math.LN2 / CLICK_SPIN_HALF_LIFE_SECONDS) * dt);
      const clickSpinEnergy = clickSpinImpulse / MAX_CLICK_SPIN_SPEED;
      const targetSpinVelocity = baseSpeed * (1 - proximity * 0.4);
      if (!dragging) spinVelocity = damp(spinVelocity, targetSpinVelocity, 2.4, dt);
      const velocity = dragging ? 0 : spinVelocity + clickSpinImpulse;
      phase += velocity * dt;

      if (!dragging) {
        const rotationError = quaternionRotationVector(interactionOrientation);
        interactionAngularVelocity = [
          interactionAngularVelocity[0] - rotationError[0] * ORIENTATION_SPRING * dt,
          interactionAngularVelocity[1] - rotationError[1] * ORIENTATION_SPRING * dt,
          interactionAngularVelocity[2] - rotationError[2] * ORIENTATION_SPRING * dt,
        ];
        let relativeAngularSpeed = Math.hypot(...interactionAngularVelocity);
        const restDamping = 0.62 + (1 - smootherstep(relativeAngularSpeed / 2.8)) * 1.65;
        interactionAngularVelocity = interactionAngularVelocity.map((value) => value * Math.exp(-restDamping * dt)) as Vector3;
        relativeAngularSpeed = Math.hypot(...interactionAngularVelocity);
        if (relativeAngularSpeed > 0.0001) {
          const axis = interactionAngularVelocity.map((value) => value / relativeAngularSpeed) as Vector3;
          interactionOrientation = normalizedQuaternion(
            multiplyQuaternion(axisAngle(axis, relativeAngularSpeed * dt), interactionOrientation),
          );
        }
        if (Math.hypot(...rotationError) < 0.0005 && relativeAngularSpeed < 0.002) {
          interactionOrientation = [0, 0, 0, 1];
          interactionAngularVelocity = [0, 0, 0];
        }
      }

      const targetTiltX = dragging ? tiltX : -attentionY * 0.19 + Math.sin(now / 1900) * 0.018;
      const targetTiltZ = dragging ? tiltZ : -attentionX * 0.18 + Math.sin(now / 2300 + 1.1) * 0.014;
      const followRate = dragging ? 18 : 5.2;
      tiltX = damp(tiltX, targetTiltX, followRate, dt);
      tiltZ = damp(tiltZ, targetTiltZ, followRate, dt);
      const baseOrientation = resolvePrismObjectPoseQuaternion(phase, 1, 1) as Quaternion;
      const tiltOrientation = multiplyQuaternion(axisAngle([0, 0, 1], tiltZ), axisAngle([1, 0, 0], tiltX));
      const orientation = normalizedQuaternion(
        multiplyQuaternion(tiltOrientation, multiplyQuaternion(interactionOrientation, baseOrientation)),
      );
      const orientationDelta = normalizedQuaternion(multiplyQuaternion(orientation, conjugateQuaternion(previousSampleOrientation)));
      const angularVelocity = dt > 0 ? angularVelocityFromQuaternion(orientationDelta, dt) : ([0, TAU * velocity, 0] as Vector3);
      const finalSpeedTurns = dt > 0 ? Math.hypot(...quaternionRotationVector(orientationDelta)) / (dt * TAU) : Math.abs(velocity);
      previousSampleOrientation = orientation;
      const scale = 1 + Math.sin(now / 1050) * 0.009 + proximity * 0.018 + pressedScale * PRESS_SCALE_AMOUNT;
      relativeLightPhase += dt * (1 / options.lightPeriod - baseSpeed);
      const rect = element.getBoundingClientRect();
      const projectedVertices = projectPrismIconPose(
        {
          cameraDistance: 4.2,
          orientation,
          projectionDistance: 2.96,
          renderScale: baseScale * clamp(scale, 0.9, 1.12),
        },
        32,
      );
      currentPrismHull = convexHull(
        projectedVertices.map(([x, y]): Vector2 => [rect.left + rect.width * 0.5 + x - 16, rect.top + rect.height * 0.5 + y - 16]),
      );
      const speedEnergy = clamp((finalSpeedTurns - baseSpeed) / MAX_CLICK_SPIN_SPEED);
      return {
        angularVelocity,
        dispersion: 1.25 + clickSpinEnergy * 0.22,
        hdrHeadroom: 1.25 + speedEnergy * 0.34,
        lightPhaseOffset: 0.23 + relativeLightPhase,
        orientation,
        phase,
        scale,
        sheenStrength: 1.15 + proximity * 0.16 + pressedScale * 0.07 + clickSpinEnergy * 0.16,
        sourceHalo: 2.05 + clickSpinEnergy * 0.32,
        velocity,
      };
    },
    setInputEnabled(enabled: boolean) {
      inputEnabled = enabled;
      if (!inputEnabled) {
        resetPointer();
        pointerInside = false;
        pointerX = 0;
        pointerY = 0;
      }
    },
  };
}
