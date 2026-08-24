/* eslint-disable @typescript-eslint/no-non-null-assertion -- Fixed topology tables and validated matrix dimensions make indexed entries total in this geometry kernel. */

const TAU = Math.PI * 2;
const SPINNER_ORIENTATION_OFFSET = 0.62;
const SPINNER_ORIENTATION_TURNS = SPINNER_ORIENTATION_OFFSET / TAU;
const EPSILON = 0.000001;
export const PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS = 0.72;
export const PRISM_SPINNER_SILHOUETTE_PHASE_TURNS = 0.25;
export const PRISM_SPINNER_CSS_SIZE = 18;
const PRISM_SPINNER_SWAP_SCALE = 1.11118849;
const PRISM_SPINNER_SIZE_COMPLETION_PROGRESS = 5 / 11;

type Vector3 = readonly [number, number, number];
type Triangle = readonly [number, number, number];
type Quaternion = readonly [number, number, number, number];

const PRISM_X = 0.735 / (Math.sqrt(3) * 0.5);
export const PRISM_VERTICES: readonly Vector3[] = Object.freeze([
  [PRISM_X, -0.49, -0.58],
  [-PRISM_X, -0.49, 0.58],
  [0, 0.98, -0.58],
  [PRISM_X, -0.49, 0.58],
  [0, 0.98, 0.58],
  [-PRISM_X, -0.49, -0.58],
]);
const OCTAHEDRON_RADIUS = 0.98;
const OCTAHEDRON_VERTICES: readonly Vector3[] = Object.freeze([
  [OCTAHEDRON_RADIUS, 0, 0],
  [-OCTAHEDRON_RADIUS, 0, 0],
  [0, OCTAHEDRON_RADIUS, 0],
  [0, -OCTAHEDRON_RADIUS, 0],
  [0, 0, OCTAHEDRON_RADIUS],
  [0, 0, -OCTAHEDRON_RADIUS],
]);
const MORPH_TRIANGLES: readonly Triangle[] = Object.freeze([
  [4, 0, 2],
  [2, 0, 5],
  [3, 0, 4],
  [5, 0, 3],
  [2, 1, 4],
  [5, 1, 2],
  [4, 1, 3],
  [3, 1, 5],
]);

function clamp(value: number, minimum = 0, maximum = 1): number {
  return Math.max(minimum, Math.min(maximum, value));
}

export function normalizeSpinnerPhase(phase: number): number {
  if (!Number.isFinite(phase)) return 0;
  return ((phase % 1) + 1) % 1;
}

export function spinnerSourceWorldRotationPhase(worldPhase: number): number {
  return worldPhase || 0;
}

function requireFrameCount(frameCount: number): number {
  if (!Number.isSafeInteger(frameCount) || frameCount <= 0) {
    throw new RangeError('Spinner frame count must be a positive integer.');
  }
  return frameCount;
}

function smootherstep(value: number): number {
  const normalized = clamp(value);
  return normalized * normalized * normalized * (normalized * (normalized * 6 - 15) + 10);
}

function transitionWindow(progress: number, start: number, end: number): number {
  return smootherstep((clamp(progress) - start) / Math.max(end - start, EPSILON));
}

function subtract(left: Vector3, right: Vector3): Vector3 {
  return [left[0] - right[0], left[1] - right[1], left[2] - right[2]];
}

function cross(left: Vector3, right: Vector3): Vector3 {
  return [left[1] * right[2] - left[2] * right[1], left[2] * right[0] - left[0] * right[2], left[0] * right[1] - left[1] * right[0]];
}

function dot(left: Vector3, right: Vector3): number {
  return left[0] * right[0] + left[1] * right[1] + left[2] * right[2];
}

function normalize(vector: Vector3): Vector3 {
  const length = Math.hypot(...vector) || 1;
  return [vector[0] / length, vector[1] / length, vector[2] / length];
}

function quaternionMultiply(left: Quaternion, right: Quaternion): Quaternion {
  return [
    left[3] * right[0] + right[3] * left[0] + left[1] * right[2] - left[2] * right[1],
    left[3] * right[1] + right[3] * left[1] + left[2] * right[0] - left[0] * right[2],
    left[3] * right[2] + right[3] * left[2] + left[0] * right[1] - left[1] * right[0],
    left[3] * right[3] - left[0] * right[0] - left[1] * right[1] - left[2] * right[2],
  ];
}

function quaternionFromAxisAngle(axis: Vector3, angle: number): Quaternion {
  const unit = normalize(axis);
  const sine = Math.sin(angle * 0.5);
  return [unit[0] * sine, unit[1] * sine, unit[2] * sine, Math.cos(angle * 0.5)];
}

function quaternionConjugate(quaternion: Quaternion): Quaternion {
  return [-quaternion[0], -quaternion[1], -quaternion[2], quaternion[3]];
}

function normalizeQuaternion(quaternion: Quaternion): Quaternion {
  const length = Math.hypot(...quaternion) || 1;
  return [quaternion[0] / length, quaternion[1] / length, quaternion[2] / length, quaternion[3] / length];
}

function principalRotationVector(quaternion: Quaternion): Vector3 {
  let normalized = normalizeQuaternion(quaternion);
  if (normalized[3] < 0) normalized = normalized.map((value) => -value) as unknown as Quaternion;
  const sine = Math.hypot(normalized[0], normalized[1], normalized[2]);
  if (sine < EPSILON) return [0, 0, 0];
  const angle = 2 * Math.atan2(sine, clamp(normalized[3], -1, 1));
  return [(normalized[0] / sine) * angle, (normalized[1] / sine) * angle, (normalized[2] / sine) * angle];
}

