import { PRISM_VERTICES, prismOrientation } from './prism-spinner-morph.ts';

type Vector2 = readonly [number, number];
type Vector3 = readonly [number, number, number];
type Quaternion = readonly [number, number, number, number];
type Edge = readonly [number, number];
type EdgeFaces = readonly [number, number];
type Face = readonly number[];

const CAMERA_PLANE_SAFETY_DISTANCE = 0.000001;
const EPSILON = 0.000001;
const PHASE_VALIDATION_RATE = 240;
const PRISM_ICON_CANONICAL_CSS_SIZE = 32;
const PRISM_ICON_OPTICAL_OFFSET_Y = 1.25 / 24;
const PRISM_OBJECT_VIEWPORT_REFERENCE_SIZE = 132;
const PRISM_ICON_SVG_FIT_SCALE = 0.5832;
const PRISM_ICON_TO_VIEWPORT_SCALE = PRISM_ICON_CANONICAL_CSS_SIZE / (PRISM_OBJECT_VIEWPORT_REFERENCE_SIZE * 0.5);
const PRISM_ICON_CAMERA_DISTANCE_UNSCALED = 32 / 15;
const PRISM_SETTLED_CAMERA_DISTANCE = 4.2;
const PRISM_SETTLED_PROJECTION_DISTANCE = 2.96;
const TAU = Math.PI * 2;

export const PRISM_ICON_DURATION_SECONDS = 2.2;
export const PRISM_ICON_IDLE_PHASE_TURNS = 1 / 8;
export const PRISM_ICON_CAMERA_DISTANCE = PRISM_ICON_CAMERA_DISTANCE_UNSCALED * PRISM_ICON_TO_VIEWPORT_SCALE;
export const PRISM_ICON_PROJECTION_DISTANCE =
  ((PRISM_ICON_CAMERA_DISTANCE_UNSCALED * PRISM_SETTLED_PROJECTION_DISTANCE) / PRISM_SETTLED_CAMERA_DISTANCE) *
  PRISM_ICON_TO_VIEWPORT_SCALE;
export const PRISM_ICON_IDLE_RENDER_SCALE = PRISM_ICON_SVG_FIT_SCALE * PRISM_ICON_TO_VIEWPORT_SCALE;

export type PrismIconPose = {
  cameraDistance: number;
  orientation: Quaternion;
  projectionDistance: number;
  renderScale: number;
  viewportOffsetY?: number;
};

export type PrismIconGeometry = {
  edgeFaces: readonly EdgeFaces[];
  edges: readonly Edge[];
  faces: readonly Face[];
  vertices: readonly Vector3[];
};

export type PrismIconMorphSample = {
  angularVelocity: Vector3;
  cameraDistance: number;
  edgeAlpha: number;
  edgeHighlightProgress: number;
  edgeWidthCssPixels: number;
  optics: number;
  orientation: Quaternion;
  phase: number;
  progress: number;
  projectionDistance: number;
  renderScale: number;
  spectrumPassProgress: number;
  surface: number;
  surfaceReveal: number;
  velocity: number;
};

export type PrismIconMorphTrajectory = {
  readonly correctionEndTime: number;
  readonly correctionStartDerivative: Vector3;
  readonly correctionStartVector: Vector3;
  durationSeconds: number;
  readonly endPhase: number;
  iconSize: number;
  readonly phaseVelocityControls: readonly number[];
  prismSize: number;
  prismVelocity: number;
  readonly startAngularVelocity: Vector3;
  readonly startEdgeHighlightProgress: number;
  readonly startOrientation: Quaternion;
  readonly startPhase: number;
  startProgress: number;
  readonly startVelocity: number;
  targetProgress: 0 | 1;
};

export const PRISM_ICON_GEOMETRY: PrismIconGeometry = Object.freeze({
  edgeFaces: Object.freeze([
    [0, 2],
    [0, 3],
    [0, 4],
    [1, 4],
    [1, 2],
    [1, 3],
    [2, 4],
    [2, 3],
    [3, 4],
  ] as const),
  edges: Object.freeze([
    [0, 2],
    [2, 5],
    [5, 0],
    [1, 3],
    [3, 4],
    [4, 1],
    [0, 3],
    [2, 4],
    [5, 1],
  ] as const),
  faces: Object.freeze([
    [0, 5, 2],
    [1, 3, 4],
    [0, 2, 4, 3],
    [5, 1, 4, 2],
    [0, 3, 1, 5],
  ] as const),
  vertices: PRISM_VERTICES,
});

