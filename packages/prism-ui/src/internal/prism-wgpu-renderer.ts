/// <reference types="@webgpu/types" />

export type PrismWebRenderer = {
  frameUniformByteLength: number;
  free(): void;
  render(
    frameUniformBytes: Uint8Array,
    opticalPaths: Float32Array,
    width: number,
    height: number,
    lightResolutionScale: number,
    materialResolutionScale: number,
    lightSourceSampleCount: number,
    materialSourceSampleCount: number,
    scissorX: number,
    scissorY: number,
    scissorWidth: number,
    scissorHeight: number,
    renderLight: boolean,
    hdrHeadroom: number,
  ): void;
  renderComputed(
    frameUniformBytes: Uint8Array,
    planes: Float32Array,
    planeCount: number,
    transitionScale: number,
    prismScale: number,
    bevel: number,
    phase: number,
    lightPhase: number,
    depthTransition: number,
    perspectiveTransition: number,
    lightCount: number,
    lightRadius: number,
    sourceSize: number,
    sourceDivergence: number,
    sourceSampleCount: number,
    ior: number,
    dispersion: number,
    width: number,
    height: number,
    lightResolutionScale: number,
    materialResolutionScale: number,
    lightSourceSampleCount: number,
    materialSourceSampleCount: number,
    scissorX: number,
    scissorY: number,
    scissorWidth: number,
    scissorHeight: number,
    renderLight: boolean,
    hdrHeadroom: number,
  ): void;
};

export type CreatePrismWebRenderer = (canvas: HTMLCanvasElement, preferHdr: boolean) => Promise<PrismWebRenderer>;

export const FRAME_UNIFORM_BYTES = 672;
const FLOAT_OFFSETS = {
  uResolution: 0,
  uPhase: 8,
  uLightPhase: 12,
  uIor: 24,
  uDispersion: 28,
  uTransmission: 32,
  uLightThroughput: 36,
  uMaterialOpacityScale: 40,
  uRoughness: 44,
  uFresnelStrength: 48,
  uSheenStrength: 52,
  uSheenWidth: 56,
  uSheenChroma: 60,
  uVisibility: 64,
  uBevel: 68,
  'uPrismPlanes[0]': 80,
  uShadowStrength: 404,
  uLightRadius: 408,
  uSourceSize: 412,
  uSourceDivergence: 416,
  uSourceHalo: 420,
  uRayleighMix: 428,
  uCausticHalo: 432,
  uTurbulenceStrength: 436,
  uTurbulenceSpeed: 440,
  uOpticalTime: 444,
  uIncidentStrength: 448,
  uScatteringStrength: 452,
  uScatteringFalloff: 456,
  uSpectralFanReach: 460,
  uBeamWidth: 464,
  uTransitionGeometry: 480,
  uTransitionAppearance: 496,
  uIconSize: 512,
  uViewportScale: 516,
  uPrismScale: 520,
  uTransitionPrismScale: 524,
  uSpinnerMorph: 528,
  uSpinnerMaterialMorph: 544,
  uSpinnerMorphScale: 552,
  uSpinnerFrameSize: 556,
  uCssPixelRatio: 560,
  uSpinnerMorphMatchPhase: 564,
  uObjectPoseQuaternion: 576,
  uObjectPoseOverride: 592,
  uSpinnerCreaseMorph: 596,
  uMaterialSpectralNormalization: 608,
  uDarkMode: 620,
  uEnvironmentLuminance: 624,
  uEnvironmentLightMix: 628,
  uRenderPrismScale: 632,
  uScaledScatteringFalloff: 636,
  uIconEdgeColor: 640,
  uObjectProjection: 656,
};

const INTEGER_OFFSETS = {
  uLightCount: 16,
  uRenderLayer: 20,
  uPrismPlaneCount: 400,
  uSourceSampleCount: 424,
  uMaterialSpectralSampleCount: 600,
};

type IntegerUniformName = keyof typeof INTEGER_OFFSETS;
type FloatUniformName = keyof typeof FLOAT_OFFSETS;

type OpticalFrame = {
  bevel: number;
  depthTransition: number;
  dispersion: number;
  enabled: boolean;
  ior: number;
  lightCount: number;
  lightPhase: number;
  lightRadius: number;
  perspectiveTransition: number;
  phase: number;
  planeCount: number;
  planes: Float32Array;
  prismScale: number;
  sourceDivergence: number;
  sourceSampleCount: number;
  sourceSize: number;
  transitionScale: number;
};