function quaternionFromRotationVector(vector: Vector3): Quaternion {
  const angle = Math.hypot(...vector);
  if (angle < EPSILON) {
    return normalizeQuaternion([vector[0] * 0.5, vector[1] * 0.5, vector[2] * 0.5, 1]);
  }
  return quaternionFromAxisAngle([vector[0] / angle, vector[1] / angle, vector[2] / angle], angle);
}

function solveLinearSystem(matrix: number[][], values: number[]): number[] {
  const size = values.length;
  for (let column = 0; column < size; column += 1) {
    let pivot = column;
    for (let row = column + 1; row < size; row += 1) {
      if (Math.abs(matrix[row]![column]!) > Math.abs(matrix[pivot]![column]!)) pivot = row;
    }
    const columnRow = matrix[column]!;
    const pivotRow = matrix[pivot]!;
    matrix[column] = pivotRow;
    matrix[pivot] = columnRow;
    const columnValue = values[column]!;
    values[column] = values[pivot]!;
    values[pivot] = columnValue;
    const divisor = matrix[column]![column]!;
    if (Math.abs(divisor) < EPSILON) throw new RangeError('Linear system is singular.');
    for (let entry = column; entry < size; entry += 1) matrix[column]![entry]! /= divisor;
    values[column]! /= divisor;
    for (let row = 0; row < size; row += 1) {
      if (row === column) continue;
      const factor = matrix[row]![column]!;
      for (let entry = column; entry < size; entry += 1) {
        matrix[row]![entry]! -= factor * matrix[column]![entry]!;
      }
      values[row]! -= factor * values[column]!;
    }
  }
  return values;
}

export type PrismSpinnerPolynomialCurve = {
  durationSeconds: number;
  startPhase: number;
  velocityControls: readonly number[];
};

export type PrismSpinnerOrientationCurve = {
  correctionCoefficients: readonly Vector3[];
  correctionOrigin: Quaternion;
  durationSeconds: number;
  endSpinner: boolean;
  spinCurve: PrismSpinnerPolynomialCurve;
  spinnerOffset: Quaternion;
  startSpinner: boolean;
  swapTime: number | null;
};

export type FixedPrismSpinnerTrajectory = {
  curve: PrismSpinnerPolynomialCurve;
  durationSeconds: number;
  endPhase: number;
  endVelocity: number;
  handoff: SpinnerHandoff | null;
  matchPhase: number;
  orientation: PrismSpinnerOrientationCurve;
  swapTime: number | null;
  startProgress: number;
  targetProgress: number;
};

type FixedPrismSpinnerTrajectoryOptions = {
  frameCount: number;
  prismVelocity: number;
  spinnerVelocity: number;
  startAngularVelocity?: Vector3;
  startOrientation?: Quaternion;
  startPhase: number;
  startProgress: number;
  startVelocity: number;
  targetProgress: number;
  totalDurationSeconds: number;
};

function bernsteinWeights(degree: number, time: number): number[] {
  const amount = clamp(time);
  const weights = Array.from({ length: degree + 1 }, () => 0);
  if (amount <= 0) {
    weights[0] = 1;
    return weights;
  }
  if (amount >= 1) {
    weights[degree] = 1;
    return weights;
  }
  const remaining = 1 - amount;
  const ratio = amount / remaining;
  weights[0] = remaining ** degree;
  for (let index = 1; index <= degree; index += 1) {
    weights[index] = ((weights[index - 1]! * (degree - index + 1)) / index) * ratio;
  }
  return weights;
}

function integratedBernsteinWeights(degree: number, time: number): number[] {
  const elevated = bernsteinWeights(degree + 1, time);
  const weights = Array.from({ length: degree + 1 }, () => 0);
  let tail = 0;
  for (let index = degree; index >= 0; index -= 1) {
    tail += elevated[index + 1]!;
    weights[index] = tail / (degree + 1);
  }
  return weights;
}

function sampleVelocityControls(controls: readonly number[], time: number): number {
  const weights = bernsteinWeights(controls.length - 1, time);
  return controls.reduce((sum, control, index) => sum + control * weights[index]!, 0);
}

function integrateVelocityControls(controls: readonly number[], time: number): number {
  const weights = integratedBernsteinWeights(controls.length - 1, time);
  return controls.reduce((sum, control, index) => sum + control * weights[index]!, 0);
}

const TRAJECTORY_VELOCITY_SAMPLING_WEIGHTS = Array.from({ length: 33 }, (_, index) => bernsteinWeights(7, index / 32));

function createVelocityControls(
  startPhase: number,
  waypointPhase: number,
  endPhase: number,
  startVelocity: number,
  endVelocity: number,
  waypointTime: number,
  durationSeconds: number,
): number[] | null {
  const controls = [startVelocity, 0, 0, 0, 0, 0, 0, endVelocity];
  const unknownIndices = [1, 2, 3, 4, 5, 6];
  const target = unknownIndices.map((index) => startVelocity + (endVelocity - startVelocity) * smootherstep(index / 7));
  const totalWeights = unknownIndices.map(() => durationSeconds / 8);
  const integratedWaypointWeights = integratedBernsteinWeights(7, waypointTime);
  const waypointWeights = unknownIndices.map((index) => durationSeconds * integratedWaypointWeights[index]!);
  const fixedIndices = [0, 7];
  const totalTarget = endPhase - startPhase - fixedIndices.reduce((sum, index) => sum + (controls[index]! * durationSeconds) / 8, 0);
  const waypointTarget =
    waypointPhase -
    startPhase -
    fixedIndices.reduce((sum, index) => sum + controls[index]! * durationSeconds * integratedWaypointWeights[index]!, 0);
  const residual = [
    totalTarget - totalWeights.reduce((sum, weight, index) => sum + weight * target[index]!, 0),
    waypointTarget - waypointWeights.reduce((sum, weight, index) => sum + weight * target[index]!, 0),
  ];
  const gram00 = totalWeights.reduce((sum, value) => sum + value * value, 0);
  const gram01 = totalWeights.reduce((sum, value, index) => sum + value * waypointWeights[index]!, 0);
  const gram11 = waypointWeights.reduce((sum, value) => sum + value * value, 0);
  const determinant = gram00 * gram11 - gram01 * gram01;
  if (Math.abs(determinant) < EPSILON) return null;
  const lambda0 = (residual[0]! * gram11 - residual[1]! * gram01) / determinant;
  const lambda1 = (residual[1]! * gram00 - residual[0]! * gram01) / determinant;
  for (const [index, controlIndex] of unknownIndices.entries()) {
    controls[controlIndex] = target[index]! + totalWeights[index]! * lambda0 + waypointWeights[index]! * lambda1;
  }
  return controls.every((velocity) => Number.isFinite(velocity) && velocity > 0.02 && velocity < 1.2) ? controls : null;
}

