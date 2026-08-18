import { contextBridge, ipcRenderer } from 'electron';

contextBridge.exposeInMainWorld('shell', {
  platform: process.platform,
  login: () => ipcRenderer.send('auth:login'),
  retry: () => ipcRenderer.send('page:retry'),
  onAuthError: (callback: (message: string) => void) => {
    const listener = (_event: unknown, message: string) => callback(message);
    ipcRenderer.on('auth:error', listener);
    return () => ipcRenderer.removeListener('auth:error', listener);
  },
});
