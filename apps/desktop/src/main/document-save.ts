import { randomUUID } from 'node:crypto';
import { DOCUMENT_SAVE_ACTION_LABELS, getDocumentSaveDialogText } from '@typie/lib/document-save';
import { ipcMain } from 'electron';
import { showDocumentSaveDialog } from './document-save-dialog';
import type { DocumentDepartureReason, DocumentSaveDialogData, DocumentSaveRequest, DocumentSaveResult } from '@typie/lib/desktop';
import type { BaseWindow, IpcMainEvent, WebContents } from 'electron';
import type { ThemePayload } from './theme';

export type DocumentSaveTarget = { id: string; title: string; webContents: WebContents };
const unknown: DocumentSaveResult = { status: 'unknown', documents: [] };

export function requestDocumentSave(
  target: DocumentSaveTarget,
  request: Omit<DocumentSaveRequest, 'id'>,
  signal?: AbortSignal,
): Promise<DocumentSaveResult> {
  const wc = target.webContents;
  if (wc.isDestroyed() || signal?.aborted) return Promise.resolve(unknown);
  const frame = wc.mainFrame;
  const id = randomUUID();
  return new Promise((resolve) => {
    const finish = (result: DocumentSaveResult) => {
      clearTimeout(timer);
      ipcMain.removeListener('document:saved', receive);
      signal?.removeEventListener('abort', abort);
      resolve(result);
    };
    const abort = () => finish(unknown);
    const receive = (event: IpcMainEvent, payload: unknown) => {
      if (
        !payload ||
        typeof payload !== 'object' ||
        event.sender !== wc ||
        event.senderFrame !== frame ||
        !('id' in payload) ||
        payload.id !== id
      )
        return;
      if (
        !('status' in payload) ||
        !['protected', 'pending', 'failed', 'unknown'].includes(String(payload.status)) ||
        !('documents' in payload) ||
        !Array.isArray(payload.documents)
      ) {
        finish(unknown);
        return;
      }
      const documents = payload.documents.filter(
        (entry): entry is DocumentSaveResult['documents'][number] =>
          entry !== null &&
          typeof entry === 'object' &&
          typeof entry.id === 'string' &&
          typeof entry.sessionId === 'string' &&
          typeof entry.title === 'string',
      );
      finish({
        status: payload.status as DocumentSaveResult['status'],
        generation: 'generation' in payload && typeof payload.generation === 'string' ? payload.generation : undefined,
        documents: documents.slice(0, 100).map(({ id, sessionId, title, icon, iconColor, status, protectedChanges }) => ({
          id: id.slice(0, 500),
          sessionId: sessionId.slice(0, 500),
          title: title.slice(0, 500),
          icon: typeof icon === 'string' ? icon.slice(0, 100) : undefined,
          iconColor: typeof iconColor === 'string' ? iconColor.slice(0, 100) : undefined,
          status: ['protected', 'synced', 'pending', 'failed', 'sync-failed', 'unknown'].includes(String(status)) ? status : undefined,
          protectedChanges: protectedChanges === true,
        })),
      });
    };
    const timer = setTimeout(() => finish(unknown), 4000);
    ipcMain.on('document:saved', receive);
    signal?.addEventListener('abort', abort, { once: true });
    try {
      wc.send('document:save', { ...request, id });
    } catch {
      finish(unknown);
    }
  });
}

type PreparedTarget = {
  target: DocumentSaveTarget;
  result: DocumentSaveResult;
  request?: Promise<void>;
};

// Keep rows that were shown through recovery. An empty/unknown response may
// retain a document's identity, but cannot assert its previous storage state.
function updateDocuments(
  displayed: Map<WebContents, Map<string, DocumentSaveDialogData['documents'][number]>>,
  targets: PreparedTarget[],
): DocumentSaveDialogData['documents'] {
  for (const { target, result } of targets) {
    let rows = displayed.get(target.webContents);
    if (!rows) {
      rows = new Map();
      displayed.set(target.webContents, rows);
    }
    const location = target.title || '탭';
    const placeholder = `tab:${target.id}`;
    if (result.documents.length > 0) {
      rows.delete(placeholder);
      const received = new Set<string>();
      for (const document of result.documents) {
        const id = `${target.id}:${document.sessionId}`;
        received.add(id);
        const protectedChanges = result.status === 'protected' || (result.status !== 'unknown' && document.protectedChanges === true);
        if (protectedChanges && !rows.has(id)) continue;
        rows.set(id, {
          ...document,
          id,
          location,
          protectedChanges,
          status: result.status === 'unknown' ? 'unknown' : (document.status ?? 'unknown'),
        });
      }
      if (result.status === 'protected') {
        for (const [id, row] of rows) {
          if (!received.has(id)) rows.set(id, { ...row, location, status: 'unknown', protectedChanges: true });
        }
      }
    } else if (rows.size > 0) {
      for (const [id, row] of rows) {
        rows.set(id, {
          ...row,
          location: id === placeholder ? '' : location,
          status: id === placeholder && result.status === 'pending' ? 'pending' : 'unknown',
          protectedChanges: result.status === 'protected',
        });
      }
    } else if (result.status !== 'protected') {
      rows.set(placeholder, {
        id: placeholder,
        title: location,
        location: '',
        status: result.status === 'pending' ? 'pending' : 'unknown',
        protectedChanges: false,
      });
    }
  }
  return [...displayed.values()].flatMap((rows) => [...rows.values()]);
}

