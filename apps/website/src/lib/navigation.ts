import { beforeNavigate, goto as svelteGoto } from '$app/navigation';

export type NavigationRequest = { reason: 'leave' | 'reload' | 'logout' | 'login'; paneIds?: readonly string[] };
export type NavigationPreparation = { isCurrent: () => boolean; release: () => void };
type NavigationInterceptor = (request: NavigationRequest) => Promise<NavigationPreparation | false>;

let interceptor: NavigationInterceptor | undefined;

export function registerNavigationInterceptor(next: NavigationInterceptor): () => void {
  interceptor = next;
  return () => {
    if (interceptor === next) interceptor = undefined;
  };
}

export async function runNavigation<T>(request: NavigationRequest, commit: () => T | Promise<T>): Promise<T | false> {
  const current = interceptor;
  if (!current) return commit();
  const preparation = await current(request);
  if (!preparation) return false;
  try {
    if (interceptor !== current || !preparation.isCurrent()) return false;
    return await commit();
  } finally {
    preparation.release();
  }
}

let preparedNavigation: { url: string } | undefined;
async function performNavigation(path: string | URL, options?: Parameters<typeof svelteGoto>[1]): Promise<void> {
  const navigation = { url: new URL(path, location.href).href };
  preparedNavigation = navigation;
  try {
    await svelteGoto(path, options);
  } finally {
    if (preparedNavigation === navigation) preparedNavigation = undefined;
  }
}

export async function goto(path: string | URL, options?: Parameters<typeof svelteGoto>[1]): Promise<void> {
  // beforeNavigate cannot expose the original goto options. Prepare first so
  // state, history replacement, focus, and invalidation options remain intact.
  await runNavigation({ reason: 'leave' }, () => performNavigation(path, options));
}

// Pane mutations already checkpoint the editors they replace. URL updates that
// only reflect their focus/tree must not stop retained editors a second time.
export async function reflectPaneUrl(path: string, options?: Parameters<typeof svelteGoto>[1]): Promise<void> {
  await performNavigation(path, options);
}

export function guardNavigation(): void {
  let pending = false;
  let historyNavigation: { url: string; complete: ReturnType<typeof Promise.withResolvers<undefined>> } | undefined;

  beforeNavigate((navigation) => {
    if (navigation.type === 'leave' || !navigation.to) return;
    const target = navigation.to.url.href;
    if (target === preparedNavigation?.url) return;
    if (target === historyNavigation?.url) {
      const { complete } = historyNavigation;
      historyNavigation = undefined;
      void navigation.complete.then(() => complete.resolve(undefined)).catch(complete.reject);
      return;
    }
    if (!interceptor) return;
    navigation.cancel();
    if (pending) return;
    pending = true;
    const restored =
      navigation.type === 'popstate' && navigation.from ? waitForHistoryRestoration(navigation.from.url.href) : Promise.resolve(true);
    void runNavigation({ reason: 'leave' }, async () => {
      if (!(await restored)) return;
      if (navigation.willUnload) {
        await performPageUnload(() => location.assign(target));
        return;
      }
      if (navigation.type === 'popstate' && navigation.delta) {
        const complete = Promise.withResolvers<undefined>();
        historyNavigation = { url: target, complete };
        history.go(navigation.delta);
        await complete.promise;
      } else {
        await performNavigation(target);
      }
    }).finally(() => {
      pending = false;
      historyNavigation = undefined;
    });
  });
}

// SvelteKit restores a cancelled back/forward navigation with another popstate.
// Replaying before that restoration would traverse from the wrong history entry.
function waitForHistoryRestoration(url: string): Promise<boolean> {
  return new Promise((resolve) => {
    const finish = (restored: boolean) => {
      window.removeEventListener('popstate', onPopState);
      clearTimeout(timer);
      resolve(restored);
    };
    const onPopState = () => {
      if (location.href === url) finish(true);
    };
    const timer = setTimeout(() => finish(false), 3000);
    window.addEventListener('popstate', onPopState);
  });
}

async function performPageUnload(action: () => void): Promise<void> {
  // Hold preparation through unload. If navigation is cancelled or fails, actual
  // interaction with the retained page releases it without replaying the request.
  const events = new AbortController();
  const resumed = Promise.withResolvers<undefined>();
  const options = { capture: true, signal: events.signal };
  window.addEventListener('pointerdown', () => resumed.resolve(undefined), options);
  window.addEventListener('keydown', () => resumed.resolve(undefined), options);
  window.addEventListener('pageshow', () => resumed.resolve(undefined), options);
  try {
    action();
    await resumed.promise;
  } finally {
    events.abort();
  }
}

let reload: Promise<unknown> | undefined;
export function reloadPage(): Promise<unknown> {
  return (reload ??= runNavigation({ reason: 'reload' }, () => performPageUnload(() => location.reload())).finally(() => {
    reload = undefined;
  }));
}

export function leavePage(url: string, reason: 'logout' | 'login', beforeLeave?: () => Promise<unknown>): Promise<unknown> {
  return runNavigation({ reason }, async () => {
    await beforeLeave?.();
    await performPageUnload(() => location.assign(url));
  });
}
