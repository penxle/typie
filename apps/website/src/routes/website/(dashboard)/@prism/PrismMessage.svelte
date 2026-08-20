<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { parseMarkdown } from './lib/markdown.ts';
  import PrismMarkdown from './PrismMarkdown.svelte';
  import type { TranscriptMessage } from './lib/conversation.ts';

  type Props = {
    message: TranscriptMessage;
  };

  let { message }: Props = $props();

  const blocks = $derived(message.role === 'assistant' && message.text !== null ? parseMarkdown(message.text) : []);

  const TOOL_LABEL = { executed: '완료', rejected: '거절됨', requested: '응답 대기', resolved: '응답 받음' } as const;

  const riseIn = css.raw({
    animation: '[rise-in 200ms cubic-bezier(0.23, 1, 0.32, 1) both]',
    _motionReduce: { animation: 'none' },
  });
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
{:else if message.role === 'tool'}
  {@const failed = message.ok === false}
  {@const pending = message.phase === 'requested'}
  <div
    class={css(riseIn, {
      display: 'flex',
      alignItems: 'center',
      gap: '6px',
      fontSize: '12px',
      color: failed ? 'text.danger' : pending ? 'text.subtle' : 'text.faint',
    })}
  >
    <div
      class={css({
        size: '5px',
        borderRadius: 'full',
        flexShrink: '0',
        backgroundColor: failed ? 'text.danger' : pending ? 'accent.brand.default' : 'border.strong',
        animation: pending ? 'pulse 1.6s ease-in-out infinite' : undefined,
      })}
    ></div>
    {message.name || '도구'} · {failed ? '실패' : TOOL_LABEL[message.phase]}
  </div>
{/if}