type DocumentDepartureOptions = {
  targets: () => DocumentSaveTarget[];
  reason: DocumentDepartureReason;
  window: BaseWindow;
  commit: () => void | Promise<void>;
  getActiveContents?: () => WebContents | undefined;
  onProgress?: (targets: DocumentSaveTarget[], visible: boolean) => void;
  targetChanges?: EventTarget;
  theme?: ThemePayload;
};

type DocumentSaveConfirmation =
  | { action: 'cancel'; dialogShown: boolean }
  | { action: 'retry'; dialogShown: boolean }
  | {
      action: 'proceed';
      targets: { target: DocumentSaveTarget; generation: string | undefined }[];
      discard: boolean;
      dialogShown: boolean;
      completionShown: boolean;
    };

function matchesTargets(selected: { target: DocumentSaveTarget }[], targets: DocumentSaveTarget[]): boolean {
  return (
    selected.length === targets.length && selected.every(({ target }) => targets.some((next) => next.webContents === target.webContents))
  );
}

// One confirmation owns its requests, dialog, polling, and progress UI. Returning
// a decision tears these down before the caller revalidates or starts a retry.
async function confirmDocumentSave(
  { targets, reason, window, getActiveContents, onProgress, targetChanges, theme }: DocumentDepartureOptions,
  operationId: string,
  participants: Map<WebContents, PreparedTarget>,
): Promise<DocumentSaveConfirmation> {
  const displayed = new Map<WebContents, Map<string, DocumentSaveDialogData['documents'][number]>>();
  const controller = new AbortController();
  const automatic = Promise.withResolvers<{ action: 'retry' | 'continue'; completionShown: false }>();
  const progressHosts = new Set<WebContents>();
  const required = reason === 'login';
  const labels = DOCUMENT_SAVE_ACTION_LABELS[reason === 'close' ? 'leave' : reason];
  // Approval covers only the targets and generations actually presented. The
  // canonical results can keep changing while the user decides what to do.
  let presented: { target: DocumentSaveTarget; generation: string | undefined }[] = [];
  let prompt: ReturnType<typeof showDocumentSaveDialog> | undefined;
  let progressVisible = false;
  let poll: ReturnType<typeof setInterval> | undefined;
  const hideProgress = () => {
    progressVisible = false;
    clearTimeout(progress);
    if (!window.isDestroyed()) window.setProgressBar(-1);
    for (const host of progressHosts) {
      if (!host.isDestroyed()) host.send('bridge:document-save-progress', false);
    }
    progressHosts.clear();
  };
  const dialogData = (selected: PreparedTarget[]) => {
    const completed = selected.every(({ result }) => result.status === 'protected');
    return {
      ...getDocumentSaveDialogText({
        reason: reason === 'close' ? 'leave' : reason,
        pending:
          selected.some(({ result }) => result.status === 'pending') &&
          selected.every(({ result }) => result.status === 'protected' || result.status === 'pending'),
        unknown: selected.some(({ result }) => result.status === 'unknown'),
        completed,
      }),
      documents: updateDocuments(displayed, selected),
      completed,
      continueLabel: labels.proceed,
    };
  };
  const present = (selected: PreparedTarget[]) => {
    const selectedTargets = selected.map(({ target }) => target);
    const targetsChanged = !matchesTargets(presented, selectedTargets);
    if (targetsChanged && (progressVisible || prompt)) onProgress?.(selectedTargets, true);
    if (!prompt) return;
    if (!prompt.usingAppDialog) {
      // System dialogs cannot update their target list or show a countdown.
      if (targetsChanged) {
        automatic.resolve({ action: 'retry', completionShown: false });
      } else if (selected.every(({ result }) => result.status === 'protected')) {
        automatic.resolve({ action: 'continue', completionShown: false });
      }
      return;
    }
    presented = selected.map(({ target, result }) => ({ target, generation: result.generation }));
    prompt.update(dialogData(selected));
  };

  // Initial preparation, polling, and newly selected tabs all take this path.
  // Each tab has at most one request; a late tab starts without waiting for its siblings.
  const refresh = async (phase: 'prepare' | 'check' = 'check'): Promise<PreparedTarget[]> => {
    while (!controller.signal.aborted) {
      const selected = targets().map((target) => {
        let entry = participants.get(target.webContents);
        const requestPhase = entry ? phase : 'prepare';
        if (!entry) {
          entry = { target, result: { status: 'pending', documents: [] } };
          participants.set(target.webContents, entry);
        }
        entry.target = target;
        if (!entry.request) {
          const participant = entry;
          participant.request = requestDocumentSave(target, { operationId, phase: requestPhase }, controller.signal).then((result) => {
            participant.request = undefined;
            if (controller.signal.aborted) return;
            if (requestPhase === 'check' && result.generation !== undefined && result.generation !== participant.result.generation) {
              automatic.resolve({ action: 'retry', completionShown: false });
            }
            participant.result = result;
          });
        }
        return entry;
      });
      // Invalidate a visible completion countdown before awaiting a newly added tab.
      present(selected);
      await Promise.all(selected.map(({ request }) => request));
      if (controller.signal.aborted) break;
      if (matchesTargets(selected, targets())) {
        present(selected);
        return selected;
      }
      phase = 'check';
    }
    return [];
  };
  const targetsChanged = () => void refresh();
  targetChanges?.addEventListener('change', targetsChanged);
  const progress = setTimeout(() => {
    progressVisible = true;
    if (!window.isDestroyed()) window.setProgressBar(2);
    onProgress?.(targets(), true);
    for (const host of [...targets().map((target) => target.webContents), getActiveContents?.()]) {
      if (!host || host.isDestroyed()) continue;
      progressHosts.add(host);
      host.send('bridge:document-save-progress', true);
    }
  }, 350);

  try {
    const selected = await refresh('prepare').finally(hideProgress);
    if (window.isDestroyed()) return { action: 'cancel', dialogShown: false };
    if (!matchesTargets(selected, targets())) return { action: 'retry', dialogShown: false };
    presented = selected.map(({ target, result }) => ({ target, generation: result.generation }));

    let discard = false;
    let completionShown = false;
    if (selected.some(({ result }) => result.status !== 'protected')) {
      onProgress?.(
        selected.map(({ target }) => target),
        true,
      );
      prompt = showDocumentSaveDialog(
        window,
        {
          id: randomUUID(),
          ...dialogData(selected),
          required,
          cancelLabel: required ? '다시 시도' : labels.cancel,
          discardLabel: labels.discard,
        },
        controller.signal,
        theme,
      );
      poll = setInterval(() => void refresh(), 1000);
      const response = await Promise.race([prompt.result, automatic.promise]);
      if (response.action === 'cancel' || response.action === 'aborted') return { action: 'cancel', dialogShown: true };
      if (response.action === 'retry') return { action: 'retry', dialogShown: true };
      discard = response.action === 'discard';
      completionShown = response.completionShown;
    }
    return {
      action: 'proceed',
      targets: presented,
      discard,
      dialogShown: prompt !== undefined,
      completionShown,
    };
  } finally {
    targetChanges?.removeEventListener('change', targetsChanged);
    clearInterval(poll);
    controller.abort();
    hideProgress();
    await Promise.all([...participants.values()].map(({ request }) => request));
  }
}

