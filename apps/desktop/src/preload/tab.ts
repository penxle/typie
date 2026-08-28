import { contextBridge, ipcRenderer } from 'electron';
import type { TabIcon, TypieDesktopBridge } from '@typie/lib/desktop';

const IPC_THEME_CHANGED = 'theme:changed';
const IPC_CONTEXT_MENU = 'contextmenu:request';
const IPC_PAGE_RETRY = 'page:retry';
const IPC_TAB_ICON = 'tab:icon';
const IPC_TAB_OPEN = 'tab:open';

const RENDERER_DEV_PORT = '5300';

const resolveTheme = (value: string | undefined) => {
  if (value === 'dark' || value === 'light') return value;
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
};

const readTheme = () => {
  const root = document.documentElement;
  return {
    theme: resolveTheme(root.dataset.theme),
    variantLight: root.dataset.variantLight ?? 'white',
    variantDark: root.dataset.variantDark ?? 'black',
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

const listeners = { focus: new Set<() => void>(), preference: new Set<() => void>() };
ipcRenderer.on('bridge:focus', () => {
  for (const listener of listeners.focus) listener();
});
ipcRenderer.on('bridge:preference', () => {
  for (const listener of listeners.preference) listener();
});

contextBridge.exposeInMainWorld(
  'typieDesktop',
  Object.freeze({
    version: appVersion,
    platform: process.platform as 'darwin' | 'win32',
    openExternal: (url: string) => ipcRenderer.invoke('bridge:open-external', url) as Promise<void>,
    on: (event: 'focus' | 'preference', callback: () => void) => {
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
  } satisfies TypieDesktopBridge),
);

if (location.protocol === 'file:' || location.port === RENDERER_DEV_PORT) {
  contextBridge.exposeInMainWorld('shell', {
    platform: process.platform,
    retry: () => ipcRenderer.send(IPC_PAGE_RETRY),
  });
}
