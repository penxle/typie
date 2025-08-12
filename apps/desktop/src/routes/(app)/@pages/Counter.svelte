<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import { tabState } from '../tabs.svelte';

  type Props = {
    tabId: string;
  };

  const { tabId }: Props = $props();

  let count = $state(0);

  const increment = () => {
    count++;
  };

  const decrement = () => {
    count--;
  };

  const reset = () => {
    count = 0;
  };

  $effect(() => {
    tabState.setTitle(tabId, `카운터 (${count})`);
  });
</script>

<main class={center({ height: 'full' })}>
  <div class={flex({ flexDirection: 'column', gap: '24px', alignItems: 'center' })}>
    <h1 class={css({ fontSize: '24px', fontWeight: 'bold' })}>카운터</h1>

    <div
      class={css({
        fontSize: '[48px]',
        fontWeight: 'bold',
        color: count > 0 ? 'green.600' : count < 0 ? 'red.600' : 'gray.900',
      })}
    >
      {count}
    </div>

    <div class={css({ display: 'flex', gap: '8px' })}>
      <Button onclick={decrement} variant="secondary">-1</Button>
      <Button onclick={reset} variant="secondary">리셋</Button>
      <Button onclick={increment}>+1</Button>
    </div>

    <div
      class={css({
        padding: '16px',
        backgroundColor: 'gray.50',
        borderRadius: '8px',
        fontSize: '13px',
        color: 'gray.600',
        maxWidth: '400px',
        textAlign: 'center',
      })}
    >
      탭 ID: {tabId}
    </div>
  </div>
</main>
