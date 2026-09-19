import { contextBridge, ipcRenderer } from 'electron';
import type { DocumentSaveDialogAction, DocumentSaveDialogData } from '@typie/lib/desktop';

contextBridge.exposeInMainWorld('documentSaveDialog', {
  subscribe: (callback: (data: DocumentSaveDialogData) => void) => {
    const receive = (_event: unknown, data: DocumentSaveDialogData) => callback(data);
    ipcRenderer.on('document-dialog:show', receive);
    ipcRenderer.send('document-dialog:ready');
    return () => ipcRenderer.removeListener('document-dialog:show', receive);
  },
  ready: (id: string, height: number, completed = false) => ipcRenderer.send('document-dialog:response', id, 'ready', height, completed),
  respond: (id: string, action: DocumentSaveDialogAction) => ipcRenderer.send('document-dialog:response', id, action),
  resize: (height: number) => ipcRenderer.send('document-dialog:height', height),
});