function createEndpointVelocityControls(
  startPhase: number,
  endPhase: number,
  startVelocity: number,
  endVelocity: number,
  durationSeconds: number,
): number[] | null {
  const controls = [startVelocity, 0, 0, 0, 0, 0, 0, endVelocity];
  const unknownIndices = [1, 2, 3, 4, 5, 6];
  const target = unknownIndices.map((index) => startVelocity + (endVelocity - startVelocity) * smootherstep(index / 7));
  const weight = durationSeconds / 8;
  const fixedTravel = (controls[0]! + controls[7]!) * weight;
  const residual = endPhase - startPhase - fixedTravel - target.reduce((sum, velocity) => sum + velocity * weight, 0);
  const adjustment = residual / (unknownIndices.length * weight);
  for (const [index, controlIndex] of unknownIndices.entries()) {
    controls[controlIndex] = target[index]! + adjustment;
  }
  return controls.every((velocity) => Number.isFinite(velocity) && velocity > 0.08 && velocity < 0.82) ? controls : null;
}

function trajectoryVelocityScore(controls: readonly number[], startVelocity: number, endVelocity: number): number {
  let score = 0;
  let previousVelocity = controls[0]!;
  let previousDirection = 0;
  let directionChanges = 0;
  for (let index = 0; index <= 32; index += 1) {
    const amount = index / 32;
    const velocity = controls.reduce(
      (sum, control, controlIndex) => sum + control * TRAJECTORY_VELOCITY_SAMPLING_WEIGHTS[index]![controlIndex]!,
      0,
    );
    if (!Number.isFinite(velocity) || velocity <= 0.02 || velocity >= 1.2) return Infinity;
    if (index > 0) {
      const delta = velocity - previousVelocity;
      const direction = Math.abs(delta) < 0.00001 ? 0 : Math.sign(delta);
      if (direction !== 0 && previousDirection !== 0 && direction !== previousDirection) directionChanges += 1;
      if (direction !== 0) previousDirection = direction;
    }
    previousVelocity = velocity;
    const expected = startVelocity + (endVelocity - startVelocity) * smootherstep(amount);
    score += (velocity - expected) ** 2;
  }
  // A single acceleration-to-deceleration turn reads as one smooth speed arc.
  // More turns are the actual brake/restart behavior reported during closing,
  // regardless of the curve's absolute peak speed.
  return directionChanges <= 1 ? score : Infinity;
}

function forwardSilhouetteCandidates(startPhase: number, naturalTravel: number): number[] {
  const candidates = new Map<string, number>();
  for (let attempt = -2; attempt < 5; attempt += 1) {
    const phase = nextForwardSpinnerSilhouettePhase(startPhase, Math.max(naturalTravel + attempt * 0.5, 0.02));
    candidates.set(phase.toFixed(12), phase);
  }
  return [...candidates.values()];
}

export function prismOrientation(phase: number): Quaternion {
  return normalizeQuaternion(
    quaternionMultiply(
      quaternionMultiply(quaternionFromAxisAngle([0, 1, 0], phase * TAU), quaternionFromAxisAngle([1, 0, 0], -0.3)),
      quaternionFromAxisAngle([0, 0, 1], 0.1),
    ),
  );
}

function spinnerOrientation(phase: number): Quaternion {
  return normalizeQuaternion(
    quaternionMultiply(
      quaternionMultiply(quaternionFromAxisAngle([0, 0, 1], -0.03), quaternionFromAxisAngle([1, 0, 0], -0.11)),
      quaternionFromAxisAngle([0, 1, 0], spinnerSourceWorldRotationPhase(phase) * TAU),
    ),
  );
}

function matchedOrientations(phase: number): {
  prism: Quaternion;
  spinner: Quaternion;
} {
  const rotationPhase = spinnerSourceWorldRotationPhase(phase);
  const halfTurn = Math.round((rotationPhase - PRISM_SPINNER_SILHOUETTE_PHASE_TURNS) / 0.5);
  const opposite = ((halfTurn % 2) + 2) % 2 === 1;
  const prismEuler = opposite
    ? ([4.538357686911922, -0.36307344990699775, 0.5859565030498252] as const)
    : ([7.655983625573993, -0.6043348316624402, -0.41008976644368156] as const);
  const spinnerEuler = opposite
    ? ([0.26242631851598863, -0.5043809145361788, 4.1276226515921355] as const)
    : ([-0.016989387322692104, 0.5043809158044072, 7.26921530201536] as const);
  const matchedPrism = normalizeQuaternion(
    quaternionMultiply(
      quaternionMultiply(quaternionFromAxisAngle([0, 1, 0], prismEuler[0]), quaternionFromAxisAngle([1, 0, 0], prismEuler[1])),
      quaternionFromAxisAngle([0, 0, 1], prismEuler[2]),
    ),
  );
  const matchedSpinner = normalizeQuaternion(
    quaternionMultiply(
      quaternionMultiply(quaternionFromAxisAngle([0, 0, 1], spinnerEuler[0]), quaternionFromAxisAngle([1, 0, 0], spinnerEuler[1])),
      quaternionFromAxisAngle([0, 1, 0], spinnerEuler[2]),
    ),
  );
  return {
    prism: matchedPrism,
    spinner: matchedSpinner,
  };
}

