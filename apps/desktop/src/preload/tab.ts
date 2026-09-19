import { DEFAULT_DARK_VARIANT, DEFAULT_LIGHT_VARIANT } from '@typie/styled-system/presets';
import { contextBridge, ipcRenderer } from 'electron';
import type {
  DesktopBridgeListeners,
  DesktopZoomAction,
  DocumentSaveRequest,
  DocumentSaveResult,
  TabIcon,
  TypieDesktopBridge,
} from '@typie/lib/desktop';

const IPC_THEME_CHANGED = 'theme:changed';
const IPC_CONTEXT_MENU = 'contextmenu:request';
const IPC_PAGE_RETRY = 'page:retry';
const IPC_TAB_ICON = 'tab:icon';
const IPC_TAB_OPEN = 'tab:open';
const IPC_BRIDGE_ZOOM = 'bridge:zoom';
const IPC_BRIDGE_ZOOM_SHORTCUT = 'bridge:zoom-shortcut';

const RENDERER_DEV_PORT = '5400';

const resolveTheme = (value: string | undefined) => {
  if (value === 'dark' || value === 'light') return value;
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
};

const readTheme = () => {
  const root = document.documentElement;
  return {
    theme: resolveTheme(root.dataset.theme),
    variantLight: root.dataset.variantLight ?? DEFAULT_LIGHT_VARIANT,
    variantDark: root.dataset.variantDark ?? DEFAULT_DARK_VARIANT,
  };
};

const publishTheme = () => ipcRenderer.send(IPC_THEME_CHANGED, readTheme());

window.addEventListener('DOMContentLoaded', () => {
  publishTheme();
  new MutationObserver(publishTheme).observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme', 'data-variant-light', 'data-variant-dark'],
  });
});

window.addEventListener(
  'contextmenu',
  (event) => {
    setTimeout(() => {
      if (event.defaultPrevented) return;
      const target = event.target as HTMLElement | null;
      const anchor = target?.closest('a[href]') as HTMLAnchorElement | null;
      const editable = !!target?.closest('input, textarea, [contenteditable=""], [contenteditable="true"]');
      ipcRenderer.send(IPC_CONTEXT_MENU, {
        x: event.clientX,
        y: event.clientY,
        linkURL: anchor?.href ?? '',
        selectionText: window.getSelection()?.toString() ?? '',
        isEditable: editable,
      });
    }, 0);
  },
  { capture: false },
);

const appVersion = process.argv.find((arg) => arg.startsWith('--typie-app-version='))?.split('=')[1] ?? '0.0.0';

let documentSave: ((request: DocumentSaveRequest) => Promise<DocumentSaveResult>) | undefined;
const documentGeneration = crypto.getRandomValues(new Uint32Array(4)).join('-');
let saveRegistration = 0;
const departures = new Map<string, { committed: boolean }>();
ipcRenderer.on('document:save', async (_event, request: DocumentSaveRequest) => {
  if (request.phase === 'release') departures.delete(request.operationId);
  const commit = request.phase === 'commit' ? { committed: false } : undefined;
  if (commit) departures.set(request.operationId, commit);
  const registration = saveRegistration;
  try {
    let result = (await documentSave?.(request)) ?? { status: 'unknown', documents: [] };
    if (registration !== saveRegistration || (commit && departures.get(request.operationId) !== commit)) {
      result = { status: 'unknown', documents: [] };
    }
    if (commit && departures.get(request.operationId) === commit) {
      if (result.status === 'protected') commit.committed = true;
      else departures.delete(request.operationId);
    }
    ipcRenderer.send('document:saved', {
      ...result,
      id: request.id,
      // Pane replacement can change editing sessions without replacing the page
      // or its handler. Bind approval to all three identities.
      generation: JSON.stringify([documentGeneration, saveRegistration, result.generation]),
    });
  } catch (err) {
    if (commit && departures.get(request.operationId) === commit) departures.delete(request.operationId);
    console.error('Document save response could not be delivered', err);
    ipcRenderer.send('document:saved', {
      status: 'unknown',
      documents: [],
      id: request.id,
      generation: JSON.stringify([documentGeneration, saveRegistration, null]),
    });
  }
});

const listeners: { [Event in keyof DesktopBridgeListeners]: Set<DesktopBridgeListeners[Event]> } = {
  focus: new Set(),
  preference: new Set(),
  'document-save-recovered': new Set(),
  'document-save-progress': new Set(),
  'zoom-shortcut': new Set(),
};
ipcRenderer.on('bridge:focus', () => {
  for (const listener of listeners.focus) listener();
});
ipcRenderer.on('bridge:preference', () => {
  for (const listener of listeners.preference) listener();
});
ipcRenderer.on('bridge:document-save-recovered', () => {
  for (const listener of listeners['document-save-recovered']) listener();
});
ipcRenderer.on('bridge:document-save-progress', (_event, visible: boolean) => {
  for (const listener of listeners['document-save-progress']) listener(visible);
});
ipcRenderer.on(IPC_BRIDGE_ZOOM_SHORTCUT, (_event, action: DesktopZoomAction) => {
  const handled = [...listeners['zoom-shortcut']].some((listener) => listener(action));
  if (!handled) ipcRenderer.send(IPC_BRIDGE_ZOOM, action);
});

contextBridge.exposeInMainWorld(
  'typieDesktop',
  Object.freeze({
    version: appVersion,
    platform: process.platform as 'darwin' | 'win32',
    openExternal: (url: string) => ipcRenderer.invoke('bridge:open-external', url) as Promise<void>,
    on: <Event extends keyof DesktopBridgeListeners>(event: Event, callback: DesktopBridgeListeners[Event]) => {
      const set = listeners[event];
      // eslint-disable-next-line @typescript-eslint/no-empty-function
      if (!set) return () => {};
      set.add(callback);
      return () => {
        set.delete(callback);
      };
    },
    setTabIcon: (icon: TabIcon) => ipcRenderer.send(IPC_TAB_ICON, icon),
    openTab: (url: string) => ipcRenderer.send(IPC_TAB_OPEN, url),
    onDocumentSave: (handler: (request: DocumentSaveRequest) => Promise<DocumentSaveResult>) => {
      // The HTML's empty response is replaced before any editor is created.
      // A committed close must not race with that replacement and admit edits.
      if ([...departures.values()].some((departure) => departure.committed))
        throw new Error('Document departure has already been approved');
      documentSave = handler;
      saveRegistration++;
      return () => {
        if (documentSave === handler) {
          documentSave = undefined;
          saveRegistration++;
        }
      };
    },
    requestDocumentDeparture: (reason: 'logout' | 'login') => ipcRenderer.invoke('document:departure', reason) as Promise<void>,
  } satisfies TypieDesktopBridge),
);

if (location.protocol === 'file:' || location.port === RENDERER_DEV_PORT) {
  contextBridge.exposeInMainWorld('shell', {
    platform: process.platform,
    retry: () => ipcRenderer.send(IPC_PAGE_RETRY),
  });
}
