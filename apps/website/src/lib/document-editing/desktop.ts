import { Toast } from '@typie/ui/notification';
import { desktop } from '$lib/desktop';
import { cleanupBrowserPushForLogout } from '$lib/push';
import { DocumentPreparation } from './session';
import { documentEditing } from './state.svelte';
import type { DocumentSaveResult } from '@typie/lib/desktop';

export function watchDesktopDocumentSave(): () => void {
  const preparations = new Map<string, { preparation: DocumentPreparation; revoke?: () => void }>();
  const sessionGeneration = () =>
    JSON.stringify(documentEditing.sessions.map((session) => session.id).toSorted((a, b) => a.localeCompare(b)));
  const release = (id: string) => {
    const entry = preparations.get(id);
    if (!entry) return;
    preparations.delete(id);
    entry.revoke?.();
    entry.preparation.release();
    documentEditing.nativePreparations--;
  };
  const off = desktop?.onDocumentSave?.(async (request): Promise<DocumentSaveResult> => {
    if (request.phase === 'release') {
      release(request.operationId);
      return { status: 'protected', documents: [], generation: sessionGeneration() };
    }
    if (request.phase === 'capture') {
      await documentEditing.capture();
      const failures = documentEditing.sessions.filter((session) => !session.isProtected());
      return {
        generation: sessionGeneration(),
        status: failures.length > 0 ? 'failed' : 'protected',
        documents: failures.map((session) => ({
          id: session.documentId,
          sessionId: session.id,
          title: session.title() || '(제목 없음)',
          icon: session.entity?.()?.icon,
          iconColor: session.entity?.()?.iconColor,
        })),
      };
    }
    let entry = preparations.get(request.operationId);
    const isCurrent = () =>
      entry !== undefined &&
      preparations.get(request.operationId) === entry &&
      entry.preparation.sessions.length === documentEditing.sessions.length &&
      entry.preparation.sessions.every((session) => documentEditing.sessions.includes(session) && !session.disposed);
    // A pane may finish loading while native departure is waiting. Capture the
    // current session list instead of polling the obsolete preparation forever.
    if (request.phase === 'prepare' || (entry && request.phase === 'check' && !isCurrent())) {
      if (!entry || !isCurrent()) {
        const previous = entry;
        entry = { preparation: new DocumentPreparation(documentEditing.sessions) };
        preparations.set(request.operationId, entry);
        if (!previous) documentEditing.nativePreparations++;
        // Refresh only the checkpoint, not the native departure. Keep the tab
        // stopped and invalidate any in-flight request for the previous entry.
        previous?.revoke?.();
        previous?.preparation.release();
      }
      entry.revoke?.();
      entry.revoke = undefined;
      await entry.preparation.checkpoint();
    }
    const protectedChanges = !!entry && isCurrent() && entry.preparation.isProtected();
    if (entry && request.phase === 'commit' && isCurrent() && (protectedChanges || request.discard)) {
      if (request.reason === 'logout') await cleanupBrowserPushForLogout().catch(() => null);
      if (!isCurrent()) return { status: 'unknown', documents: [], generation: sessionGeneration() };
      entry.revoke ??= documentEditing.authorizeDeparture(entry.preparation.sessions);
      return { status: 'protected', documents: [], generation: sessionGeneration() };
    }
    const documents = documentEditing.sessions.map((session) => ({
      id: session.documentId,
      sessionId: session.id,
      title: session.title() || '(제목 없음)',
      icon: session.entity?.()?.icon,
      iconColor: session.entity?.()?.iconColor,
      status: entry?.preparation.getSessionStatus(session) ?? 'unknown',
      protectedChanges: session.isProtected(),
    }));
    let status: DocumentSaveResult['status'] = 'pending';
    if (protectedChanges) status = 'protected';
    else if (!isCurrent() || documents.some((document) => document.status === 'unknown')) status = 'unknown';
    else if (
      documents.some((document) => !document.protectedChanges && (document.status === 'failed' || document.status === 'sync-failed'))
    )
      status = 'failed';
    return {
      generation: sessionGeneration(),
      status,
      documents,
    };
  });
  const offRecovered = desktop?.on('document-save-recovered', () => {
    Toast.success('최근 변경사항을 안전하게 저장했어요.');
  });
  const offProgress = desktop?.on('document-save-progress', (visible) => {
    documentEditing.nativeProgress = visible;
  });
  return () => {
    offRecovered?.();
    offProgress?.();
    documentEditing.nativeProgress = false;
    off?.();
    for (const id of preparations.keys()) release(id);
  };
}