function clamp(value: number, minimum = 0, maximum = 1): number {
  return Math.max(minimum, Math.min(maximum, value));
}

function smoothstep(value: number): number {
  const amount = clamp(value);
  return amount * amount * (3 - 2 * amount);
}

function smootherstep(value: number): number {
  const amount = clamp(value);
  return clamp(amount * amount * amount * (amount * (amount * 6 - 15) + 10));
}

function transitionWindow(value: number, start: number, end: number): number {
  return smootherstep((value - start) / Math.max(end - start, EPSILON));
}

function linearWindow(value: number, start: number, end: number): number {
  return clamp((value - start) / Math.max(end - start, EPSILON));
}

function lerp(start: number, end: number, amount: number): number {
  return start + (end - start) * amount;
}

function normalizeQuaternion(quaternion: Quaternion): Quaternion {
  const length = Math.hypot(...quaternion);
  return quaternion.map((value) => value / length) as unknown as Quaternion;
}

function quaternionMultiply(left: Quaternion, right: Quaternion): Quaternion {
  return [
    left[3] * right[0] + left[0] * right[3] + left[1] * right[2] - left[2] * right[1],
    left[3] * right[1] - left[0] * right[2] + left[1] * right[3] + left[2] * right[0],
    left[3] * right[2] + left[0] * right[1] - left[1] * right[0] + left[2] * right[3],
    left[3] * right[3] - left[0] * right[0] - left[1] * right[1] - left[2] * right[2],
  ];
}

function quaternionConjugate(quaternion: Quaternion): Quaternion {
  return [-quaternion[0], -quaternion[1], -quaternion[2], quaternion[3]];
}

function quaternionFromAxisAngle(axis: Vector3, angle: number): Quaternion {
  const sine = Math.sin(angle * 0.5);
  return [axis[0] * sine, axis[1] * sine, axis[2] * sine, Math.cos(angle * 0.5)];
}

function quaternionFromRotationVector(vector: Vector3): Quaternion {
  const angle = Math.hypot(...vector);
  if (angle <= 1e-12) return normalizeQuaternion([vector[0] * 0.5, vector[1] * 0.5, vector[2] * 0.5, 1]);
  const scale = Math.sin(angle * 0.5) / angle;
  return [vector[0] * scale, vector[1] * scale, vector[2] * scale, Math.cos(angle * 0.5)];
}

function quaternionRotationVector(quaternion: Quaternion): Vector3 {
  let normalized = normalizeQuaternion(quaternion);
  if (normalized[3] < 0) normalized = normalized.map((value) => -value) as unknown as Quaternion;
  const vectorLength = Math.hypot(normalized[0], normalized[1], normalized[2]);
  if (vectorLength <= 1e-12) return [normalized[0] * 2, normalized[1] * 2, normalized[2] * 2];
  const angle = 2 * Math.atan2(vectorLength, clamp(normalized[3], -1, 1));
  const scale = angle / vectorLength;
  return [normalized[0] * scale, normalized[1] * scale, normalized[2] * scale];
}

function slerpQuaternion(from: Quaternion, to: Quaternion, amount: number): Quaternion {
  const start = normalizeQuaternion(from);
  let end = normalizeQuaternion(to);
  let cosine = start[0] * end[0] + start[1] * end[1] + start[2] * end[2] + start[3] * end[3];
  if (cosine < 0) {
    end = end.map((value) => -value) as unknown as Quaternion;
    cosine = -cosine;
  }
  const progress = clamp(amount);
  if (cosine > 0.9995) {
    return normalizeQuaternion([
      start[0] + (end[0] - start[0]) * progress,
      start[1] + (end[1] - start[1]) * progress,
      start[2] + (end[2] - start[2]) * progress,
      start[3] + (end[3] - start[3]) * progress,
    ]);
  }
  const angle = Math.acos(clamp(cosine, -1, 1));
  const denominator = Math.sin(angle);
  const startWeight = Math.sin((1 - progress) * angle) / denominator;
  const endWeight = Math.sin(progress * angle) / denominator;
  return normalizeQuaternion([
    start[0] * startWeight + end[0] * endWeight,
    start[1] * startWeight + end[1] * endWeight,
    start[2] * startWeight + end[2] * endWeight,
    start[3] * startWeight + end[3] * endWeight,
  ]);
}

