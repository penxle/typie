<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import PrismMessageActions from './PrismMessageActions.svelte';
  import type { TranscriptMessage } from '@typie/prism';

  type Props = {
    message: TranscriptMessage;
  };

  let { message }: Props = $props();
</script>

{#if message.role === 'user'}
  <div
    class={cx('group', flex({ alignSelf: 'flex-end', alignItems: 'flex-end', flexDirection: 'column', maxWidth: '[86%]' }))}
    data-message-actions-host
    data-message-role="user"
  >
    <div
      class={css({
        paddingX: '12px',
        paddingY: '8px',
        borderRadius: '12px',
        borderBottomRightRadius: '2px',
        backgroundColor: 'surface.inset',
        fontSize: '14px',
        lineHeight: '[1.6]',
        whiteSpace: 'pre-wrap',
      })}
    >
      {message.text}
    </div>
    <PrismMessageActions at={message.at} role="user" text={message.text} />
  </div>
{/if}
