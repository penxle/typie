<script lang="ts">
  import { backInOut, linear, sineInOut } from 'svelte/easing';
  import { Tween } from 'svelte/motion';
  import { fade, fly } from 'svelte/transition';
  import LightbulbIcon from '~icons/lucide/lightbulb';
  import XIcon from '~icons/lucide/x';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import Shortcut from './Shortcut.svelte';
  import { store } from './store';
  import type { Tip } from './store';

  type Props = {
    tip: Tip;
  };

  let { tip }: Props = $props();

  const dismiss = () => store.update((v) => v.filter((t) => t.id !== tip.id));
  const progress = new Tween(100, { duration: 10_000, easing: linear });

  const tokens = $derived(tip.message.split(/(`[^`]+`)/g).filter(Boolean));

  $effect(() => {
    if (progress.current === 0) {
      dismiss();
    }
  });
</script>

<div
  class={flex({
    flexDirection: 'column',
    gap: '8px',
    borderWidth: '1px',
    borderRadius: '8px',
    paddingX: '20px',
    paddingY: '12px',
    width: 'full',
    backgroundColor: 'white',
    boxShadow: 'xlarge',
    pointerEvents: 'auto',
  })}
  onintroend={() => (progress.target = 0)}
  in:fly={{ y: '10px', duration: 400, easing: backInOut }}
  out:fade={{ duration: 400, easing: sineInOut }}
>
  <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <Icon style={css.raw({ color: 'gray.500' })} icon={LightbulbIcon} size={12} />
      <div class={css({ fontSize: '13px', color: 'gray.500' })}>이용 팁</div>
    </div>

    <button onclick={dismiss} type="button">
      <Icon style={css.raw({ color: 'gray.500' })} icon={XIcon} size={12} />
    </button>
  </div>

  <div class={css({ fontSize: '14px', fontWeight: 'medium', color: 'gray.700', verticalAlign: 'middle' })}>
    {#each tokens as token, index (index)}
      {#if token.startsWith('`') && token.endsWith('`')}
        <Shortcut shortcut={token.slice(1, -1)} />
      {:else}
        <span>{token}</span>
      {/if}
    {/each}
  </div>
</div>
