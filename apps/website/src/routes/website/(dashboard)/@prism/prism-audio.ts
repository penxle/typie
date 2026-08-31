import type { PrismNotificationSound } from './prism-notifications';

const SOUND_URLS: Record<PrismNotificationSound, string> = {
  resolved: '/sounds/prism/resolved.wav',
  'action-required': '/sounds/prism/action-required.wav',
};

export const createPrismAudioPlayer = () => {
  const AudioContextConstructor = window.AudioContext;
  const context = AudioContextConstructor ? new AudioContextConstructor() : null;
  const buffers = new Map<PrismNotificationSound, AudioBuffer>();
  let loading: Promise<unknown> | null = null;

  const load = () => {
    if (context === null) return Promise.resolve();
    if (loading !== null) return loading;

    const request = Promise.all(
      Object.entries(SOUND_URLS).map(async ([kind, url]) => {
        const response = await fetch(url);
        if (!response.ok) throw new Error(`failed to load prism sound: ${response.status}`);
        buffers.set(kind as PrismNotificationSound, await context.decodeAudioData(await response.arrayBuffer()));
      }),
    );
    loading = request;
    void request.catch(() => {
      if (loading === request) loading = null;
    });
    return loading;
  };

  const unlock = () => {
    if (context === null) return;
    void context
      .resume()
      .then(load)
      .catch(() => null);
  };

  document.addEventListener('pointerdown', unlock, { capture: true, passive: true });
  document.addEventListener('keydown', unlock, { capture: true });

  const canPlay = (kind: PrismNotificationSound) => context?.state === 'running' && buffers.has(kind);

  return {
    canPlay,
    play: (kind: PrismNotificationSound) => {
      if (context === null || !canPlay(kind)) return false;
      const buffer = buffers.get(kind);
      if (buffer === undefined) return false;

      const source = context.createBufferSource();
      source.buffer = buffer;
      source.connect(context.destination);
      source.start();
      return true;
    },
    destroy: () => {
      document.removeEventListener('pointerdown', unlock, { capture: true });
      document.removeEventListener('keydown', unlock, { capture: true });
      void context?.close();
    },
  };
};
