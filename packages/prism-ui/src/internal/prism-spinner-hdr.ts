/// <reference types="@webgpu/types" />

/* eslint-disable unicorn/consistent-class-member-order -- Public playback lifecycle methods stay grouped; private GPU helpers remain beside their call sites. */

export type PrismSpinnerHdrMode = 'auto' | 'off' | 'on';
export type PrismSpinnerHdrState = 'active' | 'failed' | 'initializing' | 'off' | 'unsupported';

export const PRISM_SPINNER_HDR_HEADROOM_DEFAULT = 1.15;

function shouldAttemptPrismSpinnerHdr(mode: PrismSpinnerHdrMode, dynamicRangeHigh: boolean): boolean {
  return mode === 'on' || (mode === 'auto' && dynamicRangeHigh);
}

const SHADER = `
struct VertexOutput {
  @builtin(position) position: vec4f,
  @location(0) color: vec4f,
};

@vertex
fn vertexMain(
  @location(0) position: vec2f,
  @location(1) color: vec4f,
) -> VertexOutput {
  var output: VertexOutput;
  output.position = vec4f(position, 0.0, 1.0);
  output.color = color;
  return output;
}

@fragment
fn fragmentMain(input: VertexOutput) -> @location(0) vec4f {
  return input.color;
}
`;

type SharedResources = {
  device: GPUDevice;
  pipeline: GPURenderPipeline;
};

let sharedResourcesPromise: Promise<SharedResources> | null = null;

function requestSharedResources(): Promise<SharedResources> {
  if (sharedResourcesPromise) return sharedResourcesPromise;
  const promise = (async () => {
    const gpu = globalThis.navigator?.gpu;
    if (!gpu) throw new Error('WebGPU is unavailable.');
    const adapter = await gpu.requestAdapter();
    const device = await adapter?.requestDevice();
    if (!device) throw new Error('A WebGPU device is unavailable.');
    const shader = device.createShaderModule({ code: SHADER });
    const pipeline = device.createRenderPipeline({
      fragment: {
        entryPoint: 'fragmentMain',
        module: shader,
        targets: [
          {
            blend: {
              alpha: { dstFactor: 'one-minus-src-alpha', operation: 'add', srcFactor: 'one' },
              color: { dstFactor: 'one-minus-src-alpha', operation: 'add', srcFactor: 'one' },
            },
            format: 'rgba16float',
          },
        ],
      },
      layout: 'auto',
      primitive: { topology: 'triangle-list' },
      vertex: {
        buffers: [
          {
            arrayStride: 24,
            attributes: [
              { format: 'float32x2', offset: 0, shaderLocation: 0 },
              { format: 'float32x4', offset: 8, shaderLocation: 1 },
            ],
          },
        ],
        entryPoint: 'vertexMain',
        module: shader,
      },
    });
    return { device, pipeline };
  })();
  sharedResourcesPromise = promise;
  promise
    .then(({ device }) =>
      device.lost.then(() => {
        if (sharedResourcesPromise === promise) sharedResourcesPromise = null;
      }),
    )
    .catch(() => {
      if (sharedResourcesPromise === promise) sharedResourcesPromise = null;
    });
  return promise;
}

export type PrismSpinnerHdrRenderJob = {
  device: GPUDevice;
  encode: (encoder: GPUCommandEncoder) => void;
};

export function submitPrismSpinnerHdrRenderJobs(jobs: readonly PrismSpinnerHdrRenderJob[]): void {
  const jobsByDevice = new Map<GPUDevice, PrismSpinnerHdrRenderJob[]>();
  for (const job of jobs) {
    const deviceJobs = jobsByDevice.get(job.device);
    if (deviceJobs) deviceJobs.push(job);
    else jobsByDevice.set(job.device, [job]);
  }
  for (const [device, deviceJobs] of jobsByDevice) {
    const encoder = device.createCommandEncoder();
    for (const job of deviceJobs) job.encode(encoder);
    device.queue.submit([encoder.finish()]);
  }
}

const queuedSpinnerHdrOverlays = new Set<PrismSpinnerHdrOverlay>();

export function flushPrismSpinnerHdrRenderBatch(): void {
  const jobs = [...queuedSpinnerHdrOverlays]
    .map((overlay) => overlay.takeQueuedRenderJob())
    .filter((job): job is PrismSpinnerHdrRenderJob => Boolean(job));
  queuedSpinnerHdrOverlays.clear();
  submitPrismSpinnerHdrRenderJobs(jobs);
}

export class PrismSpinnerHdrOverlay {
  private activeMode: PrismSpinnerHdrMode = 'off';
  private buffer: GPUBuffer | null = null;
  private bufferCapacity = 0;
  private readonly canvas: HTMLCanvasElement;
  private connected = false;
  private context: GPUCanvasContext | null = null;
  private device: GPUDevice | null = null;
  private generation = 0;
  private mediaQuery: MediaQueryList | null = null;
  private readonly onStateChange: (state: PrismSpinnerHdrState) => void;
  private pendingFrame: { cssSize: number; vertices: Float32Array } | null = null;
  private pipeline: GPURenderPipeline | null = null;
  private queuedRenderJob: PrismSpinnerHdrRenderJob | null = null;

  constructor(canvas: HTMLCanvasElement, onStateChange: (state: PrismSpinnerHdrState) => void) {
    this.canvas = canvas;
    this.onStateChange = onStateChange;
    this.canvas.hidden = true;
  }

