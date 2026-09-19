// eslint-disable-next-line @typescript-eslint/no-require-imports -- Electron sandbox preloads use CommonJS
const { contextBridge, ipcRenderer } = require('electron');

let protectedChanges = true;
let pendingChanges = false;
let generation = crypto.getRandomValues(new Uint32Array(4)).join('-');
let replaceOnCommit = false;
let silent = false;
let recoveryNotifications = 0;
let progress = false;
let prepareDelay = 0;
let documents;
const stopped = new Set();
contextBridge.exposeInMainWorld('saveFixture', {
  configure: (config) => {
    protectedChanges = config.protected;
    pendingChanges = config.pending ?? false;
    silent = config.silent ?? false;
    replaceOnCommit = config.replaceOnCommit ?? false;
    prepareDelay = config.prepareDelay ?? 0;
    documents = config.documents;
  },
  stops: () => stopped.size,
  recoveryNotifications: () => recoveryNotifications,
  progress: () => progress,
  respondToDialog: (id, choice) => ipcRenderer.send('document-dialog:response', id, choice),
});
ipcRenderer.on('bridge:document-save-recovered', () => recoveryNotifications++);
ipcRenderer.on('bridge:document-save-progress', (_event, value) => (progress = value));
// Keep real renderer/frame identity while controlling document save responses.
ipcRenderer.on('document:save', async (_event, request) => {
  if (request.phase === 'release') stopped.delete(request.operationId);
  else if (request.phase === 'prepare') stopped.add(request.operationId);
  if (silent) return;
  if (prepareDelay && request.phase === 'prepare') await new Promise((resolve) => setTimeout(resolve, prepareDelay));
  if (replaceOnCommit && request.phase === 'commit') {
    replaceOnCommit = false;
    generation = crypto.getRandomValues(new Uint32Array(4)).join('-');
    protectedChanges = false;
  }
  ipcRenderer.send('document:saved', {
    id: request.id,
    generation,
    status: protectedChanges || request.discard ? 'protected' : pendingChanges ? 'pending' : 'failed',
    documents: documents ?? [
      {
        id: 'document',
        sessionId: 'session',
        title: '저장 실패 문서 — 긴 문서 이름도 잘려 사라지지 않고 확인할 수 있어야 해요',
        icon: 'book-open',
        iconColor: 'blue',
        status: protectedChanges ? 'protected' : pendingChanges ? 'pending' : 'failed',
        protectedChanges,
      },
    ],
  });
});
