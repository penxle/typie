<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { untrack } from 'svelte';
  import FileIcon from '~icons/lucide/file';
  import { tooltip } from '../actions/tooltip.svelte';
  import { entityIconMap, getEntityIconColor } from '../constants/entity-icons';
  import Button from './Button.svelte';
  import DocumentSaveStatusIcon, { documentSaveStatusMessage } from './DocumentSaveStatusIcon.svelte';
  import Icon from './Icon.svelte';
  import Marquee from './Marquee.svelte';
  import type { DocumentSaveDocument } from '@typie/lib/document-save';

  let {
    title,
    description,
    documents,
    completed = false,
    onCompletedShown,
    continueLabel = '닫기',
    cancelLabel,
    discardLabel,
    retry = false,
    onAction,
  }: {
    title: string;
    description: string;
    documents: DocumentSaveDocument[];
    completed?: boolean;
    onCompletedShown?: () => void;
    continueLabel?: string;
    cancelLabel?: string;
    discardLabel?: string;
    retry?: boolean;
    onAction: (choice: 'cancel' | 'discard' | 'retry' | 'continue') => void;
  } = $props();

  const statusReference = (row: HTMLElement) => row.querySelector<HTMLElement>('[data-save-status]');
  const saved = $derived(completed);
  let countdown = $state(5);
  $effect(() => {
    if (!saved) return;
    countdown = 5;
    untrack(() => onCompletedShown?.());
    // Start only while the completed dialog is mounted. The operation owner
    // still revalidates protection and identity before acting on this choice.
    const timer = setInterval(() => {
      if (countdown > 1) countdown--;
      else {
        clearInterval(timer);
        onAction('continue');
      }
    }, 1000);
    return () => clearInterval(timer);
  });
</script>

<h2 class={css({ fontSize: '18px', fontWeight: 'semibold' })}>{title}</h2>
<p class={css({ fontSize: '14px', color: 'text.muted', whiteSpace: 'pre-line' })}>{description}</p>
<ul class={css({ display: 'flex', flexDirection: 'column', gap: '8px', maxHeight: '240px', overflowY: 'auto' })}>
  {#each documents as document (document.id)}
    <li
      class={css({ display: 'flex', flexDirection: 'column', gap: '2px', minWidth: '0', fontSize: '14px' })}
      use:tooltip={{
        message: documentSaveStatusMessage(document.status, document.protectedChanges),
        placement: 'top-end',
        getReference: statusReference,
      }}
    >
      <div class={css({ display: 'flex', alignItems: 'center', gap: '8px', minWidth: '0' })}>
        <span style:color={getEntityIconColor(document.iconColor ?? 'gray')}>
          <Icon icon={entityIconMap.get(document.icon ?? '') ?? FileIcon} size={16} />
        </span>
        <Marquee class={css({ minWidth: '0', flex: '1' })} fogSize={16} text={document.title} />
        <span class={css({ display: 'flex', flexShrink: '0' })} data-save-status>
          <DocumentSaveStatusIcon protectedChanges={document.protectedChanges} showTooltip={false} status={document.status} />
        </span>
      </div>
      {#if document.location}
        <span class={css({ marginLeft: '24px', color: 'text.muted', fontSize: '12px' })}>{document.location}</span>
      {/if}
    </li>
  {/each}
</ul>
<div class={css({ display: 'flex', flexWrap: 'wrap', justifyContent: 'flex-end', gap: '8px' })}>
  {#if cancelLabel}<Button onclick={() => onAction('cancel')} variant="secondary">{cancelLabel}</Button>{/if}
  {#if completed}
    <Button onclick={() => onAction('continue')}>
      {continueLabel}
      <span
        class={css({ marginLeft: '6px', opacity: '[0.6]', fontWeight: 'bold', fontVariantNumeric: 'tabular-nums' })}
        data-save-countdown
      >
        {countdown}
      </span>
    </Button>
  {:else}
    {#if discardLabel}<Button onclick={() => onAction('discard')} variant="danger">{discardLabel}</Button>{/if}
    {#if retry}<Button onclick={() => onAction('retry')} variant="secondary">다시 시도</Button>{/if}
  {/if}
</div>
