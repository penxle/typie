import { describe, expect, it, vi } from 'vitest';
import { BROWSER_PUSH_STORAGE_KEY, browserPushEnabled, createBrowserPushManager, readBrowserPushIntent } from './browser-push';
import type { BrowserPushDependencies } from './browser-push';

class MemoryStorage {
  readonly values = new Map<string, string>();

  getItem(key: string): string | null {
    return this.values.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    this.values.set(key, value);
  }
}

function setup(overrides: Partial<BrowserPushDependencies> = {}) {
  const storage = overrides.storage ?? new MemoryStorage();
  const dependencies: BrowserPushDependencies = {
    acquireToken: vi.fn(async () => 'browser-token'),
    deleteToken: vi.fn(async () => true),
    getPermission: () => 'granted',
    isSupported: vi.fn(async () => true),
    registerToken: vi.fn(() => Promise.resolve()),
    requestPermission: vi.fn(async (): Promise<NotificationPermission> => 'granted'),
    storage,
    unregisterToken: vi.fn(() => Promise.resolve()),
    ...overrides,
  };

  return { dependencies, manager: createBrowserPushManager(dependencies), storage };
}

describe('browser push lifecycle', () => {
  it('treats existing granted browsers as enabled and registers them on reconcile', async () => {
    const { dependencies, manager, storage } = setup();

    await expect(manager.reconcile()).resolves.toBe(true);

    expect(dependencies.registerToken).toHaveBeenCalledWith('browser-token');
    expect(readBrowserPushIntent(storage)).toMatchObject({ enabled: true });
    expect(browserPushEnabled('granted', readBrowserPushIntent(storage))).toBe(true);
  });

  it('reconciles an existing enabled intent without publishing another cross-tab operation', async () => {
    const { manager, storage } = setup();
    storage.setItem('typie:browser-push', JSON.stringify({ enabled: true, operationId: 'other-tab' }));

    await expect(manager.reconcile()).resolves.toBe(true);

    expect(readBrowserPushIntent(storage)).toEqual({ enabled: true, operationId: 'other-tab' });
  });

  it('does not create a new token while reconciling a browser that is already disabled', async () => {
    const { dependencies, manager, storage } = setup();
    storage.setItem('typie:browser-push', JSON.stringify({ enabled: false, operationId: 'existing' }));

    await expect(manager.reconcile()).resolves.toBe(true);

    expect(dependencies.acquireToken).not.toHaveBeenCalled();
    expect(dependencies.deleteToken).toHaveBeenCalledOnce();
  });

  it('keeps an explicit first enable visibly off when registration fails', async () => {
    const { manager, storage } = setup({
      registerToken: vi.fn(async () => {
        throw new Error('offline');
      }),
    });

    await expect(manager.enable()).resolves.toBe(false);

    expect(readBrowserPushIntent(storage)).toMatchObject({ enabled: false });
  });

  it('does not roll back or remove a newer enabled intent when an earlier enable fails', async () => {
    const storage = new MemoryStorage();
    const deleteToken = vi.fn(async () => true);
    const unregisterToken = vi.fn(() => Promise.resolve());
    const { manager } = setup({
      deleteToken,
      registerToken: vi.fn(async () => {
        storage.setItem('typie:browser-push', JSON.stringify({ enabled: true, operationId: 'newer-enable' }));
        throw new Error('offline');
      }),
      storage,
      unregisterToken,
    });

    await expect(manager.enable()).resolves.toBe(false);

    expect(readBrowserPushIntent(storage)).toEqual({ enabled: true, operationId: 'newer-enable' });
    expect(unregisterToken).not.toHaveBeenCalled();
    expect(deleteToken).not.toHaveBeenCalled();
  });

  it('notifies the current document when the browser preference changes', async () => {
    const listener = vi.fn();
    window.addEventListener('storage', listener);

    try {
      const { manager } = setup();

      await manager.disable();

      expect(listener).toHaveBeenCalledWith(expect.objectContaining({ key: BROWSER_PUSH_STORAGE_KEY }));
    } finally {
      window.removeEventListener('storage', listener);
    }
  });

  it('reports failure without changing registration when the browser preference cannot be stored', async () => {
    const storage = {
      getItem: () => null,
      setItem: () => {
        throw new DOMException('blocked', 'SecurityError');
      },
    };
    const { dependencies, manager } = setup({ storage });

    await expect(manager.disable()).resolves.toBe(false);

    expect(dependencies.acquireToken).not.toHaveBeenCalled();
    expect(dependencies.deleteToken).not.toHaveBeenCalled();
  });

  it('does not register when a disabled browser preference cannot be replaced', async () => {
    const storage = {
      getItem: () => JSON.stringify({ enabled: false, operationId: 'existing' }),
      setItem: () => {
        throw new DOMException('blocked', 'SecurityError');
      },
    };
    const { dependencies, manager } = setup({ storage });

    await expect(manager.enable()).resolves.toBe(false);

    expect(dependencies.acquireToken).not.toHaveBeenCalled();
    expect(dependencies.registerToken).not.toHaveBeenCalled();
  });

  it('keeps the browser disabled when Firebase deletion succeeds even if server unregister fails', async () => {
    const unregisterToken = vi.fn(async () => {
      throw new Error('server unavailable');
    });
    const { dependencies, manager, storage } = setup({ unregisterToken });

    await expect(manager.disable()).resolves.toBe(true);

    expect(unregisterToken).toHaveBeenCalledWith('browser-token');
    expect(dependencies.deleteToken).toHaveBeenCalledOnce();
    expect(readBrowserPushIntent(storage)).toMatchObject({ enabled: false });
  });

  it('restores registration when Firebase deletion fails', async () => {
    const registerToken = vi.fn(() => Promise.resolve());
    const { manager, storage } = setup({ deleteToken: vi.fn(async () => false), registerToken });

    await expect(manager.disable()).resolves.toBe(false);

    expect(registerToken).toHaveBeenCalledWith('browser-token');
    expect(readBrowserPushIntent(storage)).toMatchObject({ enabled: true });
  });

  it('compensates when another tab disables push during registration', async () => {
    const storage = new MemoryStorage();
    const unregisterToken = vi.fn(() => Promise.resolve());
    const deleteToken = vi.fn(async () => true);
    const { manager } = setup({
      deleteToken,
      registerToken: vi.fn(async () => {
        storage.setItem('typie:browser-push', JSON.stringify({ enabled: false, operationId: 'other-tab' }));
      }),
      storage,
      unregisterToken,
    });

    await expect(manager.enable()).resolves.toBe(false);

    expect(unregisterToken).toHaveBeenCalledWith('browser-token');
    expect(deleteToken).toHaveBeenCalledOnce();
    expect(readBrowserPushIntent(storage)).toEqual({ enabled: false, operationId: 'other-tab' });
  });

  it('cleans both registrations on logout without changing the browser preference', async () => {
    const { dependencies, manager, storage } = setup();
    storage.setItem('typie:browser-push', JSON.stringify({ enabled: true, operationId: 'existing' }));

    await expect(manager.cleanupForLogout()).resolves.toBeUndefined();

    expect(dependencies.unregisterToken).toHaveBeenCalledWith('browser-token');
    expect(dependencies.deleteToken).toHaveBeenCalledOnce();
    expect(readBrowserPushIntent(storage)).toEqual({ enabled: true, operationId: 'existing' });
  });
});