function angularVelocityForPhase(orientationAt: (phase: number) => Quaternion, phase: number, phaseVelocity: number): Vector3 {
  const epsilon = 0.00001;
  const before = orientationAt(phase - epsilon);
  const after = orientationAt(phase + epsilon);
  const delta = quaternionMultiply(after, quaternionConjugate(before));
  const rotation = principalRotationVector(delta);
  const scale = phaseVelocity / (2 * epsilon);
  return [rotation[0] * scale, rotation[1] * scale, rotation[2] * scale];
}

function orientationCorrection(orientation: Quaternion, phase: number): Quaternion {
  return normalizeQuaternion(quaternionMultiply(quaternionConjugate(quaternionFromAxisAngle([0, 1, 0], phase * TAU)), orientation));
}

function advanceOrientation(orientation: Quaternion, angularVelocity: Vector3, seconds: number): Quaternion {
  const speed = Math.hypot(...angularVelocity);
  if (speed < EPSILON || Math.abs(seconds) < EPSILON) return orientation;
  return normalizeQuaternion(
    quaternionMultiply(
      quaternionFromAxisAngle([angularVelocity[0] / speed, angularVelocity[1] / speed, angularVelocity[2] / speed], speed * seconds),
      orientation,
    ),
  );
}

function commonCorrection(orientation: Quaternion, phase: number, spinner: boolean, spinnerOffset: Quaternion): Quaternion {
  const commonOrientation = spinner ? quaternionMultiply(orientation, quaternionConjugate(spinnerOffset)) : orientation;
  return orientationCorrection(commonOrientation, phase);
}

function relativeCorrectionVector(origin: Quaternion, correction: Quaternion): Vector3 {
  return principalRotationVector(quaternionMultiply(quaternionConjugate(origin), correction));
}

function createCorrectionCoefficients(
  end: Vector3,
  startDerivative: Vector3,
  endDerivative: Vector3,
  waypoint: Vector3 | null,
  waypointTime: number | null,
): readonly Vector3[] {
  const zero: Vector3 = [0, 0, 0];
  if (waypoint === null || waypointTime === null) {
    const quadratic = end.map((value, index) => 3 * value - 2 * startDerivative[index]! - endDerivative[index]!) as unknown as Vector3;
    const cubic = end.map((value, index) => -2 * value + startDerivative[index]! + endDerivative[index]!) as unknown as Vector3;
    return [zero, startDerivative, quadratic, cubic];
  }

  const time = clamp(waypointTime, 0.02, 0.98);
  const squared = time * time;
  const cubed = squared * time;
  const fourth = cubed * time;
  const coefficients = [zero, startDerivative, zero, zero, zero] as Vector3[];
  for (let component = 0; component < 3; component += 1) {
    const [quadratic, cubic, quartic] = solveLinearSystem(
      [
        [1, 1, 1],
        [2, 3, 4],
        [squared, cubed, fourth],
      ],
      [
        end[component]! - startDerivative[component]!,
        endDerivative[component]! - startDerivative[component]!,
        waypoint[component]! - startDerivative[component]! * time,
      ],
    );
    coefficients[2] = coefficients[2]!.map((value, index) => (index === component ? quadratic! : value)) as unknown as Vector3;
    coefficients[3] = coefficients[3]!.map((value, index) => (index === component ? cubic! : value)) as unknown as Vector3;
    coefficients[4] = coefficients[4]!.map((value, index) => (index === component ? quartic! : value)) as unknown as Vector3;
  }
  return coefficients;
}

function sampleCorrectionVector(coefficients: readonly Vector3[], time: number): Vector3 {
  const amount = clamp(time);
  return coefficients.reduce(
    (sum, coefficient, degree) =>
      [
        sum[0] + coefficient[0] * amount ** degree,
        sum[1] + coefficient[1] * amount ** degree,
        sum[2] + coefficient[2] * amount ** degree,
      ] as Vector3,
    [0, 0, 0] as Vector3,
  );
}

