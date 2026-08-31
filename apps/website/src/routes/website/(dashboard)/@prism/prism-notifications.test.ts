import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPrismNotifications } from './prism-notifications';

const createStorage = (): Storage => {
  const entries = new Map<string, string>();
  return {
    get length() {
      return entries.size;
    },
    clear: () => entries.clear(),
    getItem: (key) => entries.get(key) ?? null,
    key: (index) => [...entries.keys()][index] ?? null,
    removeItem: (key) => entries.delete(key),
    setItem: (key, value) => entries.set(key, value),
  };
};

Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: createStorage() });

const notification = {
  id: 'notification-1',
  sessionId: 'session-1',
  kind: 'TURN_RESOLVED' as const,
  elapsedMs: 10_000,
};

const notifications = (
  canPlay: () => boolean,
  play: (sound: 'resolved' | 'action-required') => boolean,
  viewingSessionId: string | null = null,
) => {
  const options = {
    userId: 'user-1',
    enabled: true,
    viewingSessionId,
    canPlay,
    play,
  };
  return createPrismNotifications(options);
};

const setPageActive = (active: boolean) => {
  Object.defineProperty(document, 'visibilityState', { configurable: true, value: active ? 'visible' : 'hidden' });
  vi.spyOn(document, 'hasFocus').mockReturnValue(active);
};

describe('createPrismNotifications', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    localStorage.clear();
    setPageActive(true);

    let lock: Promise<unknown> = Promise.resolve();
    Object.defineProperty(navigator, 'locks', {
      configurable: true,
      value: {
        request: async (_name: string, callback: LockGrantedCallback<unknown>) => {
          const result = lock.then(() => callback({} as Lock));
          lock = result.catch(() => null);
          return await result;
        },
      },
    });
  });

  afterEach(() => {
    vi.useRealTimers();
    Reflect.deleteProperty(navigator, 'locks');
  });

  it('suppresses a short response while the focused tab views its session', async () => {
    const play = vi.fn(() => true);
    notifications(() => true, play, 'session-1').handle(notification);

    await vi.runAllTimersAsync();

    expect(play).not.toHaveBeenCalled();
  });

  it('plays a long response even while the focused tab views its session', async () => {
    const play = vi.fn(() => true);
    notifications(() => true, play, 'session-1').handle({ ...notification, elapsedMs: 30_000 });

    await vi.runAllTimersAsync();

    expect(play).toHaveBeenCalledWith('resolved');
  });

  it('plays in a hidden tab even when that session is selected', async () => {
    setPageActive(false);
    const play = vi.fn(() => true);
    notifications(() => true, play, 'session-1').handle({ ...notification, kind: 'USER_ACTION_REQUIRED' });

    await vi.runAllTimersAsync();

    expect(play).toHaveBeenCalledWith('action-required');
  });

  it('lets a ready tab play when another tab cannot play audio', async () => {
    const unavailablePlay = vi.fn(() => false);
    const readyPlay = vi.fn(() => true);
    const unavailable = notifications(() => false, unavailablePlay);
    const ready = notifications(() => true, readyPlay);
    unavailable.handle(notification);
    ready.handle(notification);

    await vi.advanceTimersByTimeAsync(200);

    expect(unavailablePlay).not.toHaveBeenCalled();
    expect(readyPlay).toHaveBeenCalledOnce();
  });

  it('plays the same notification only once across ready tabs', async () => {
    const firstPlay = vi.fn(() => true);
    const secondPlay = vi.fn(() => true);
    const first = notifications(() => true, firstPlay);
    const second = notifications(() => true, secondPlay);
    first.handle(notification);
    second.handle(notification);

    await vi.advanceTimersByTimeAsync(200);

    expect(firstPlay.mock.calls.length + secondPlay.mock.calls.length).toBe(1);
  });

  it('ignores disabled and malformed notifications', async () => {
    const play = vi.fn(() => true);
    const instance = notifications(() => true, play);
    instance.update({ enabled: false, viewingSessionId: null });
    instance.handle(notification);
    instance.update({ enabled: true, viewingSessionId: null });
    instance.handle({ ...notification, id: 'notification-2', elapsedMs: NaN });

    await vi.runAllTimersAsync();

    expect(play).not.toHaveBeenCalled();
  });
});
