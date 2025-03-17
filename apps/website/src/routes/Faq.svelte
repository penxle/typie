<script lang="ts">
  import PlusIcon from '~icons/lucide/plus';
  import XIcon from '~icons/lucide/x';
  import { css } from '$styled-system/css';

  type Props = {
    question: string;
    answer: string;
  };

  const { question, answer }: Props = $props();

  let open = $state(false);
</script>

<button
  class={css({
    paddingY: '16px',
    fontSize: '16px',
    fontWeight: '[700]',
    borderYWidth: '1px',
    borderYColor: '[#E9E9E9]',
    textAlign: 'left',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
  })}
  onclick={() => (open = !open)}
  type="button"
>
  <span>
    {question}
  </span>
  {#if open}
    <XIcon class={css({ size: '16px' })} />
  {:else}
    <PlusIcon class={css({ size: '16px' })} />
  {/if}
</button>

{#if open}
  <div class={css({ paddingY: '16px', fontSize: '16px', borderYWidth: '1px', borderYColor: '[#E9E9E9]' })}>
    {answer}
  </div>
{/if}
