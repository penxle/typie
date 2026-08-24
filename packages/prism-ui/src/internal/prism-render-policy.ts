function clamp(value: number, minimum: number, maximum: number) {
  return Math.max(minimum, Math.min(maximum, value));
}

const MATERIAL_SPECTRAL_NORMALIZATIONS = {
  3: [3.6, 3.5, 0.7875],
  5: [3.7285714285714286, 4, 1.425],
  7: [3.6285714285714286, 3.7846153846153845, 1.95],
} as const;

export function resolvePrismRenderTargets(width: number, height: number, lightResolutionScale: number, materialResolutionScale = 1) {
  const outputWidth = Math.max(1, Math.round(width || 0));
  const outputHeight = Math.max(1, Math.round(height || 0));
  const materialScale = clamp(Number(materialResolutionScale) || 1, 2 / 3, 1);
  const material = {
    width: Math.max(1, Math.round(outputWidth * materialScale)),
    height: Math.max(1, Math.round(outputHeight * materialScale)),
  };
  const lightScale = clamp(lightResolutionScale || 1, 0.25, 1);
  return {
    material,
    light: {
      width: Math.max(1, Math.round(outputWidth * lightScale)),
      height: Math.max(1, Math.round(outputHeight * lightScale)),
    },
  };
}

export function resolvePrismRenderWork(spinnerGeometry: number, iconSample: { optics: number } | null = null) {
  if (iconSample) {
    const opticsVisible = Number(iconSample.optics) > 0;
    return {
      light: opticsVisible,
      opticalPaths: opticsVisible,
    };
  }
  const physicalPrismVisible = spinnerGeometry < 0.5;
  return {
    light: physicalPrismVisible,
    opticalPaths: physicalPrismVisible,
  };
}

export function createAdaptiveLightResolutionPolicy(initialRequestedScale: number) {
  let requestedScale = clamp(initialRequestedScale || 1, 0.25, 1);
  let qualityLevels = qualityLevelsFor(requestedScale);
  let qualityIndex = 0;
  let slowFrames = 0;
  let stableFrames = 0;
  let cooldownFrames = 0;

  function qualityLevelsFor(requested: number) {
    const resolutionScales = [...new Set([requested, Math.max(0.25, requested * 0.75), 0.25])].toSorted((left, right) => right - left);
    const minimumScale = resolutionScales.at(-1) ?? 0.25;
    return [
      ...resolutionScales.map((scale) => ({ scale, sourceSampleCap: 5 })),
      { scale: minimumScale, sourceSampleCap: 3 },
      { materialScale: 0.82, materialSpectralSampleCount: 5, scale: minimumScale, sourceSampleCap: 3 },
      { materialScale: 2 / 3, materialSpectralSampleCount: 3, scale: minimumScale, sourceSampleCap: 3 },
    ].map((level) => ({
      materialScale: 1,
      materialSpectralSampleCount: 7,
      ...level,
    }));
  }

  function reset(nextRequestedScale: number) {
    requestedScale = clamp(nextRequestedScale || 1, 0.25, 1);
    qualityLevels = qualityLevelsFor(requestedScale);
    qualityIndex = 0;
    slowFrames = 0;
    stableFrames = 0;
    cooldownFrames = 0;
  }

  function currentQualityLevel() {
    const level = qualityLevels[qualityIndex];
    if (!level) throw new RangeError('Prism render quality index is out of bounds.');
    return level;
  }

  return {
    sample(frameIntervalMs: number, targetFrameIntervalMs: number, nextRequestedScale = requestedScale, sampleWeight = 1) {
      const nextRequested = clamp(Number(nextRequestedScale) || 1, 0.25, 1);
      if (Math.abs(nextRequested - requestedScale) > 0.0001) reset(nextRequested);

      const target = Math.max(targetFrameIntervalMs || 0, 1);
      const interval = Math.max(frameIntervalMs || target, 0);
      const pressure = interval / target;
      const weight = clamp(Math.round(Number(sampleWeight) || 1), 1, 60);
      if (cooldownFrames > 0) cooldownFrames = Math.max(0, cooldownFrames - weight);

      if (pressure > 1.18) {
        slowFrames += 1;
        stableFrames = 0;
      } else {
        slowFrames = Math.max(0, slowFrames - 1);
        stableFrames = pressure <= 1.08 ? stableFrames + weight : 0;
      }

      if (cooldownFrames === 0 && slowFrames >= 6 && qualityIndex < qualityLevels.length - 1) {
        qualityIndex += 1;
        slowFrames = 0;
        stableFrames = 0;
        cooldownFrames = 30;
      } else if (cooldownFrames === 0 && stableFrames >= 180 && qualityIndex > 0) {
        qualityIndex -= 1;
        slowFrames = 0;
        stableFrames = 0;
        cooldownFrames = 180;
      }

      return currentQualityLevel().scale;
    },
    reset,
    get scale() {
      return currentQualityLevel().scale;
    },
    get qualityLevel() {
      return qualityIndex + 1;
    },
    get qualityLevelCount() {
      return qualityLevels.length;
    },
    get sourceSampleCap() {
      return currentQualityLevel().sourceSampleCap;
    },
    get materialScale() {
      return currentQualityLevel().materialScale;
    },
    get materialSpectralSampleCount() {
      return currentQualityLevel().materialSpectralSampleCount;
    },
    get materialSpectralNormalization() {
      const sampleCount = currentQualityLevel().materialSpectralSampleCount as 3 | 5 | 7;
      return MATERIAL_SPECTRAL_NORMALIZATIONS[sampleCount];
    },
  };
}
