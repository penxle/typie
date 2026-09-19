<script lang="ts">
  import { getDocumentSaveDialogText } from '@typie/lib/document-save';
  import { css } from '@typie/styled-system/css';
  import { Modal } from '@typie/ui/components';
  import DocumentSaveDialogContent from '@typie/ui/components/document-save-dialog-content';
  import { Toast } from '@typie/ui/notification';
  import { documentEditing } from './state.svelte';
  import type { DocumentSaveDocument } from '@typie/lib/document-save';
  import type { DocumentSaveReason } from './state.svelte';

  const actionLabels: Record<DocumentSaveReason, { proceed: string; discard?: string; cancel: string }> = {
    leave: { proceed: '닫기', discard: '저장하지 않고 닫기', cancel: '계속 편집' },
    reload: { proceed: '불러오기', discard: '변경사항 버리고 불러오기', cancel: '계속 편집' },
    logout: { proceed: '로그아웃', discard: '저장하지 않고 로그아웃', cancel: '로그아웃 취소' },
    login: { proceed: '로그인', discard: '저장하지 않고 로그인', cancel: '계속 편집' },
    save: { proceed: '확인', cancel: '계속 편집' },
    sync: { proceed: '확인', cancel: '계속 편집' },
  };

  const completed = $derived(documentEditing.completed);
  const operations = $derived(documentEditing.operations.filter((operation) => operation.phase !== 'finished'));
  const needsDecision = $derived(operations.some((operation) => operation.phase === 'blocked'));
  const open = $derived(needsDecision || completed);
  $effect(() => {
    if (open) {
      documentEditing.markDialogShown();
    }
  });
  const showProgress = $derived(!open && operations.some((operation) => operation.phase === 'saving' && operation.showProgress));
  $effect(() => {
    if (showProgress) return Toast.loading('저장 중…');
  });
  const reason = $derived(operations.find((operation) => !operation.canCancel)?.reason ?? operations[0]?.reason ?? 'leave');
  const labels = $derived(actionLabels[reason]);
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