function createOrientationCurve(
  options: FixedPrismSpinnerTrajectoryOptions,
  durationSeconds: number,
  velocityControls: readonly number[],
  endPhase: number,
  endVelocity: number,
  matchPhase: number,
  swapTime: number | null,
): PrismSpinnerOrientationCurve {
  const startProgress = clamp(Number(options.startProgress) || 0);
  const targetProgress = clamp(Number(options.targetProgress) || 0);
  const startPhase = Number(options.startPhase) || 0;
  const startSpinner = startProgress >= PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS;
  const startFamily = startSpinner ? spinnerOrientation : prismOrientation;
  const endSpinner = targetProgress >= PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS;
  const endFamily = endSpinner ? spinnerOrientation : prismOrientation;
  const startOrientation = normalizeQuaternion(options.startOrientation ?? startFamily(startPhase));
  const endOrientation = endFamily(endPhase);
  const startAngularVelocity = options.startAngularVelocity ?? angularVelocityForPhase(startFamily, startPhase, options.startVelocity);
  const endAngularVelocity = angularVelocityForPhase(endFamily, endPhase, endVelocity);
  const spinCurve = { durationSeconds, startPhase, velocityControls };
  const bridge = swapTime === null ? null : matchedOrientations(matchPhase);
  const spinnerOffset =
    bridge === null
      ? ([0, 0, 0, 1] as Quaternion)
      : normalizeQuaternion(quaternionMultiply(quaternionConjugate(bridge.prism), bridge.spinner));
  const correctionOrigin = commonCorrection(startOrientation, startPhase, startSpinner, spinnerOffset);
  const endCorrection = commonCorrection(endOrientation, endPhase, endSpinner, spinnerOffset);
  const endCorrectionVector = relativeCorrectionVector(correctionOrigin, endCorrection);
  const derivativeStep = 0.00001;
  const derivativeSeconds = derivativeStep * durationSeconds;
  const startCorrectionAhead = commonCorrection(
    advanceOrientation(startOrientation, startAngularVelocity, derivativeSeconds),
    startPhase + options.startVelocity * derivativeSeconds,
    startSpinner,
    spinnerOffset,
  );
  const startDerivative = relativeCorrectionVector(correctionOrigin, startCorrectionAhead).map(
    (value) => value / derivativeStep,
  ) as unknown as Vector3;
  const endOrientationBehind = advanceOrientation(endOrientation, endAngularVelocity, -derivativeSeconds);
  const endCorrectionBehind = commonCorrection(endOrientationBehind, endPhase - endVelocity * derivativeSeconds, endSpinner, spinnerOffset);
  const endVectorBehind = relativeCorrectionVector(correctionOrigin, endCorrectionBehind);
  const endDerivative = endCorrectionVector.map((value, index) => (value - endVectorBehind[index]!) / derivativeStep) as unknown as Vector3;
  const waypointCorrection = bridge === null ? null : orientationCorrection(bridge.prism, matchPhase);
  const waypointVector = waypointCorrection === null ? null : relativeCorrectionVector(correctionOrigin, waypointCorrection);
  return {
    correctionCoefficients: createCorrectionCoefficients(endCorrectionVector, startDerivative, endDerivative, waypointVector, swapTime),
    correctionOrigin,
    durationSeconds,
    endSpinner,
    spinCurve,
    spinnerOffset,
    startSpinner,
    swapTime,
  };
}

function sampleOrientation(curve: PrismSpinnerOrientationCurve, time: number): Quaternion {
  const amount = clamp(time);
  const phase = curve.spinCurve.startPhase + curve.durationSeconds * integrateVelocityControls(curve.spinCurve.velocityControls, amount);
  const correction = normalizeQuaternion(
    quaternionMultiply(curve.correctionOrigin, quaternionFromRotationVector(sampleCorrectionVector(curve.correctionCoefficients, amount))),
  );
  const commonOrientation = normalizeQuaternion(quaternionMultiply(quaternionFromAxisAngle([0, 1, 0], phase * TAU), correction));
  const spinner = curve.swapTime === null || amount < curve.swapTime ? curve.startSpinner : curve.endSpinner;
  return spinner ? normalizeQuaternion(quaternionMultiply(commonOrientation, curve.spinnerOffset)) : commonOrientation;
}

function orientationSamplingInterval(curve: PrismSpinnerOrientationCurve, time: number): readonly [number, number] {
  if (curve.swapTime === null) return [0, 1];
  return time < curve.swapTime ? [0, curve.swapTime - EPSILON * 0.001] : [curve.swapTime, 1];
}

function sampleOrientationAngularVelocity(curve: PrismSpinnerOrientationCurve, time: number): Vector3 {
  const amount = clamp(time);
  const duration = Math.max(curve.durationSeconds, EPSILON);
  const epsilon = 0.00002;
  const [intervalStart, intervalEnd] = orientationSamplingInterval(curve, amount);
  const beforeTime = Math.max(amount - epsilon, intervalStart);
  const afterTime = Math.min(amount + epsilon, intervalEnd);
  const before = sampleOrientation(curve, beforeTime);
  const after = sampleOrientation(curve, afterTime);
  const angularDisplacement = principalRotationVector(quaternionMultiply(after, quaternionConjugate(before)));
  const angularScale = 1 / Math.max((afterTime - beforeTime) * duration, EPSILON);
  return [angularDisplacement[0] * angularScale, angularDisplacement[1] * angularScale, angularDisplacement[2] * angularScale];
}

function meaningfulDirectionChanges(values: readonly number[], minimumExcursion: number): number {
  if (values.length < 2) return 0;
  let direction = 0;
  let extreme = values[0]!;
  let changes = 0;
  for (const value of values.slice(1)) {
    if (direction >= 0) {
      if (value > extreme) {
        extreme = value;
      } else if (extreme - value >= minimumExcursion) {
        if (direction > 0) changes += 1;
        direction = -1;
        extreme = value;
      }
    }
    if (direction <= 0) {
      if (value < extreme) {
        extreme = value;
      } else if (value - extreme >= minimumExcursion) {
        if (direction < 0) changes += 1;
        direction = 1;
        extreme = value;
      }
    }
  }
  return changes;
}

function orientationCurveScore(curve: PrismSpinnerOrientationCurve): number {
  const samples = Array.from({ length: 25 }, (_, index) => sampleOrientationAngularVelocity(curve, index / 24));
  const speeds = samples.map((velocity) => Math.hypot(...velocity));
  if (speeds.some((speed) => !Number.isFinite(speed) || speed < EPSILON)) {
    return Infinity;
  }
  const speedDirectionChanges = meaningfulDirectionChanges(speeds, 0.02);
  const axes = samples.map((velocity, index): Vector3 => [
    velocity[0] / speeds[index]!,
    velocity[1] / speeds[index]!,
    velocity[2] / speeds[index]!,
  ]);
  const minimumForwardAxis = Math.min(...axes.map((axis) => axis[1]));
  const maximumAxisStep = Math.max(
    ...axes.slice(1).map((axis, index) => Math.hypot(axis[0] - axes[index]![0], axis[1] - axes[index]![1], axis[2] - axes[index]![2])),
  );
  const axisPathLength = axes
    .slice(1)
    .reduce((sum, axis, index) => sum + Math.hypot(axis[0] - axes[index]![0], axis[1] - axes[index]![1], axis[2] - axes[index]![2]), 0);
  const axisEndpointDistance = Math.hypot(axes.at(-1)![0] - axes[0]![0], axes.at(-1)![1] - axes[0]![1], axes.at(-1)![2] - axes[0]![2]);
  const axisDetour = Math.max(axisPathLength - axisEndpointDistance, 0);
  const endpointSpeed = Math.max(speeds[0]!, speeds.at(-1)!, EPSILON);
  const speedRatio = Math.max(...speeds) / endpointSpeed;
  // Prefer a calm forward axis first; phase-speed similarity only breaks ties
  // between trajectories with comparable ride quality.
  return (
    Math.max(0, 0.9 - minimumForwardAxis) * 1000 +
    Math.max(0, maximumAxisStep - 0.05) * 1000 +
    axisDetour * 2000 +
    Math.max(0, speedDirectionChanges - 1) * 100_000 +
    Math.max(0, speedRatio - 1.55) * 40
  );
}