const IDLE_CORRECTION: Quaternion = Object.freeze([-0.680330509147905, 0.388123683378068, -0.454269067717495, 0.424440830786425]);
const ORDINARY_PRISM_CORRECTION = normalizeQuaternion(
  quaternionMultiply(quaternionFromAxisAngle([1, 0, 0], -0.3), quaternionFromAxisAngle([0, 0, 1], 0.1)),
);

function yRotation(phase: number): Quaternion {
  return quaternionFromAxisAngle([0, 1, 0], phase * TAU);
}

function canonicalOrientation(phase: number, progress: number): Quaternion {
  const correction = slerpQuaternion(IDLE_CORRECTION, ORDINARY_PRISM_CORRECTION, smootherstep(progress));
  return normalizeQuaternion(quaternionMultiply(yRotation(phase), correction));
}

export const PRISM_ICON_IDLE_POSE: PrismIconPose = Object.freeze({
  cameraDistance: PRISM_ICON_CAMERA_DISTANCE,
  orientation: canonicalOrientation(PRISM_ICON_IDLE_PHASE_TURNS, 0),
  projectionDistance: PRISM_ICON_PROJECTION_DISTANCE,
  renderScale: PRISM_ICON_IDLE_RENDER_SCALE,
  viewportOffsetY: PRISM_ICON_OPTICAL_OFFSET_Y,
});

export const PRISM_ICON_IDLE_VISIBLE_EDGE_INDICES = Object.freeze([0, 1, 2, 3, 6, 8] as const);

function rotateByQuaternion(vector: Vector3, quaternion: Quaternion): Vector3 {
  const [qx, qy, qz, qw] = normalizeQuaternion(quaternion);
  const tx = 2 * (qy * vector[2] - qz * vector[1]);
  const ty = 2 * (qz * vector[0] - qx * vector[2]);
  const tz = 2 * (qx * vector[1] - qy * vector[0]);
  return [vector[0] + qw * tx + qy * tz - qz * ty, vector[1] + qw * ty + qz * tx - qx * tz, vector[2] + qw * tz + qx * ty - qy * tx];
}

function requireFinitePositive(value: number, label: string): void {
  if (!Number.isFinite(value) || value <= 0) throw new RangeError(`${label} must be finite and positive.`);
}

function validatePose(pose: PrismIconPose): void {
  requireFinitePositive(pose.cameraDistance, 'Prism icon camera distance');
  requireFinitePositive(pose.projectionDistance, 'Prism icon projection distance');
  requireFinitePositive(pose.renderScale, 'Prism icon render scale');
  if (pose.viewportOffsetY !== undefined && !Number.isFinite(pose.viewportOffsetY)) {
    throw new RangeError('Prism icon viewport offset must be finite.');
  }
  const norm = Array.isArray(pose.orientation) ? Math.hypot(...pose.orientation) : NaN;
  if (
    norm === 0 ||
    !Number.isFinite(norm) ||
    !Array.isArray(pose.orientation) ||
    pose.orientation.length !== 4 ||
    !pose.orientation.every(Number.isFinite)
  ) {
    throw new RangeError('Prism icon orientation must be a finite, non-zero quaternion.');
  }
}

export function projectPrismIconPose(pose: PrismIconPose, cssSize = 32): readonly Vector2[] {
  requireFinitePositive(cssSize, 'Prism icon CSS size');
  validatePose(pose);
  const center = cssSize * 0.5;
  const centerY = center + (pose.viewportOffsetY ?? 0) * cssSize;
  const viewportPixelsPerWorldUnit = (PRISM_OBJECT_VIEWPORT_REFERENCE_SIZE * 0.5 * cssSize) / PRISM_ICON_CANONICAL_CSS_SIZE;
  return PRISM_ICON_GEOMETRY.vertices.map((vertex): Vector2 => {
    const rotated = rotateByQuaternion(vertex, pose.orientation);
    const world = rotated.map((value) => value * pose.renderScale) as unknown as Vector3;
    const cameraPlaneDistance = pose.cameraDistance - world[2];
    if (cameraPlaneDistance <= CAMERA_PLANE_SAFETY_DISTANCE) {
      throw new RangeError('Prism icon vertices must remain safely in front of the camera plane.');
    }
    const perspective = pose.projectionDistance / cameraPlaneDistance;
    return [center + world[0] * perspective * viewportPixelsPerWorldUnit, centerY - world[1] * perspective * viewportPixelsPerWorldUnit];
  });
}

