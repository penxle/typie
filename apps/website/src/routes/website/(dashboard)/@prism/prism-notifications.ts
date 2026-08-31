export type PrismNotification = {
  id: string;
  sessionId: string;
  kind: 'TURN_RESOLVED' | 'USER_ACTION_REQUIRED';
  elapsedMs: number;
};

export type PrismNotificationSound = 'resolved' | 'action-required';

const LONG_RESPONSE_MS = 30_000;
const BACKGROUND_SETTLE_MS = 80;
const HANDLED_TTL_MS = 24 * 60 * 60 * 1000;
const HANDLED_LIMIT = 100;

const delay = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

const pageActive = () => document.visibilityState === 'visible' && document.hasFocus();

const notificationSound = (args: {
  notification: PrismNotification;
  enabled: boolean;
  viewingSessionId: string | null;
}): PrismNotificationSound | null => {
  const { notification } = args;
  if (!args.enabled || !Number.isFinite(notification.elapsedMs) || notification.elapsedMs < 0) return null;
  if (notification.elapsedMs < LONG_RESPONSE_MS && pageActive() && args.viewingSessionId === notification.sessionId) return null;
  return notification.kind === 'USER_ACTION_REQUIRED' ? 'action-required' : 'resolved';
};

export const createPrismNotifications = (args: {
  userId: string;
  enabled: boolean;
  viewingSessionId: string | null;
  canPlay: (sound: PrismNotificationSound) => boolean;
  play: (sound: PrismNotificationSound) => boolean;
}) => {
  const tabId = crypto.randomUUID();
  const keyPrefix = `typie:prism-notifications:${args.userId}`;
  const handledKey = `${keyPrefix}:handled`;
  const lockPrefix = `${keyPrefix}:notification:`;
  const pending = new Set<string>();
  const handled = new Set<string>();
  let enabled = args.enabled;
  let viewingSessionId = args.viewingSessionId;

  const readHandled = (): { id: string; at: number }[] => {
    try {
      const value: unknown = JSON.parse(localStorage.getItem(handledKey) ?? '[]');
      if (!Array.isArray(value)) return [];
      const now = Date.now();
      return value.filter((entry): entry is { id: string; at: number } => {
        if (typeof entry !== 'object' || entry === null) return false;
        const candidate = entry as { id?: unknown; at?: unknown };
        return typeof candidate.id === 'string' && typeof candidate.at === 'number' && now - candidate.at <= HANDLED_TTL_MS;
      });
    } catch {
      return [];
    }
  };

  const isHandled = (notificationId: string) => {
    if (handled.has(notificationId)) return true;
    const found = readHandled().some((entry) => entry.id === notificationId);
    if (found) handled.add(notificationId);
    return found;
  };

  const markHandled = (notificationId: string) => {
    handled.add(notificationId);
    try {
      const entries = [{ id: notificationId, at: Date.now() }, ...readHandled().filter((entry) => entry.id !== notificationId)].slice(
        0,
        HANDLED_LIMIT,
      );
      localStorage.setItem(handledKey, JSON.stringify(entries));
    } catch {
      // In-memory deduplication still covers this tab.
    }
  };

  const claim = async (notificationId: string, owned: () => boolean) => {
    const navigatorWithLocks = navigator as Navigator & { locks?: LockManager };
    if (navigatorWithLocks.locks) {
      await navigatorWithLocks.locks.request(`${lockPrefix}${notificationId}`, () => {
        if (isHandled(notificationId) || !owned()) return;
        markHandled(notificationId);
      });
      return;
    }

    const claimKey = `${lockPrefix}${notificationId}`;
    const claimValue = `${tabId}:${crypto.randomUUID()}`;
    let claimed: boolean;
    try {
      localStorage.setItem(claimKey, claimValue);
      await delay(40 + Math.random() * 40);
      claimed = localStorage.getItem(claimKey) === claimValue;
    } catch {
      claimed = true;
    }

    try {
      if (!claimed || isHandled(notificationId) || !owned()) return;
      markHandled(notificationId);
    } finally {
      try {
        if (localStorage.getItem(claimKey) === claimValue) localStorage.removeItem(claimKey);
      } catch {
        // Storage can be unavailable in restricted browsing contexts.
      }
    }
  };

  const settle = async (notification: PrismNotification) => {
    pending.add(notification.id);
    if (!pageActive()) await delay(BACKGROUND_SETTLE_MS);

    try {
      const sound = notificationSound({ notification, enabled, viewingSessionId });
      if (sound !== null && !args.canPlay(sound)) return;
      await claim(notification.id, () => sound === null || args.play(sound));
    } finally {
      pending.delete(notification.id);
    }
  };

  return {
    update: (next: { enabled: boolean; viewingSessionId: string | null }) => {
      enabled = next.enabled;
      viewingSessionId = next.viewingSessionId;
    },
    handle: (notification: PrismNotification) => {
      if (handled.has(notification.id) || pending.has(notification.id)) return;
      void settle(notification);
    },
  };
};
