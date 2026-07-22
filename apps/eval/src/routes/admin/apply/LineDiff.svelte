<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { LineDiffEntry } from '$lib/domain/line-diff.ts';

  type Props = { entries: LineDiffEntry[] };
  const { entries }: Props = $props();
</script>

<div
  class={css({
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '8px',
    backgroundColor: 'surface.subtle',
    overflow: 'hidden',
  })}
>
  {#if entries.length === 0}
    <p class={css({ paddingX: '12px', paddingY: '10px', fontSize: '12px', color: 'text.faint' })}>변경 없음</p>
  {:else}
    <div class={css({ maxHeight: '260px', overflowY: 'auto', fontFamily: 'mono', fontSize: '12px', lineHeight: '[1.6]' })}>
      {#each entries as entry, i (i)}
        <div
          class={css({
            paddingX: '12px',
            whiteSpace: 'pre-wrap',
            wordBreak: 'break-all',
            backgroundColor: entry.type === 'add' ? 'accent.success.subtle' : entry.type === 'del' ? 'accent.danger.subtle' : 'transparent',
            color: entry.type === 'add' ? 'text.success' : entry.type === 'del' ? 'text.danger' : 'text.default',
            textDecoration: entry.type === 'del' ? 'line-through' : 'none',
          })}
        >
          {entry.type === 'add' ? '+ ' : entry.type === 'del' ? '- ' : '  '}{entry.line === '' ? ' ' : entry.line}
        </div>
      {/each}
    </div>
  {/if}
</div>
