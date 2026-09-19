<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { focusTrap } from '@typie/ui/actions/focus-trap';
  import DocumentSaveDialogContent from '@typie/ui/components/document-save-dialog-content';
  import { onMount, tick } from 'svelte';
  import type { DocumentSaveDialogAction, DocumentSaveDialogData } from '@typie/lib/desktop';

  let data = $state.raw<DocumentSaveDialogData>();
  const dialogId = $derived(data?.id);
  let content: HTMLElement;
  const respond = (action: DocumentSaveDialogAction) => {
    if (data) window.documentSaveDialog.respond(data.id, action);
  };
  const reportReady = () => {
    if (data) window.documentSaveDialog.ready(data.id, content.getBoundingClientRect().height, data.completed ?? false);
  };
  onMount(() => window.documentSaveDialog.subscribe((snapshot) => (data = snapshot)));
  $effect(() => {
    const id = dialogId;
    if (!id) return;
    let active = true;
    const ready = () => {
      if (active) reportReady();
    };
    void tick().then(ready);
    const heartbeat = setInterval(ready, 1000);
    return () => {
      active = false;
      clearInterval(heartbeat);
    };
  });
  $effect(() => {
    if (!dialogId) return;
    const observer = new ResizeObserver(() => window.documentSaveDialog.resize(content.getBoundingClientRect().height));
    observer.observe(content);
    return () => observer.disconnect();
  });
</script>

<svelte:window
  onkeydown={(event) => {
    if (data && event.key === 'Escape' && !data.required) {
      event.preventDefault();
      respond('cancel');
    }
  }}
/>
<div
  bind:this={content}
  class={css({ display: 'flex', flexDirection: 'column', padding: '24px', gap: '16px', outline: 'none' })}
  aria-label={data?.title ?? '저장 확인'}
  aria-modal="true"
  data-focus-trap
  role="dialog"
  tabindex="-1"
  use:focusTrap={{ initialFocus: () => content, fallbackFocus: () => content, escapeDeactivates: false, returnFocusOnDeactivate: false }}
>
  {#if data}
    <DocumentSaveDialogContent
      cancelLabel={data.required ? undefined : data.cancelLabel}
      completed={data.completed}
      continueLabel={data.continueLabel}
      description={data.description}
      discardLabel={data.discardLabel}
      documents={data.documents}
      onAction={respond}
      onCompletedShown={reportReady}
      retry={data.required}
      title={data.title}
    />
  {/if}
</div>
