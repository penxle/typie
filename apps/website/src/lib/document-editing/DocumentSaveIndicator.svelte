<script lang="ts">
  import DocumentSaveStatusIcon from '@typie/ui/components/document-save-status-icon';
  import { onDestroy, untrack } from 'svelte';
  import type { DocumentSaveState } from '@typie/lib/document-save';
  import type { DocumentSaveStatus } from './session';

  let {
    status: currentStatus,
    protectedChanges,
    unprotectedSince,
    unconfirmedSince,
    inspectedStatus = null,
    onShowDetails,
  }: {
    status: DocumentSaveStatus | null;
    protectedChanges: boolean;
    unprotectedSince: number | null;
    unconfirmedSince: number | null;
    inspectedStatus?: DocumentSaveState | null;
    onShowDetails: () => void;
  } = $props();
  let visible = $state<{ status: DocumentSaveState; protectedChanges: boolean } | null>(null);
  let initialized = false;
  let hadFeedback = false;
  let completionTimer: ReturnType<typeof setTimeout> | undefined;
  const displayed = $derived(
    inspectedStatus !== null && inspectedStatus !== 'synced' ? { status: inspectedStatus, protectedChanges } : visible,
  );

  onDestroy(() => clearTimeout(completionTimer));

  $effect(() => {
    const status = currentStatus;
    const protectedNow = protectedChanges;
    const since = unprotectedSince;
    const awaitingServerSince = unconfirmedSince;
    const inspected = inspectedStatus;
    let timer: ReturnType<typeof setTimeout> | undefined;
    // The displayed snapshot must not become an input to its own effect.
    untrack(() => {
      const initialState = !initialized;
      initialized = status !== null;
      const show = (next: DocumentSaveState, protectedChanges: boolean) => {
        clearTimeout(completionTimer);
        completionTimer = undefined;
        visible = { status: next, protectedChanges };
        hadFeedback = true;
      };
      if (inspected !== null && inspected !== 'synced') hadFeedback = true;
      if (status === null) {
        clearTimeout(completionTimer);
        completionTimer = undefined;
        visible = null;
        hadFeedback = false;
      } else if (status === 'synced' || inspected === 'synced') {
        if (hadFeedback) {
          show('synced', true);
          hadFeedback = false;
          completionTimer = setTimeout(() => {
            completionTimer = undefined;
            visible = null;
          }, 2000);
        }
      } else if (status === 'failed') {
        show('failed', protectedNow);
      } else if (protectedNow && status === 'sync-failed') {
        show('sync-failed', true);
      } else if (protectedNow) {
        // Restored local changes are already safe; only new edits use the quiet period.
        if (!completionTimer && (hadFeedback || initialState)) show('protected', true);
        else if (awaitingServerSince !== null) {
          timer = setTimeout(() => show('protected', true), Math.max(0, awaitingServerSince + 5000 - Date.now()));
        }
      } else if (since !== null) {
        // Keep the last confirmed feedback through fast writes. The oldest
        // unpreserved change determines the delay, not the most recent input.
        timer = setTimeout(
          () => {
            show('pending', false);
          },
          Math.max(0, since + 5000 - Date.now()),
        );
      }
    });
    return () => clearTimeout(timer);
  });
</script>

<DocumentSaveStatusIcon {onShowDetails} protectedChanges={displayed?.protectedChanges ?? false} status={displayed?.status ?? null} />