export function resolvePrismIconShaderOffsetY(progress: number): number {
  return -PRISM_ICON_OPTICAL_OFFSET_Y * PRISM_ICON_TO_VIEWPORT_SCALE * (1 - smootherstep(progress));
}

function binomialCoefficient(degree: number, index: number): number {
  if (index < 0 || index > degree) return 0;
  let result = 1;
  for (let step = 1; step <= Math.min(index, degree - index); step += 1) {
    result = (result * (degree - step + 1)) / step;
  }
  return result;
}

function bernstein(degree: number, index: number, time: number): number {
  const amount = clamp(time);
  return binomialCoefficient(degree, index) * amount ** index * (1 - amount) ** (degree - index);
}

function integratedBernstein(degree: number, index: number, time: number): number {
  let integral = 0;
  for (let elevated = index + 1; elevated <= degree + 1; elevated += 1) {
    integral += bernstein(degree + 1, elevated, time);
  }
  return integral / (degree + 1);
}

function samplePhaseVelocity(controls: readonly number[], time: number): number {
  const degree = controls.length - 1;
  return controls.reduce((sum, velocity, index) => sum + velocity * bernstein(degree, index, time), 0);
}

function integratePhaseVelocity(controls: readonly number[], time: number): number {
  const degree = controls.length - 1;
  return controls.reduce((sum, velocity, index) => sum + velocity * integratedBernstein(degree, index, time), 0);
}

function openingVelocityControls(startVelocity: number, endVelocity: number): number[] {
  const rideVelocity = Math.max(startVelocity, endVelocity, 1.2);
  return [
    startVelocity,
    startVelocity,
    rideVelocity,
    rideVelocity,
    Math.max(endVelocity, rideVelocity * 0.75),
    Math.max(endVelocity, rideVelocity * 0.46),
    endVelocity,
    endVelocity,
  ];
}

export function nextPrismIconIdlePhase(startPhase: number): number {
  if (!Number.isFinite(startPhase)) throw new RangeError('Prism icon start phase must be finite.');
  let target = PRISM_ICON_IDLE_PHASE_TURNS + Math.ceil(startPhase - PRISM_ICON_IDLE_PHASE_TURNS + 1e-9);
  if (target <= startPhase) target += 1;
  return target;
}

function closingVelocityControls(
  startVelocity: number,
  durationSeconds: number,
  startPhase: number,
): {
  controls: readonly number[];
  endPhase: number;
} {
  let endPhase = nextPrismIconIdlePhase(startPhase);
  for (let attempt = 0; attempt < 4; attempt += 1) {
    const travel = endPhase - startPhase;
    const peak = ((8 * travel) / durationSeconds - 2 * startVelocity) / 3.05;
    const controls = [startVelocity, startVelocity, peak, peak, peak * 0.7, peak * 0.35, 0, 0];
    if (controls.every(Number.isFinite) && controls.slice(0, 6).every((value) => value > 0)) {
      return { controls, endPhase };
    }
    endPhase += 1;
  }
  throw new RangeError('Unable to create a positive forward closing prism icon phase curve.');
}

function quinticHermiteBasis(progress: number) {
  const amount = clamp(progress);
  const squared = amount * amount;
  const cubed = squared * amount;
  const fourth = cubed * amount;
  const fifth = fourth * amount;
  return {
    startPosition: 1 - 10 * cubed + 15 * fourth - 6 * fifth,
    startVelocity: amount - 6 * cubed + 8 * fourth - 3 * fifth,
  };
}

function phaseAt(trajectory: PrismIconMorphTrajectory, time: number): number {
  const amount = clamp(time);
  if (amount === 1) return trajectory.endPhase;
  return trajectory.startPhase + trajectory.durationSeconds * integratePhaseVelocity(trajectory.phaseVelocityControls, amount);
}

function progressAt(trajectory: PrismIconMorphTrajectory, time: number): number {
  return lerp(trajectory.startProgress, trajectory.targetProgress, clamp(time));
}

function baseOrientationAt(trajectory: PrismIconMorphTrajectory, time: number): Quaternion {
  return canonicalOrientation(phaseAt(trajectory, time), progressAt(trajectory, time));
}