export function createFixedPrismSpinnerTrajectory(options: FixedPrismSpinnerTrajectoryOptions): FixedPrismSpinnerTrajectory {
  const frameCount = requireFrameCount(options.frameCount);
  const startProgress = clamp(Number(options.startProgress) || 0);
  const targetProgress = clamp(Number(options.targetProgress) || 0);
  const durationSeconds = Math.max(Number(options.totalDurationSeconds) || 0, EPSILON);
  const startPhase = Number(options.startPhase) || 0;
  const prismVelocity = Math.max(Number(options.prismVelocity) || 0, EPSILON);
  const spinnerVelocity = Math.max(Number(options.spinnerVelocity) || 0, EPSILON);
  const startVelocity = Math.max(Number(options.startVelocity) || 0, EPSILON);
  const opening = targetProgress > startProgress;
  const endVelocity = targetProgress >= 1 - EPSILON ? spinnerVelocity : prismVelocity;
  const crossesSwap =
    Math.min(startProgress, targetProgress) < PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS &&
    Math.max(startProgress, targetProgress) >= PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS;
  const nominalSwapTime = crossesSwap ? (PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS - startProgress) / (targetProgress - startProgress) : null;

  if (!crossesSwap) {
    let endPhase = startPhase + durationSeconds * (startVelocity + endVelocity) * 0.5;
    let handoff: SpinnerHandoff | null = null;
    if (targetProgress >= 1 - EPSILON) {
      handoff = nextForwardSpinnerFrame(startPhase, durationSeconds * (startVelocity + endVelocity) * 0.5, frameCount);
      endPhase = handoff.worldPhase;
    }
    const velocityControls = createEndpointVelocityControls(startPhase, endPhase, startVelocity, endVelocity, durationSeconds);
    if (!velocityControls) throw new RangeError('Unable to create a monotonic prism-spinner trajectory.');
    return {
      curve: { durationSeconds, startPhase, velocityControls },
      durationSeconds,
      endPhase,
      endVelocity,
      handoff,
      matchPhase: startPhase,
      orientation: createOrientationCurve(options, durationSeconds, velocityControls, endPhase, endVelocity, startPhase, null),
      swapTime: null,
      startProgress,
      targetProgress,
    };
  }

  const naturalTotalTravel = durationSeconds * (startVelocity + endVelocity) * 0.5;
  const naturalSwapTravel = naturalTotalTravel * nominalSwapTime!;
  type Candidate = {
    endPhase: number;
    handoff: SpinnerHandoff | null;
    matchPhase: number;
    score: number;
    swapTime: number;
    velocityControls: number[];
  };
  let best: Candidate | null = null;
  const matchCandidates: Candidate[] = [];

  const earliestSwapTime = 0.06;
  const latestSwapTime = 0.94;
  const flexibleSwapTimes = [
    ...new Set(
      [nominalSwapTime!, 0.5, ...Array.from({ length: 12 }, (_, index) => earliestSwapTime + index * 0.08)].map((value) =>
        clamp(value, earliestSwapTime, latestSwapTime).toFixed(6),
      ),
    ),
  ].map(Number);
  const swapTimes = flexibleSwapTimes;

  for (const matchPhase of forwardSilhouetteCandidates(startPhase, naturalSwapTravel)) {
    const bestForMatch: Candidate[] = [];
    const remainingNaturalTravel = Math.max(naturalTotalTravel - (matchPhase - startPhase), 0.04);
    for (let travelOffset = -16; travelOffset <= 40; travelOffset += 2) {
      const extraTravel = travelOffset * 0.04;
      const handoff = opening
        ? nextForwardSpinnerFrame(matchPhase, Math.max(remainingNaturalTravel + extraTravel, 0.02), frameCount)
        : null;
      let endPhase = handoff?.worldPhase ?? startPhase + Math.max(naturalTotalTravel + extraTravel, 0.02);
      while (endPhase <= matchPhase + EPSILON) endPhase += 0.5;
      for (const candidateSwapTime of swapTimes) {
        const velocityControls = createVelocityControls(
          startPhase,
          matchPhase,
          endPhase,
          startVelocity,
          endVelocity,
          candidateSwapTime,
          durationSeconds,
        );
        if (!velocityControls) continue;
        const score =
          trajectoryVelocityScore(velocityControls, startVelocity, endVelocity) +
          Math.abs(endPhase - startPhase - naturalTotalTravel) * 0.08 +
          Math.abs(candidateSwapTime - nominalSwapTime!) * 0.035;
        const candidate = {
          endPhase,
          handoff,
          matchPhase,
          score,
          swapTime: candidateSwapTime,
          velocityControls,
        };
        bestForMatch.push(candidate);
      }
    }
    const candidateLimit = opening ? 4 : 8;
    bestForMatch.sort((left, right) => left.score - right.score);
    matchCandidates.push(...bestForMatch.slice(0, candidateLimit));
  }

  matchCandidates.sort((left, right) => left.score - right.score);
  for (const candidate of matchCandidates) {
    if (best && candidate.score >= best.score) break;
    const orientationScore = orientationCurveScore(
      createOrientationCurve(
        options,
        durationSeconds,
        candidate.velocityControls,
        candidate.endPhase,
        endVelocity,
        candidate.matchPhase,
        candidate.swapTime,
      ),
    );
    if (!Number.isFinite(orientationScore)) continue;
    const score = candidate.score + orientationScore;
    if (!best || score < best.score) best = { ...candidate, score };
  }

  if (!best) {
    throw new RangeError('Unable to create a forward prism-spinner trajectory.');
  }

  return {
    curve: {
      durationSeconds,
      startPhase,
      velocityControls: best.velocityControls,
    },
    durationSeconds,
    endPhase: best.endPhase,
    endVelocity,
    handoff: best.handoff,
    matchPhase: best.matchPhase,
    orientation: createOrientationCurve(
      options,
      durationSeconds,
      best.velocityControls,
      best.endPhase,
      endVelocity,
      best.matchPhase,
      best.swapTime,
    ),
    swapTime: best.swapTime,
    startProgress,
    targetProgress,
  };
}