type ScissorRectangle = { x: number; y: number; width: number; height: number };
type HdrMode = 'auto' | 'off' | 'on';

export function createPrismFrameUniformWriter() {
  const buffer = new ArrayBuffer(FRAME_UNIFORM_BYTES);
  const bytes = new Uint8Array(buffer);
  const view = new DataView(buffer);

  function floatOffset(name: FloatUniformName) {
    return FLOAT_OFFSETS[name];
  }

  function writeFloats(name: FloatUniformName, values: ArrayLike<number>) {
    const offset = floatOffset(name);
    let index = 0;
    while (index < values.length) {
      view.setFloat32(offset + index * 4, Number(values[index]) || 0, true);
      index += 1;
    }
  }

  return {
    bytes,
    setFloat(name: FloatUniformName, value: number) {
      view.setFloat32(floatOffset(name), value || 0, true);
    },
    setInteger(name: IntegerUniformName, value: number) {
      view.setInt32(INTEGER_OFFSETS[name], value || 0, true);
    },
    setVector2(name: FloatUniformName, x: number, y: number) {
      const offset = floatOffset(name);
      view.setFloat32(offset, x || 0, true);
      view.setFloat32(offset + 4, y || 0, true);
    },
    setVector3(name: FloatUniformName, x: number, y: number, z: number) {
      const offset = floatOffset(name);
      view.setFloat32(offset, x || 0, true);
      view.setFloat32(offset + 4, y || 0, true);
      view.setFloat32(offset + 8, z || 0, true);
    },
    setVector4(name: FloatUniformName, x: number, y: number, z: number, w: number) {
      const offset = floatOffset(name);
      view.setFloat32(offset, x || 0, true);
      view.setFloat32(offset + 4, y || 0, true);
      view.setFloat32(offset + 8, z || 0, true);
      view.setFloat32(offset + 12, w || 0, true);
    },
    setVector4Array(name: FloatUniformName, values: ArrayLike<number>) {
      writeFloats(name, values);
    },
    readInteger(name: IntegerUniformName) {
      return view.getInt32(INTEGER_OFFSETS[name], true);
    },
  };
}

