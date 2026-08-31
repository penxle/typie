import { afterEach, describe, expect, it, vi } from 'vitest';
import { createPrismAudioPlayer } from './prism-audio';

class AudioContextStub {
  static latest: AudioContextStub;
  state: AudioContextState = 'suspended';
  destination = {} as AudioDestinationNode;
  resume = vi.fn(async () => {
    this.state = 'running';
  });
  decodeAudioData = vi.fn(async () => ({}) as AudioBuffer);
  sources: {
    addEventListener: ReturnType<typeof vi.fn>;
    buffer: AudioBuffer | null;
    connect: ReturnType<typeof vi.fn>;
    disconnect: ReturnType<typeof vi.fn>;
    ended: (() => void) | null;
    removeEventListener: ReturnType<typeof vi.fn>;
    start: ReturnType<typeof vi.fn>;
    stop: ReturnType<typeof vi.fn>;
  }[] = [];
  createBufferSource = vi.fn(() => {
    const source = {
      addEventListener: vi.fn((_type: string, listener: () => void) => {
        source.ended = listener;
      }),
      buffer: null,
      connect: vi.fn(),
      disconnect: vi.fn(),
      ended: null as (() => void) | null,
      removeEventListener: vi.fn((_type: string, listener: () => void) => {
        if (source.ended === listener) source.ended = null;
      }),
      start: vi.fn(),
      stop: vi.fn(),
    };
    this.sources.push(source);
    return source;
  }) as unknown as AudioContext['createBufferSource'];
  close = vi.fn(async () => {
    this.state = 'closed';
  });

  constructor() {
    AudioContextStub.latest = this;
  }
}

describe('createPrismAudioPlayer', () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
    Reflect.deleteProperty(window, 'AudioContext');
  });

  it('retries sound loading after a transient failure', async () => {
    Object.defineProperty(window, 'AudioContext', { configurable: true, value: AudioContextStub });
    const fetchMock = vi.fn(async () => {
      if (fetchMock.mock.calls.length === 1) return { ok: false, status: 503 } as Response;
      return { ok: true, arrayBuffer: async () => new ArrayBuffer(1) } as Response;
    });
    vi.stubGlobal('fetch', fetchMock);

    const player = createPrismAudioPlayer();
    document.dispatchEvent(new Event('pointerdown'));
    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));

    document.dispatchEvent(new Event('pointerdown'));
    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(4));

    expect(player.play('resolved')).toBe(true);
    player.destroy();
  });

  it('stops the previous sound before playing another one', async () => {
    Object.defineProperty(window, 'AudioContext', { configurable: true, value: AudioContextStub });
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({ ok: true, arrayBuffer: async () => new ArrayBuffer(1) }) as Response),
    );

    const firstEnded = vi.fn();
    const secondEnded = vi.fn();
    const player = createPrismAudioPlayer();
    const context = AudioContextStub.latest;
    document.dispatchEvent(new Event('pointerdown'));
    await vi.waitFor(() => expect(context.decodeAudioData).toHaveBeenCalledTimes(2));

    expect(player.play('resolved', firstEnded)).toBe(true);
    expect(player.play('action-required', secondEnded)).toBe(true);

    expect(context.sources[0]?.stop).toHaveBeenCalledOnce();
    context.sources[0]?.ended?.();
    expect(firstEnded).not.toHaveBeenCalled();

    context.sources[1]?.ended?.();
    expect(secondEnded).toHaveBeenCalledOnce();

    player.destroy();
  });

  it('loads and plays a preview on its first click', async () => {
    Object.defineProperty(window, 'AudioContext', { configurable: true, value: AudioContextStub });
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({ ok: true, arrayBuffer: async () => new ArrayBuffer(1) }) as Response),
    );

    const player = createPrismAudioPlayer();
    const context = AudioContextStub.latest;

    await expect(player.preview('resolved')).resolves.toBe(true);
    expect(context.resume).toHaveBeenCalledOnce();
    expect(context.sources).toHaveLength(1);

    player.destroy();
  });

  it('does not start a preview that was stopped while loading', async () => {
    Object.defineProperty(window, 'AudioContext', { configurable: true, value: AudioContextStub });
    let release!: () => void;
    const loading = new Promise<void>((resolve) => (release = resolve));
    vi.stubGlobal(
      'fetch',
      vi.fn(
        async () =>
          ({
            ok: true,
            arrayBuffer: async () => {
              await loading;
              return new ArrayBuffer(1);
            },
          }) as Response,
      ),
    );

    const player = createPrismAudioPlayer();
    const preview = player.preview('resolved');
    player.stop();
    release();

    await expect(preview).resolves.toBe(false);
    expect(AudioContextStub.latest.sources).toHaveLength(0);
    player.destroy();
  });
});
