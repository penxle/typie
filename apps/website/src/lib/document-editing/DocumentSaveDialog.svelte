<script lang="ts">
  import { DOCUMENT_SAVE_ACTION_LABELS, getDocumentSaveDialogText } from '@typie/lib/document-save';
  import { css } from '@typie/styled-system/css';
  import { Modal } from '@typie/ui/components';
  import DocumentSaveDialogContent from '@typie/ui/components/document-save-dialog-content';
  import { Toast } from '@typie/ui/notification';
  import { documentEditing } from './state.svelte';
  import type { DocumentSaveDocument } from '@typie/lib/document-save';

  const completed = $derived(documentEditing.completed);
  const operations = $derived(documentEditing.operations.filter((operation) => operation.phase !== 'finished'));
  const needsDecision = $derived(operations.some((operation) => operation.phase === 'blocked'));
  const open = $derived(documentEditing.nativePreparations === 0 && (needsDecision || completed));
  $effect(() => {
    if (open) {
      documentEditing.markDialogShown();
    }
  });
  const showProgress = $derived(
    !open && (documentEditing.nativeProgress || operations.some((operation) => operation.phase === 'saving' && operation.showProgress)),
  );
  $effect(() => {
    if (showProgress) return Toast.loading('저장 중…');
  });
  const reason = $derived(operations.find((operation) => !operation.canCancel)?.reason ?? operations[0]?.reason ?? 'leave');
  const labels = $derived(DOCUMENT_SAVE_ACTION_LABELS[reason]);
  const canCancel = $derived(operations.every((operation) => operation.canCancel));
  const saveOnly = $derived(reason === 'save' || reason === 'sync');
  const cancel = () => documentEditing.cancel();
  const unprotectedSessions = $derived(operations.flatMap((operation) => operation.unprotectedSessions));
  const pending = $derived(
    unprotectedSessions.length > 0 &&
      operations.every((operation) =>
        operation.unprotectedSessions.every((session) => operation.preparation.getSessionStatus(session) === 'pending'),
      ),
  );
  const protectedChanges = $derived(unprotectedSessions.length > 0 && unprotectedSessions.every((session) => session.isProtected()));
  const documents = $derived.by(() => {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- rebuilt as one derived snapshot
    const rows = new Map<string, DocumentSaveDocument>();
    for (const operation of operations) {
      for (const session of operation.displayedSessions) {
        // One live session can participate in both a departure and server-save
        // details. Prefer the unsatisfied requirement without merging other panes.
        const status = operation.preparation.getSessionStatus(session);
        if (operation.preparation.isSessionProtected(session) && rows.has(session.id)) continue;
        rows.set(session.id, {
          id: session.id,
          title: session.title() || '(제목 없음)',
          icon: session.entity?.()?.icon,
          iconColor: session.entity?.()?.iconColor,
          location: '',
          status,
          protectedChanges: session.isProtected(),
        });
      }
    }
    return [...rows.values()];
  });
  const text = $derived(
    getDocumentSaveDialogText({
      reason,
      pending,
      unknown: documents.some((document) => document.status === 'unknown'),
      protectedChanges,
      completed,
    }),
  );
</script>

<Modal style={css.raw({ maxWidth: '480px', padding: '24px', gap: '16px' })} closable={canCancel} onclose={cancel} {open}>
  {#if open}
    <DocumentSaveDialogContent
      cancelLabel={canCancel ? labels.cancel : undefined}
      {completed}
      continueLabel={labels.proceed}
      description={text.description}
      discardLabel={!saveOnly && needsDecision ? labels.discard : undefined}
      {documents}
      onAction={(choice) => {
        if (choice === 'cancel') cancel();
        else if (choice === 'retry') documentEditing.retry();
        else if (choice === 'discard') documentEditing.discard();
        else documentEditing.completeRecovery();
      }}
      retry={needsDecision && (!canCancel || saveOnly)}
      title={text.title}
    />
  {/if}
</Modal>
