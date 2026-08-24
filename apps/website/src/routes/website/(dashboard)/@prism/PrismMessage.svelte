<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { parseMarkdown } from './lib/markdown.ts';
  import PrismMarkdown from './PrismMarkdown.svelte';
  import type { TranscriptMessage } from '@typie/prism';

  type Props = {
    message: TranscriptMessage;
  };

  let { message }: Props = $props();

  const blocks = $derived(message.role === 'assistant' && message.text !== null ? parseMarkdown(message.text) : []);
</script>

{#if message.role === 'user'}
  <div
    class={css({
      alignSelf: 'flex-end',
      maxWidth: '[86%]',
      paddingX: '12px',
      paddingY: '8px',
      borderRadius: '12px',
      borderBottomRightRadius: '2px',
      backgroundColor: 'surface.muted',
      fontSize: '14px',
      lineHeight: '[1.6]',
      whiteSpace: 'pre-wrap',
    })}
  >
    {message.text}
  </div>
{:else if message.role === 'assistant' && message.text}
  <PrismMarkdown {blocks} />
{/if}