  connect(mode: PrismSpinnerHdrMode): void {
    if (!this.connected) {
      this.connected = true;
      this.mediaQuery = globalThis.matchMedia?.('(dynamic-range: high)') ?? null;
      this.mediaQuery?.addEventListener?.('change', this.handleDynamicRangeChange);
    }
    this.setMode(mode);
  }

  disconnect(): void {
    this.connected = false;
    this.generation += 1;
    this.mediaQuery?.removeEventListener?.('change', this.handleDynamicRangeChange);
    this.mediaQuery = null;
    this.deactivate('off');
  }

  setMode(mode: PrismSpinnerHdrMode): void {
    if (this.activeMode === mode && (mode === 'off' || this.device)) return;
    this.activeMode = mode;
    this.generation += 1;
    this.deactivate(mode === 'off' ? 'off' : 'unsupported');
    if (mode !== 'off') void this.initialize(this.generation);
  }

  renderVertices(cssSize: number, vertices: Float32Array): void {
    queuedSpinnerHdrOverlays.delete(this);
    this.queuedRenderJob = null;
    this.pendingFrame = { cssSize, vertices };
    const job = this.prepareRenderJob(cssSize, vertices);
    if (job) submitPrismSpinnerHdrRenderJobs([job]);
  }

  queueRenderVertices(cssSize: number, vertices: Float32Array): void {
    this.pendingFrame = { cssSize, vertices };
    this.queuedRenderJob = this.prepareRenderJob(cssSize, vertices);
    if (!this.queuedRenderJob) return;
    queuedSpinnerHdrOverlays.add(this);
  }

  takeQueuedRenderJob(): PrismSpinnerHdrRenderJob | null {
    const job = this.queuedRenderJob;
    this.queuedRenderJob = null;
    return job;
  }

  private prepareRenderJob(cssSize: number, vertices: Float32Array): PrismSpinnerHdrRenderJob | null {
    if (!this.device || !this.context || !this.pipeline) return null;

    const dpr = Math.max(1, globalThis.devicePixelRatio || 1);
    const pixelSize = Math.max(1, Math.round(cssSize * dpr));
    if (this.canvas.width !== pixelSize || this.canvas.height !== pixelSize) {
      this.canvas.width = pixelSize;
      this.canvas.height = pixelSize;
    }

    const byteLength = Math.max(24, vertices.byteLength);
    if (!this.buffer || this.bufferCapacity < byteLength) {
      this.buffer?.destroy();
      this.bufferCapacity = 2 ** Math.ceil(Math.log2(byteLength));
      this.buffer = this.device.createBuffer({
        size: this.bufferCapacity,
        usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.VERTEX,
      });
    }
    if (vertices.byteLength > 0) this.device.queue.writeBuffer(this.buffer, 0, vertices);

    const device = this.device;
    const context = this.context;
    const pipeline = this.pipeline;
    const buffer = this.buffer;
    const vertexCount = vertices.length / 6;
    return {
      device,
      encode(encoder) {
        const pass = encoder.beginRenderPass({
          colorAttachments: [
            {
              clearValue: { r: 0, g: 0, b: 0, a: 0 },
              loadOp: 'clear',
              storeOp: 'store',
              view: context.getCurrentTexture().createView(),
            },
          ],
        });
        pass.setPipeline(pipeline);
        if (vertexCount > 0) {
          pass.setVertexBuffer(0, buffer);
          pass.draw(vertexCount);
        }
        pass.end();
      },
    };
  }

  private readonly handleDynamicRangeChange = (): void => {
    if (this.activeMode !== 'auto') return;
    this.generation += 1;
    this.deactivate('unsupported');
    void this.initialize(this.generation);
  };

  private async initialize(generation: number): Promise<void> {
    const dynamicRangeHigh = this.mediaQuery?.matches ?? false;
    if (!shouldAttemptPrismSpinnerHdr(this.activeMode, dynamicRangeHigh)) {
      this.onStateChange('unsupported');
      return;
    }
    if (!globalThis.navigator?.gpu) {
      this.onStateChange('unsupported');
      return;
    }

    this.onStateChange('initializing');
    try {
      const { device, pipeline } = await requestSharedResources();
      const context = this.canvas.getContext('webgpu');
      if (!device || !context || generation !== this.generation || !this.connected) return;

      context.configure({
        alphaMode: 'premultiplied',
        colorSpace: 'display-p3',
        device,
        format: 'rgba16float',
        toneMapping: { mode: 'extended' },
      });
      const configuration = context.getConfiguration?.();
      if (this.activeMode === 'auto' && configuration?.toneMapping?.mode !== 'extended') {
        context.unconfigure?.();
        this.onStateChange('unsupported');
        return;
      }

      if (generation !== this.generation || !this.connected) return;

      this.context = context;
      this.device = device;
      this.pipeline = pipeline;
      this.canvas.hidden = false;
      this.onStateChange('active');
      device.lost.then(() => {
        if (this.device !== device) return;
        this.generation += 1;
        this.deactivate('failed');
      });
      if (this.pendingFrame) this.renderVertices(this.pendingFrame.cssSize, this.pendingFrame.vertices);
    } catch {
      if (generation === this.generation) this.deactivate('failed');
    }
  }

  private deactivate(state: PrismSpinnerHdrState): void {
    queuedSpinnerHdrOverlays.delete(this);
    this.queuedRenderJob = null;
    this.canvas.hidden = true;
    this.context?.unconfigure?.();
    this.buffer?.destroy();
    this.buffer = null;
    this.bufferCapacity = 0;
    this.context = null;
    this.device = null;
    this.pipeline = null;
    this.onStateChange(state);
  }
}