export function sampleFixedPrismSpinnerTrajectory(
  trajectory: FixedPrismSpinnerTrajectory,
  elapsedProgress: number,
): {
  angularVelocity: Vector3;
  orientation: Quaternion;
  phase: number;
  progress: number;
  velocity: number;
} {
  const time = clamp(elapsedProgress || 0);
  const duration = Math.max(trajectory.durationSeconds, EPSILON);
  const phase = trajectory.curve.startPhase + duration * integrateVelocityControls(trajectory.curve.velocityControls, time);
  const velocity = sampleVelocityControls(trajectory.curve.velocityControls, time);
  const progress = sampleTrajectoryProgress(trajectory, time);
  const orientation = sampleOrientation(trajectory.orientation, time);
  const angularVelocity = sampleOrientationAngularVelocity(trajectory.orientation, time);
  return {
    angularVelocity,
    orientation,
    phase,
    progress,
    velocity,
  };
}

function sampleCubicHermite(start: number, end: number, startRate: number, endRate: number, duration: number, progress: number): number {
  const amount = clamp(progress);
  const squared = amount * amount;
  const cubed = squared * amount;
  return (
    (2 * cubed - 3 * squared + 1) * start +
    (cubed - 2 * squared + amount) * startRate * duration +
    (-2 * cubed + 3 * squared) * end +
    (cubed - squared) * endRate * duration
  );
}

function sampleTrajectoryProgress(trajectory: FixedPrismSpinnerTrajectory, time: number): number {
  const amount = clamp(time);
  const swapTime = trajectory.swapTime;
  if (swapTime === null) {
    return trajectory.startProgress + (trajectory.targetProgress - trajectory.startProgress) * amount;
  }
  const waypoint = PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS;
  const beforeRate = (waypoint - trajectory.startProgress) / swapTime;
  const afterRate = (trajectory.targetProgress - waypoint) / (1 - swapTime);
  const sharedRate = (2 * beforeRate * afterRate) / (beforeRate + afterRate);
  if (amount <= swapTime) {
    return sampleCubicHermite(trajectory.startProgress, waypoint, beforeRate, sharedRate, swapTime, amount / swapTime);
  }
  return sampleCubicHermite(waypoint, trajectory.targetProgress, sharedRate, afterRate, 1 - swapTime, (amount - swapTime) / (1 - swapTime));
}

export function spinnerFrameIndexForPhase(phase: number, frameCount: number): number {
  const count = requireFrameCount(frameCount);
  return Math.min(count - 1, Math.floor(normalizeSpinnerPhase(phase) * count));
}

export function spinnerPhaseForFrameIndex(index: number, frameCount: number): number {
  const count = requireFrameCount(frameCount);
  const normalizedIndex = ((Math.trunc(index || 0) % count) + count) % count;
  return normalizedIndex / count;
}

export type PrismSpinnerMorphGeometry = {
  planeCount: number;
  planes: Float32Array;
  vertices: readonly Vector3[];
};

export function createPrismSpinnerMorphGeometry(progress: number): PrismSpinnerMorphGeometry {
  // The solid never interpolates. Geometry changes in the one
  // silhouette-matched frame; every other motion/material channel remains
  // continuous.
  const amount = clamp(progress || 0) >= PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS ? 1 : 0;
  const vertices = PRISM_VERTICES.map((start, index): Vector3 => {
    const end = OCTAHEDRON_VERTICES[index]!;
    return [start[0] + (end[0] - start[0]) * amount, start[1] + (end[1] - start[1]) * amount, start[2] + (end[2] - start[2]) * amount];
  });
  const centroid = vertices
    .reduce((sum, vertex) => [sum[0] + vertex[0], sum[1] + vertex[1], sum[2] + vertex[2]] as Vector3, [0, 0, 0] as Vector3)
    .map((value) => value / vertices.length) as unknown as Vector3;
  const candidatePlanes: number[][] = [];

  for (const triangle of MORPH_TRIANGLES) {
    const first = vertices[triangle[0]]!;
    const second = vertices[triangle[1]]!;
    const third = vertices[triangle[2]]!;
    let normal = normalize(cross(subtract(second, first), subtract(third, first)));
    if (dot(normal, subtract(first, centroid)) < 0) {
      normal = [-normal[0], -normal[1], -normal[2]];
    }
    const plane = [normal[0], normal[1], normal[2], dot(normal, first)];
    if (candidatePlanes.every((candidate) => candidate.some((value, index) => Math.abs(value - plane[index]!) >= 0.00001)))
      candidatePlanes.push(plane);
  }

  const planes = new Float32Array(candidatePlanes.length * 4);
  for (const [index, plane] of candidatePlanes.entries()) planes.set(plane, index * 4);
  return { planeCount: candidatePlanes.length, planes, vertices };
}

