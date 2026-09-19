<script lang="ts">
  import DocumentSaveStatusIcon from '@typie/ui/components/document-save-status-icon';
  import { onDestroy } from 'svelte';
  import type { DocumentSaveStatus } from './session';

  let {
    status: currentStatus,
    protectedChanges,
    onShowDetails,
  }: { status: DocumentSaveStatus | null; protectedChanges: boolean; onShowDetails: () => void } = $props();
  // Protection-only updates keep the current storage phase and its feedback timers.
  const status = $derived(currentStatus);
  let visible = $state<Exclude<DocumentSaveStatus, 'idle'> | null>(null);
  let hadFeedback = false;
  let completionTimer: ReturnType<typeof setTimeout> | undefined;

  onDestroy(() => clearTimeout(completionTimer));

  $effect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    if (status === null || status === 'failed' || status === 'sync-failed') {
      clearTimeout(completionTimer);
      completionTimer = undefined;
      visible = status;
      hadFeedback = status !== null;
    } else if (status === 'synced' && hadFeedback) {
      hadFeedback = false;
      visible = 'synced';
      completionTimer = setTimeout(() => {
        completionTimer = undefined;
        visible = null;
      }, 2000);
    } else {
      // New edits do not shorten or restart feedback for the preceding completed save.
      if (completionTimer === undefined) visible = null;
      if (status === 'pending') {
        timer = setTimeout(() => {
          visible = 'pending';
          hadFeedback = true;
        }, 5000);
      }
    }
    return () => clearTimeout(timer);
  });
</script>

<DocumentSaveStatusIcon {onShowDetails} {protectedChanges} status={visible} />