export function createDeferredPrismWgpuSurface(canvas: HTMLCanvasElement, createRenderer: CreatePrismWebRenderer) {
  const frame = createPrismFrameUniformWriter();
  const dynamicRange = globalThis.matchMedia?.('(dynamic-range: high)') ?? null;
  let renderer: PrismWebRenderer | null = null;
  let disposed = false;
  let readiness: 'loading' | 'ready' | 'unavailable' = 'loading';
  let hdrMode: HdrMode = 'auto';
  let hdrHeadroom = 1.25;
  let lightSize = { height: 1, width: 1 };
  let materialSize = { height: 1, width: 1 };
  let lightSourceSampleCount = 5;
  let materialSourceSampleCount = 5;
  let renderLight = true;
  let materialScissor: ScissorRectangle | null = null;
  let opticalFrame: OpticalFrame | null = null;
  const emptyOpticalPaths = new Float32Array(1764);
  const readyCallbacks: ((error?: unknown) => void)[] = [];
  const unavailableCallbacks = new Set<(error: unknown) => void>();
  const { promise: ready, resolve: resolveReady } = Promise.withResolvers<'ready' | 'unavailable'>();
  let unavailableError: unknown;

  function settleReadiness(state: 'ready' | 'unavailable', error?: unknown) {
    if (readiness !== 'loading') return;
    readiness = state;
    resolveReady(state);
    const callbacks = [...readyCallbacks];
    readyCallbacks.length = 0;
    for (const callback of callbacks) callback(error);
  }

  function markUnavailable(error: unknown) {
    if (readiness === 'unavailable') return;
    unavailableError = error;
    renderer?.free();
    renderer = null;
    if (readiness === 'loading') settleReadiness('unavailable', error);
    else readiness = 'unavailable';
    for (const callback of unavailableCallbacks) callback(error);
    unavailableCallbacks.clear();
  }

  void (async () => {
    try {
      const nextRenderer = await createRenderer(canvas, dynamicRange?.matches === true);
      if (disposed) {
        nextRenderer.free();
        return;
      }
      if (nextRenderer.frameUniformByteLength !== FRAME_UNIFORM_BYTES) {
        const actualLength = nextRenderer.frameUniformByteLength;
        nextRenderer.free();
        throw new RangeError(`Prism frame-uniform ABI mismatch: expected ${FRAME_UNIFORM_BYTES} bytes, received ${actualLength}.`);
      }
      renderer = nextRenderer;
      settleReadiness('ready');
    } catch (err: unknown) {
      if (disposed) return;
      console.warn('[Typie Prism] WebGPU renderer initialization failed.', err);
      markUnavailable(err);
    }
  })();

  function effectiveHeadroom() {
    const enabled = hdrMode === 'on' || (hdrMode === 'auto' && dynamicRange?.matches);
    return enabled ? hdrHeadroom : 1;
  }

  return {
    frameUniforms: frame,
    get readiness() {
      return readiness;
    },
    whenReady() {
      return ready;
    },
    onReady(callback: (error?: unknown) => void) {
      if (readiness === 'ready') callback();
      else if (readiness === 'unavailable') callback(unavailableError);
      else readyCallbacks.push(callback);
    },
    onUnavailable(callback: (error: unknown) => void) {
      if (readiness === 'unavailable') callback(unavailableError);
      else unavailableCallbacks.add(callback);
      return () => unavailableCallbacks.delete(callback);
    },
    setHdrMode(mode: HdrMode) {
      hdrMode = mode;
    },
    setHdrHeadroom(value: number) {
      hdrHeadroom = Math.max(1, Math.min(value || 1, 2.5));
    },
    setOpticalFrame(value: OpticalFrame) {
      opticalFrame = value;
    },
    resizeLightTarget(width: number, height: number) {
      lightSize = { height, width };
    },
    resizeMaterialTarget(width: number, height: number) {
      materialSize = { height, width };
    },
    drawLight(sourceSampleCount?: number) {
      renderLight = true;
      lightSourceSampleCount = sourceSampleCount ?? frame.readInteger('uSourceSampleCount');
    },
    clearLight() {
      renderLight = false;
    },
    drawPrism(scissor: ScissorRectangle | null) {
      materialScissor = scissor;
      materialSourceSampleCount = frame.readInteger('uSourceSampleCount');
    },
    compositeSdr() {
      if (!renderer || !opticalFrame || disposed) return;
      const scissor: ScissorRectangle = materialScissor ?? { x: 0, y: 0, width: 0, height: 0 };
      const lightScale = Math.min(lightSize.width / Math.max(canvas.width, 1), lightSize.height / Math.max(canvas.height, 1));
      const materialScale = Math.min(materialSize.width / Math.max(canvas.width, 1), materialSize.height / Math.max(canvas.height, 1));
      const headroom = effectiveHeadroom();
      try {
        if (opticalFrame.enabled) {
          renderer.renderComputed(
            frame.bytes,
            opticalFrame.planes,
            opticalFrame.planeCount,
            opticalFrame.transitionScale,
            opticalFrame.prismScale,
            opticalFrame.bevel,
            opticalFrame.phase,
            opticalFrame.lightPhase,
            opticalFrame.depthTransition,
            opticalFrame.perspectiveTransition,
            opticalFrame.lightCount,
            opticalFrame.lightRadius,
            opticalFrame.sourceSize,
            opticalFrame.sourceDivergence,
            opticalFrame.sourceSampleCount,
            opticalFrame.ior,
            opticalFrame.dispersion,
            canvas.width,
            canvas.height,
            lightScale,
            materialScale,
            lightSourceSampleCount,
            materialSourceSampleCount,
            scissor.x,
            scissor.y,
            scissor.width,
            scissor.height,
            renderLight,
            headroom,
          );
        } else {
          renderer.render(
            frame.bytes,
            emptyOpticalPaths,
            canvas.width,
            canvas.height,
            lightScale,
            materialScale,
            lightSourceSampleCount,
            materialSourceSampleCount,
            scissor.x,
            scissor.y,
            scissor.width,
            scissor.height,
            renderLight,
            headroom,
          );
        }
      } catch (err) {
        console.warn('[Typie Prism] WebGPU frame failed.', err);
        markUnavailable(err);
      }
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      renderer?.free();
      renderer = null;
      unavailableCallbacks.clear();
      if (readiness === 'loading') {
        settleReadiness('unavailable', new Error('WebGPU renderer was disposed before it became ready.'));
      } else {
        readiness = 'unavailable';
      }
    },
  };
}
