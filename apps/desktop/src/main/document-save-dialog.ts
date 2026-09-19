import { BrowserWindow, dialog, ipcMain, nativeTheme } from 'electron';
import { defaultThemePayload, themeColors } from './theme';
import { preloadPath, rendererUrl } from './window-manager';
import type { DocumentSaveDialogAction, DocumentSaveDialogData } from '@typie/lib/desktop';
import type { BaseWindow, IpcMainEvent } from 'electron';
import type { ThemePayload } from './theme';

// cspell:ignore minimizable maximizable

type DialogResult = { action: DocumentSaveDialogAction | 'aborted'; completionShown: boolean };

export function showDocumentSaveDialog(
  window: BaseWindow,
  data: DocumentSaveDialogData,
  signal: AbortSignal,
  theme: ThemePayload = defaultThemePayload(nativeTheme.shouldUseDarkColors ? 'dark' : 'light'),
) {
  const result = Promise.withResolvers<DialogResult>();
  const nativeDialog = new AbortController();
  const url = rendererUrl('document-save', theme);
  let current = data;
  let child: BrowserWindow | undefined;
  let mode: 'app' | 'system' | 'closed' = 'app';
  let watchdog: ReturnType<typeof setTimeout>;
  let completionShown = false;

  const disposeAppDialog = () => {
    clearTimeout(watchdog);
    ipcMain.removeListener('document-dialog:response', receive);
    ipcMain.removeListener('document-dialog:ready', present);
    ipcMain.removeListener('document-dialog:height', resize);
    if (child && !child.isDestroyed()) {
      child.webContents.removeListener('render-process-gone', unavailable);
      child.webContents.removeListener('destroyed', unavailable);
      child.destroy();
    }
  };
  const finish = (action: DialogResult['action']) => {
    if (mode === 'closed') return;
    mode = 'closed';
    disposeAppDialog();
    signal.removeEventListener('abort', abort);
    window.removeListener('closed', abort);
    nativeDialog.abort();
    result.resolve({ action, completionShown });
  };
  const abort = () => finish('aborted');
  const showSystemDialog = async () => {
    if (mode !== 'app') return;
    mode = 'system';
    disposeAppDialog();
    if (signal.aborted || window.isDestroyed()) return abort();
    const lines = current.documents.map((document) =>
      document.status === 'unknown' ? `${document.title} — 저장 상태를 확인할 수 없어요` : `${document.title} — ${document.location}`,
    );
    try {
      const { response } = await dialog.showMessageBox(window, {
        type: 'warning',
        message: current.title,
        detail: `${current.description}\n\n${lines.join('\n')}`,
        buttons: [current.cancelLabel, current.discardLabel],
        defaultId: 0,
        cancelId: 0,
        noLink: true,
        signal: nativeDialog.signal,
      });
      if (response === 1) finish('discard');
      else finish(current.required ? 'retry' : 'cancel');
    } catch {
      abort();
    }
  };
  // An unavailable document renderer remains an unknown row. Only failure of
  // this app-owned confirmation window requires a system dialog.
  const unavailable = () => void showSystemDialog();
  const valid = (event: IpcMainEvent) =>
    mode === 'app' &&
    child &&
    !child.isDestroyed() &&
    event.sender === child.webContents &&
    event.senderFrame === child.webContents.mainFrame &&
    event.senderFrame.url === url;
  const present = (event: IpcMainEvent) => {
    if (valid(event)) child?.webContents.send('document-dialog:show', current);
  };
  const resize = (event: IpcMainEvent, height: unknown) => {
    if (typeof height === 'number' && height > 0 && Number.isFinite(height) && valid(event))
      child?.setContentSize(520, Math.max(220, Math.min(600, Math.ceil(height))));
  };
  const receive = (event: IpcMainEvent, id: unknown, action: unknown, height: unknown, completed: unknown) => {
    if (!valid(event) || id !== current.id) return;
    if (action === 'ready') {
      if (typeof height !== 'number' || !Number.isFinite(height) || height <= 0) return;
      resize(event, height);
      if (completed === true && current.completed) completionShown = true;
      clearTimeout(watchdog);
      watchdog = setTimeout(unavailable, 4000);
      if (child && !child.isVisible()) {
        if (window.isMinimized()) window.restore();
        window.show();
        child.show();
      }
    } else if (
      action === 'discard' ||
      (action === 'continue' && current.completed) ||
      (action === 'cancel' && !current.required) ||
      (action === 'retry' && current.required)
    ) {
      finish(action);
    }
  };

  signal.addEventListener('abort', abort, { once: true });
  window.once('closed', abort);
  if (signal.aborted || window.isDestroyed()) abort();
  else {
    try {
      child = new BrowserWindow({
        parent: window,
        modal: true,
        show: false,
        width: 520,
        height: 360,
        useContentSize: true,
        title: '저장 확인',
        // AppKit removes the native title bar when presenting a sheet. Starting
        // frameless would subtract the title-bar height from the content.
        frame: process.platform === 'darwin',
        resizable: false,
        minimizable: false,
        maximizable: false,
        backgroundColor: themeColors(theme).background,
        webPreferences: { preload: preloadPath('document-save'), sandbox: true, contextIsolation: true, backgroundThrottling: false },
      });
      child.setMenu(null);
      if (process.platform === 'darwin') child.setWindowButtonVisibility(false);
      child.webContents.setWindowOpenHandler(() => ({ action: 'deny' }));
      child.webContents.on('will-navigate', (event) => event.preventDefault());
      child.webContents.on('will-redirect', (event) => event.preventDefault());
      child.webContents.setIgnoreMenuShortcuts(true);
      child.webContents.on('render-process-gone', unavailable);
      child.webContents.on('destroyed', unavailable);
      child.on('close', (event) => {
        event.preventDefault();
        if (!current.required) finish('cancel');
      });
      ipcMain.on('document-dialog:response', receive);
      ipcMain.on('document-dialog:ready', present);
      ipcMain.on('document-dialog:height', resize);
      watchdog = setTimeout(unavailable, 4000);
      void child.loadURL(url).catch(unavailable);
    } catch {
      unavailable();
    }
  }
  return {
    result: result.promise,
    get usingAppDialog() {
      return mode === 'app';
    },
    update: (update: Partial<Omit<DocumentSaveDialogData, 'id'>>) => {
      current = { ...current, ...update };
      if (!current.completed) completionShown = false;
      if (mode === 'app' && child && !child.isDestroyed()) {
        try {
          child.webContents.send('document-dialog:show', current);
        } catch {
          unavailable();
        }
      }
    },
  };
}
