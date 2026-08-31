import { afterEach, describe, expect, it, vi } from 'vitest';
import { createPrismAudioPlayer } from './prism-audio';

class AudioContextStub {
  state: AudioContextState = 'suspended';
  destination = {} as AudioDestinationNode;

  resume = vi.fn(async () => {
    this.state = 'running';
  });

  decodeAudioData = vi.fn(async () => ({}) as AudioBuffer);
  createBufferSource = vi.fn(() => ({
    buffer: null,
    connect: vi.fn(),
    start: vi.fn(),
  })) as unknown as AudioContext['createBufferSource'];
  close = vi.fn(async () => {
    this.state = 'closed';
  });
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
});