function sampleCorrectionVector(trajectory: PrismIconMorphTrajectory, time: number): Vector3 {
  if (trajectory.correctionEndTime <= 0 || time >= trajectory.correctionEndTime) return [0, 0, 0];
  const basis = quinticHermiteBasis(time / trajectory.correctionEndTime);
  const derivativeScale = trajectory.correctionEndTime * basis.startVelocity;
  return [
    trajectory.correctionStartVector[0] * basis.startPosition + trajectory.correctionStartDerivative[0] * derivativeScale,
    trajectory.correctionStartVector[1] * basis.startPosition + trajectory.correctionStartDerivative[1] * derivativeScale,
    trajectory.correctionStartVector[2] * basis.startPosition + trajectory.correctionStartDerivative[2] * derivativeScale,
  ];
}

function sampleOrientationOnly(trajectory: PrismIconMorphTrajectory, time: number): Quaternion {
  const amount = clamp(time);
  if (amount === 0) return trajectory.startOrientation;
  if (amount === 1) {
    return trajectory.targetProgress === 0 ? PRISM_ICON_IDLE_POSE.orientation : prismOrientation(trajectory.endPhase);
  }
  const correction = quaternionFromRotationVector(sampleCorrectionVector(trajectory, amount));
  return normalizeQuaternion(quaternionMultiply(correction, baseOrientationAt(trajectory, amount)));
}

function advanceOrientation(orientation: Quaternion, angularVelocity: Vector3, seconds: number): Quaternion {
  const rotation = quaternionFromRotationVector(angularVelocity.map((value) => value * seconds) as unknown as Vector3);
  return normalizeQuaternion(quaternionMultiply(rotation, orientation));
}

function angularVelocityBetween(from: Quaternion, to: Quaternion, seconds: number): Vector3 {
  const difference = quaternionMultiply(to, quaternionConjugate(from));
  const vector = quaternionRotationVector(difference);
  return vector.map((value) => value / seconds) as unknown as Vector3;
}

function sampleOrientationAngularVelocity(trajectory: PrismIconMorphTrajectory, time: number): Vector3 {
  const amount = clamp(time);
  if (amount === 0) return trajectory.startAngularVelocity;
  if (amount === 1) {
    return trajectory.targetProgress === 0 ? [0, 0, 0] : [0, TAU * trajectory.prismVelocity, 0];
  }
  const step = Math.min(0.00001, amount, 1 - amount);
  return angularVelocityBetween(
    sampleOrientationOnly(trajectory, amount - step),
    sampleOrientationOnly(trajectory, amount + step),
    step * 2 * trajectory.durationSeconds,
  );
}

function validateFiniteVector(vector: Vector3 | undefined, label: string): void {
  if (vector === undefined) return;
  if (!Array.isArray(vector) || vector.length !== 3 || !vector.every(Number.isFinite)) {
    throw new RangeError(`${label} must be a finite three-dimensional vector.`);
  }
}

function validateOptionalQuaternion(quaternion: Quaternion | undefined): void {
  if (quaternion === undefined) return;
  const norm = Array.isArray(quaternion) ? Math.hypot(...quaternion) : NaN;
  if (norm === 0 || !Number.isFinite(norm) || !Array.isArray(quaternion) || quaternion.length !== 4 || !quaternion.every(Number.isFinite)) {
    throw new RangeError('Prism icon start orientation must be a finite, non-zero quaternion.');
  }
}

