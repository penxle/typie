<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { modifiedBadgeStyle } from './publish-styles';

  type Props = {
    documents: { id: string; title: string; state: 'UNPUBLISHED' | 'SCHEDULED' | 'PUBLISHED'; modified: boolean }[];
    failed: { documentId: string; message: string } | null;
  };

  let { documents, failed }: Props = $props();
</script>

<div class={flex({ flexDirection: 'column', gap: '8px', minHeight: '0' })}>
  <div class={css({ fontSize: '12px', fontWeight: 'semibold', color: 'text.hint' })}>글 {documents.length}개</div>

  <ul class={flex({ flexDirection: 'column', gap: '2px', minHeight: '0', overflowY: 'auto' })}>
    {#each documents as document (document.id)}
      <li class={flex({ alignItems: 'center', gap: '8px', minHeight: '32px', paddingX: '8px', borderRadius: '6px', fontSize: '13px' })}>
        {#if document.state === 'PUBLISHED'}
          <span class={css({ flexShrink: '0', size: '8px', borderRadius: 'full', backgroundColor: 'success.default' })}></span>
        {:else if document.state === 'SCHEDULED'}
          <span class={css({ flexShrink: '0', size: '8px', borderRadius: 'full', backgroundColor: 'warning.default' })}></span>
        {:else}
          <span
            class={css({ flexShrink: '0', size: '8px', borderWidth: '1px', borderColor: 'border.default', borderRadius: 'full' })}
          ></span>
        {/if}

        <span class={css({ flex: '1', minWidth: '0', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })}>
          {document.title}
        </span>

        {#if document.modified}
          <span class={css(modifiedBadgeStyle)}>수정됨</span>
        {/if}

        {#if failed?.documentId === document.id}
          <span
            class={css({ flexShrink: '0', size: '8px', borderRadius: 'full', backgroundColor: 'danger.default' })}
            aria-label={failed.message}
            role="img"
            use:tooltip={{ message: failed.message, placement: 'top' }}
          ></span>
        {/if}
      </li>
    {/each}
  </ul>
</div>
