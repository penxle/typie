export const BROWSER_PUSH_STORAGE_KEY = 'typie:browser-push';

export type BrowserPushIntent = {
  enabled: boolean;
  operationId: string;
};

type BrowserPushStorage = Pick<Storage, 'getItem' | 'setItem'>;

type BrowserPushOperation = {
  intent: BrowserPushIntent;
  persisted: boolean;
};

export type BrowserPushDependencies = {
  acquireToken: () => Promise<string | null>;
  deleteToken: () => Promise<boolean>;
  getPermission: () => NotificationPermission | null;
  isSupported: () => Promise<boolean>;
  registerToken: (token: string) => Promise<void>;
  requestPermission: () => Promise<NotificationPermission>;
  storage: BrowserPushStorage;
  unregisterToken: (token: string) => Promise<void>;
};

export type BrowserPushManager = {
  cleanupForLogout: () => Promise<void>;
  disable: () => Promise<boolean>;
  enable: () => Promise<boolean>;
  reconcile: () => Promise<boolean>;
};

export function readBrowserPushIntent(storage: BrowserPushStorage): BrowserPushIntent | null {
  try {
    const parsed: unknown = JSON.parse(storage.getItem(BROWSER_PUSH_STORAGE_KEY) ?? 'null');
    if (
      typeof parsed === 'object' &&
      parsed !== null &&
      typeof Reflect.get(parsed, 'enabled') === 'boolean' &&
      typeof Reflect.get(parsed, 'operationId') === 'string'
    ) {
      return parsed as BrowserPushIntent;
    }
  } catch {
    // Storage can be unavailable in private browsing modes.
  }
  return null;
}

export function readCurrentBrowserPushIntent(): BrowserPushIntent | null {
  try {
    return readBrowserPushIntent(localStorage);
  } catch {
    return null;
  }
}

export function browserPushEnabled(permission: NotificationPermission | null, intent: BrowserPushIntent | null): boolean {
  return permission === 'granted' && intent?.enabled !== false;
}

export function createBrowserPushManager(dependencies: BrowserPushDependencies): BrowserPushManager {
  const writeIntent = (enabled: boolean): BrowserPushOperation => {
    const intent = { enabled, operationId: crypto.randomUUID() };
    const serialized = JSON.stringify(intent);
    try {
      dependencies.storage.setItem(BROWSER_PUSH_STORAGE_KEY, serialized);
    } catch {
      return { intent, persisted: false };
    }

    // Native storage events only notify other documents.
    if (typeof window !== 'undefined' && typeof StorageEvent !== 'undefined') {
      window.dispatchEvent(new StorageEvent('storage', { key: BROWSER_PUSH_STORAGE_KEY, newValue: serialized }));
    }
    return { intent, persisted: true };
  };

  const operationIsCurrent = (operation: BrowserPushIntent): boolean =>
    readBrowserPushIntent(dependencies.storage)?.operationId === operation.operationId;

  const removeRegistration = async (token: string | null): Promise<boolean> => {
    const results = await Promise.allSettled([
      token === null ? Promise.resolve() : dependencies.unregisterToken(token),
      dependencies.deleteToken(),
    ]);
    return results[1].status === 'fulfilled' && results[1].value;
  };

  async function cleanupRegistration(operation: BrowserPushIntent, token: string | null): Promise<boolean> {
    const before = readBrowserPushIntent(dependencies.storage);
    if (before?.enabled === true && before.operationId !== operation.operationId) return false;

    const deleted = await removeRegistration(token);
    const after = readBrowserPushIntent(dependencies.storage);
    if (after?.enabled === true && after.operationId !== operation.operationId) await applyEnabledIntent(after);
    return deleted;
  }

  async function applyEnabledIntent(operation: BrowserPushIntent): Promise<boolean> {
    const token = await dependencies.acquireToken().catch(() => null);
    if (token === null) return false;

    try {
      await dependencies.registerToken(token);
    } catch {
      await cleanupRegistration(operation, token);
      return false;
    }

    const latest = readBrowserPushIntent(dependencies.storage);
    if (latest?.enabled === false) {
      await cleanupRegistration(latest, token);
      return false;
    }
    return true;
  }

  const enable = async (): Promise<boolean> => {
    if (!(await dependencies.isSupported())) return false;

    let permission = dependencies.getPermission();
    if (permission === 'default') {
      try {
        permission = await dependencies.requestPermission();
      } catch {
        writeIntent(false);
        return false;
      }
    }
    if (permission !== 'granted') {
      writeIntent(false);
      return false;
    }

    const operation = writeIntent(true);
    if (!operation.persisted && readBrowserPushIntent(dependencies.storage)?.enabled === false) return false;
    const succeeded = await applyEnabledIntent(operation.intent);
    if (!succeeded && operation.persisted && operationIsCurrent(operation.intent)) writeIntent(false);
    return succeeded;
  };

  const disable = async (): Promise<boolean> => {
    const operation = writeIntent(false);
    if (!operation.persisted) return false;
    if (dependencies.getPermission() !== 'granted') return true;
    const token = await dependencies.acquireToken().catch(() => null);
    const deleted = await cleanupRegistration(operation.intent, token);
    const latest = readBrowserPushIntent(dependencies.storage);

    if (latest?.enabled === true) {
      return false;
    }
    if (!operationIsCurrent(operation.intent)) return deleted;
    if (deleted) return true;

    writeIntent(true);
    if (token !== null) await dependencies.registerToken(token).catch(() => null);
    return false;
  };

  const reconcile = async (): Promise<boolean> => {
    if (!(await dependencies.isSupported()) || dependencies.getPermission() !== 'granted') return false;
    const intent = readBrowserPushIntent(dependencies.storage);
    if (intent?.enabled === false) return cleanupRegistration(intent, null);
    return applyEnabledIntent(intent ?? writeIntent(true).intent);
  };

  const cleanupForLogout = async (): Promise<void> => {
    if (!(await dependencies.isSupported()) || dependencies.getPermission() !== 'granted') return;
    const token =
      readBrowserPushIntent(dependencies.storage)?.enabled === false ? null : await dependencies.acquireToken().catch(() => null);
    await removeRegistration(token);
  };

  return { cleanupForLogout, disable, enable, reconcile };
}