export async function prepareDocumentDeparture(options: DocumentDepartureOptions): Promise<boolean> {
  const { targets, reason, window, commit, getActiveContents, onProgress } = options;
  const operationId = randomUUID();
  // Participants stay stopped across retries and are released together, including
  // targets that disappeared from the selection while confirmation was open.
  const participants = new Map<WebContents, PreparedTarget>();
  const allowUnload = (event: Electron.Event) => event.preventDefault();
  let dialogShown = false;
  try {
    while (!window.isDestroyed()) {
      const confirmation = await confirmDocumentSave(options, operationId, participants);
      dialogShown ||= confirmation.dialogShown;
      if (confirmation.action === 'cancel') return false;
      if (confirmation.action === 'retry') continue;

      const { targets: approved, discard, completionShown } = confirmation;
      const isCurrent = () => matchesTargets(approved, targets());
      if (!isCurrent()) continue;
      const checked = await Promise.all(
        approved.map(async ({ target, generation }) => {
          const result = await requestDocumentSave(target, { operationId, phase: 'commit', discard, reason });
          return result.generation === generation && (discard || result.status === 'protected');
        }),
      );
      if (!isCurrent() || checked.some((valid) => !valid)) continue;

      for (const { target } of approved) target.webContents.on('will-prevent-unload', allowUnload);
      await commit();
      if (dialogShown && !completionShown && !discard && !window.isDestroyed()) {
        const host = getActiveContents?.();
        if (host && !host.isDestroyed()) {
          try {
            host.send('bridge:document-save-recovered');
          } catch {
            // The surviving UI can disappear during navigation/teardown.
          }
        }
      }
      return true;
    }
    return false;
  } finally {
    for (const { target } of participants.values()) {
      target.webContents.removeListener('will-prevent-unload', allowUnload);
      if (!target.webContents.isDestroyed()) target.webContents.send('document:save', { id: randomUUID(), operationId, phase: 'release' });
    }
    onProgress?.(
      [...participants.values()].map(({ target }) => target),
      false,
    );
  }
}