export type PrismSpinnerMorphChannels = {
  alignment: number;
  camera: number;
  crease: number;
  edge: number;
  geometry: number;
  geometryScale: number;
  handoff: number;
  material: number;
  sheen: number;
  size: number;
  stabilization: number;
};

export function resolvePrismSpinnerSizeProgress(progress: number): number {
  const normalized = clamp(progress || 0);
  const windowProgress = clamp(normalized / PRISM_SPINNER_SIZE_COMPLETION_PROGRESS);
  return 1 - (1 - windowProgress) ** 3;
}

export function resolvePrismSpinnerMorphChannels(progress: number, prismSize: number, spinnerSize: number): PrismSpinnerMorphChannels {
  const normalized = clamp(progress || 0);
  const startSize = Math.max(prismSize || 72, 1);
  const endSize = Math.max(spinnerSize || PRISM_SPINNER_CSS_SIZE, 1);
  const sizeProgress = resolvePrismSpinnerSizeProgress(normalized);
  const size =
    normalized <= 0
      ? startSize
      : normalized >= 1
        ? endSize
        : Math.exp(Math.log(startSize) * (1 - sizeProgress) + Math.log(endSize) * sizeProgress);
  const edge =
    normalized < PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS
      ? 0.72 * transitionWindow(normalized, 0.22, PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS)
      : 0.72 + 0.28 * transitionWindow(normalized, PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS, 0.94);
  const geometry = normalized >= PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS ? 1 : 0;
  const stabilization = transitionWindow(normalized, 0.72, 1);
  // The topology switch may change internal pose and scale once, but its
  // projected contour stays continuous. That is the only discrete state in
  // the otherwise smooth visible trajectory.
  const geometryScale =
    normalized < PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS
      ? 1
      : PRISM_SPINNER_SWAP_SCALE +
        (1 - PRISM_SPINNER_SWAP_SCALE) * transitionWindow(normalized, PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS, 0.96);
  const crease =
    normalized < PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS
      ? transitionWindow(normalized, 0.24, 0.46) * (1 - transitionWindow(normalized, 0.46, 0.64))
      : transitionWindow(normalized, 0.76, 0.98);
  return {
    alignment: transitionWindow(normalized, 0.08, PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS),
    camera: transitionWindow(normalized, 0.12, PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS),
    crease,
    edge,
    geometry,
    geometryScale,
    handoff: transitionWindow(normalized, 0.98, 1),
    material: transitionWindow(normalized, 0.18, PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS),
    sheen: transitionWindow(normalized, PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS, 0.92),
    size,
    stabilization,
  };
}

export function spinnerPhaseToWorldPhase(spinnerPhase: number, turn = 0): number {
  return Math.trunc(Number(turn) || 0) + normalizeSpinnerPhase(spinnerPhase) + SPINNER_ORIENTATION_TURNS;
}

export type SpinnerHandoff = {
  frameIndex: number;
  framePhase: number;
  worldPhase: number;
};

export function nextForwardSpinnerFrame(startWorldPhase: number, naturalTravel: number, frameCount: number): SpinnerHandoff {
  const count = requireFrameCount(frameCount);
  const start = startWorldPhase || 0;
  const desired = start + Math.max(naturalTravel || 0, 0);
  const firstTurn = Math.floor(start) - 1;
  const lastTurn = Math.max(firstTurn + 4, Math.ceil(desired) + 2);
  const candidates: SpinnerHandoff[] = [];

  for (let turn = firstTurn; turn <= lastTurn; turn += 1) {
    for (let frameIndex = 0; frameIndex < count; frameIndex += 1) {
      const framePhase = spinnerPhaseForFrameIndex(frameIndex, count);
      const worldPhase = spinnerPhaseToWorldPhase(framePhase, turn);
      if (worldPhase > start + EPSILON) candidates.push({ frameIndex, framePhase, worldPhase });
    }
  }
  if (candidates.length === 0) throw new RangeError('Unable to select a forward spinner frame.');
  return candidates.reduce((best, candidate) =>
    Math.abs(candidate.worldPhase - desired) < Math.abs(best.worldPhase - desired) ? candidate : best,
  );
}

export function nextForwardSpinnerSilhouettePhase(startWorldPhase: number, naturalTravel: number): number {
  const start = startWorldPhase || 0;
  const desired = start + Math.max(naturalTravel || 0, 0);
  const solveInputPhase = (targetRotationPhase: number) => {
    let lower = targetRotationPhase - 0.12;
    let upper = targetRotationPhase + 0.12;
    for (let iteration = 0; iteration < 48; iteration += 1) {
      const middle = (lower + upper) * 0.5;
      if (spinnerSourceWorldRotationPhase(middle) < targetRotationPhase) lower = middle;
      else upper = middle;
    }
    return (lower + upper) * 0.5;
  };
  const firstTarget = Math.floor(spinnerSourceWorldRotationPhase(start) * 2) / 2 + PRISM_SPINNER_SILHOUETTE_PHASE_TURNS;
  const candidateCount = Math.max(5, Math.ceil(Math.max(desired - start, 0) * 2) + 4);
  const candidates = Array.from({ length: candidateCount }, (_, index) => solveInputPhase(firstTarget + index * 0.5)).filter(
    (candidate) => candidate > start + EPSILON,
  );
  if (candidates.length === 0) throw new RangeError('Unable to select a forward spinner silhouette phase.');
  return candidates.reduce((best, candidate) => (Math.abs(candidate - desired) < Math.abs(best - desired) ? candidate : best));
}