export function createPrismIconMorphTrajectory(options: {
  durationSeconds: number;
  iconSize: number;
  prismSize: number;
  prismVelocity: number;
  startAngularVelocity?: Vector3;
  startEdgeHighlightProgress?: number;
  startOrientation?: Quaternion;
  startPhase: number;
  startProgress: number;
  startVelocity: number;
  targetProgress: 0 | 1;
}): PrismIconMorphTrajectory {
  requireFinitePositive(options.durationSeconds, 'Prism icon morph duration');
  requireFinitePositive(options.iconSize, 'Prism icon size');
  requireFinitePositive(options.prismSize, 'Prism size');
  requireFinitePositive(options.prismVelocity, 'Prism velocity');
  for (const [value, label] of [
    [options.startPhase, 'Prism icon start phase'],
    [options.startProgress, 'Prism icon start progress'],
    [options.startVelocity, 'Prism icon start velocity'],
  ] as const) {
    if (!Number.isFinite(value)) throw new RangeError(`${label} must be finite.`);
  }
  if (options.startProgress < 0 || options.startProgress > 1) {
    throw new RangeError('Prism icon start progress must be between zero and one.');
  }
  if (options.startVelocity < 0) throw new RangeError('Prism icon start velocity must not be negative.');
  if (options.targetProgress !== 0 && options.targetProgress !== 1) {
    throw new RangeError('Prism icon target progress must be exactly zero or one.');
  }
  if (
    options.startProgress !== options.targetProgress &&
    options.startVelocity === 0 &&
    !(options.startProgress === 0 && options.targetProgress === 1)
  ) {
    throw new RangeError('An interrupted prism icon morph must retain a positive forward velocity.');
  }
  validateFiniteVector(options.startAngularVelocity, 'Prism icon start angular velocity');
  validateOptionalQuaternion(options.startOrientation);
  if (
    options.startEdgeHighlightProgress !== undefined &&
    (!Number.isFinite(options.startEdgeHighlightProgress) ||
      options.startEdgeHighlightProgress < 0 ||
      options.startEdgeHighlightProgress > 1)
  ) {
    throw new RangeError('Prism icon start edge highlight progress must be between zero and one.');
  }

  const requestedHighlightProgress = options.startEdgeHighlightProgress ?? 1;
  const startEdgeHighlightProgress =
    requestedHighlightProgress > EPSILON && requestedHighlightProgress < 1 - EPSILON ? requestedHighlightProgress : 1;

  const endVelocity = options.targetProgress === 1 ? options.prismVelocity : 0;
  let phaseVelocityControls: readonly number[];
  let endPhase: number;
  if (options.targetProgress === 1) {
    phaseVelocityControls = openingVelocityControls(options.startVelocity, endVelocity);
    endPhase = options.startPhase + options.durationSeconds * integratePhaseVelocity(phaseVelocityControls, 1);
  } else {
    const closing = closingVelocityControls(options.startVelocity, options.durationSeconds, options.startPhase);
    phaseVelocityControls = closing.controls;
    endPhase = closing.endPhase;
  }
  if (endPhase <= options.startPhase || !Number.isFinite(endPhase) || !phaseVelocityControls.every(Number.isFinite)) {
    throw new RangeError('Prism icon phase curve must remain finite and strictly forward.');
  }

  const preliminary: PrismIconMorphTrajectory = {
    correctionEndTime: 1,
    correctionStartDerivative: [0, 0, 0],
    correctionStartVector: [0, 0, 0],
    durationSeconds: options.durationSeconds,
    endPhase,
    iconSize: options.iconSize,
    phaseVelocityControls,
    prismSize: options.prismSize,
    prismVelocity: options.prismVelocity,
    startAngularVelocity: [0, 0, 0],
    startEdgeHighlightProgress,
    startOrientation: PRISM_ICON_IDLE_POSE.orientation,
    startPhase: options.startPhase,
    startProgress: options.startProgress,
    startVelocity: options.startVelocity,
    targetProgress: options.targetProgress,
  };

  const activeTrajectory = options.startProgress !== options.targetProgress || options.startVelocity > 0 || endVelocity > 0;
  if (activeTrajectory) {
    const boundaryStep = Math.min(1, 1 / (options.durationSeconds * PHASE_VALIDATION_RATE));
    const firstPhase = phaseAt(preliminary, boundaryStep);
    const penultimatePhase = phaseAt(preliminary, 1 - boundaryStep);
    if (firstPhase <= options.startPhase || endPhase <= penultimatePhase || ![firstPhase, penultimatePhase].every(Number.isFinite)) {
      throw new RangeError('Prism icon phase must advance representably across its 240 Hz boundary intervals.');
    }
  }

  const baseStart = baseOrientationAt(preliminary, 0);
  const startOrientation = options.startOrientation ?? baseStart;
  const correctionStartVector = quaternionRotationVector(quaternionMultiply(startOrientation, quaternionConjugate(baseStart)));
  const derivativeStep = 0.00001;
  const baseFuture = baseOrientationAt(preliminary, derivativeStep);
  const baseAngularVelocity = angularVelocityBetween(baseStart, baseFuture, derivativeStep * options.durationSeconds);
  const startAngularVelocity = options.startAngularVelocity ?? baseAngularVelocity;
  const desiredFuture = advanceOrientation(startOrientation, startAngularVelocity, derivativeStep * options.durationSeconds);
  const futureCorrection = quaternionRotationVector(quaternionMultiply(desiredFuture, quaternionConjugate(baseFuture)));
  const correctionStartDerivative: Vector3 = [
    (futureCorrection[0] - correctionStartVector[0]) / derivativeStep,
    (futureCorrection[1] - correctionStartVector[1]) / derivativeStep,
    (futureCorrection[2] - correctionStartVector[2]) / derivativeStep,
  ];
  if (![...startOrientation, ...startAngularVelocity, ...correctionStartVector, ...correctionStartDerivative].every(Number.isFinite)) {
    throw new RangeError('Prism icon orientation correction must remain finite.');
  }

  return {
    ...preliminary,
    correctionStartDerivative,
    correctionStartVector,
    startAngularVelocity,
    startOrientation,
  };
}

