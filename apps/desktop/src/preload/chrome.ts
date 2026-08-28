import '@sentry/electron/preload';

import { contextBridge, ipcRenderer } from 'electron';

contextBridge.exposeInMainWorld('shell', {
  platform: process.platform,
  newTab: () => ipcRenderer.send('tabs:new'),
  closeTab: (id: string) => ipcRenderer.send('tabs:close', id),
  activateTab: (id: string) => ipcRenderer.send('tabs:activate', id),
  moveTab: (id: string, toIndex: number) => ipcRenderer.send('tabs:move', id, toIndex),
  popupMenu: () => ipcRenderer.send('menu:popup'),
  restartToUpdate: () => ipcRenderer.send('update:restart'),
  onTabsState: (callback: (state: unknown) => void) => {
    const listener = (_event: unknown, state: unknown) => callback(state);
    ipcRenderer.on('tabs:state', listener);
    return () => ipcRenderer.removeListener('tabs:state', listener);
  },
  onCloseTabRequest: (callback: () => void) => {
    const listener = () => callback();
    ipcRenderer.on('tabs:request-close', listener);
    return () => ipcRenderer.removeListener('tabs:request-close', listener);
  },
  onTheme: (callback: (theme: unknown) => void) => {
    const listener = (_event: unknown, theme: unknown) => callback(theme);
    ipcRenderer.on('theme', listener);
    return () => ipcRenderer.removeListener('theme', listener);
  },
  onUpdateReady: (callback: () => void) => {
    const listener = () => callback();
    ipcRenderer.on('update:ready', listener);
    return () => ipcRenderer.removeListener('update:ready', listener);
  },
  onFullscreen: (callback: (fullscreen: boolean) => void) => {
    const listener = (_event: unknown, fullscreen: boolean) => callback(fullscreen);
    ipcRenderer.on('window:fullscreen', listener);
    return () => ipcRenderer.removeListener('window:fullscreen', listener);
  },
});