export function samplePrismIconMorphTrajectory(trajectory: PrismIconMorphTrajectory, elapsedProgress: number): PrismIconMorphSample {
  if (!Number.isFinite(elapsedProgress)) throw new RangeError('Prism icon elapsed progress must be finite.');
  const time = clamp(elapsedProgress);
  const progress = progressAt(trajectory, time);
  const timelineSeconds = progress * PRISM_ICON_DURATION_SECONDS;
  const scaleProgress = smoothstep(progress);
  const settledRenderScale = trajectory.prismSize / 92;
  const renderScale =
    scaleProgress <= 0
      ? PRISM_ICON_IDLE_RENDER_SCALE
      : scaleProgress >= 1
        ? settledRenderScale
        : Math.exp(lerp(Math.log(PRISM_ICON_IDLE_RENDER_SCALE), Math.log(settledRenderScale), scaleProgress));
  const surface = transitionWindow(timelineSeconds, 0.55, 1.55);
  const resumedHighlight = trajectory.startEdgeHighlightProgress < 1 - EPSILON;
  const resumedHighlightDuration = Math.min((trajectory.startEdgeHighlightProgress * 0.9) / trajectory.durationSeconds, 1);
  const edgeHighlightStartSeconds = trajectory.targetProgress === 1 ? 0.15 : 1.15;
  const edgeHighlightEndSeconds = trajectory.targetProgress === 1 ? 1.05 : 2.05;
  const edgeHighlightProgress = resumedHighlight
    ? trajectory.startEdgeHighlightProgress * (1 - linearWindow(time, 0, resumedHighlightDuration))
    : 1 - linearWindow(time * PRISM_ICON_DURATION_SECONDS, edgeHighlightStartSeconds, edgeHighlightEndSeconds);
  const sample: PrismIconMorphSample = {
    angularVelocity: sampleOrientationAngularVelocity(trajectory, time),
    cameraDistance: lerp(PRISM_ICON_CAMERA_DISTANCE, PRISM_SETTLED_CAMERA_DISTANCE, scaleProgress),
    edgeAlpha: 1 - transitionWindow(timelineSeconds, 0.2, 1.55),
    edgeHighlightProgress,
    edgeWidthCssPixels: lerp((trajectory.iconSize * 2) / 24, 1.3, transitionWindow(timelineSeconds, 0, 1.55)),
    optics: surface,
    orientation: sampleOrientationOnly(trajectory, time),
    phase: phaseAt(trajectory, time),
    progress,
    projectionDistance: lerp(PRISM_ICON_PROJECTION_DISTANCE, PRISM_SETTLED_PROJECTION_DISTANCE, scaleProgress),
    renderScale,
    spectrumPassProgress: linearWindow(timelineSeconds, 1.35, 1.75),
    surface,
    surfaceReveal: transitionWindow(timelineSeconds, 0.2, 1.2),
    velocity: samplePhaseVelocity(trajectory.phaseVelocityControls, time),
  };
  if (
    ![
      sample.cameraDistance,
      sample.edgeAlpha,
      sample.edgeHighlightProgress,
      sample.edgeWidthCssPixels,
      sample.optics,
      sample.phase,
      sample.progress,
      sample.projectionDistance,
      sample.renderScale,
      sample.spectrumPassProgress,
      sample.surface,
      sample.surfaceReveal,
      sample.velocity,
      ...sample.orientation,
      ...sample.angularVelocity,
    ].every(Number.isFinite)
  ) {
    throw new RangeError('Prism icon morph sample must remain finite.');
  }
  return sample;
}
