<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { backInOut, sineInOut } from 'svelte/easing';
  import { fade, fly } from 'svelte/transition';
  import LightbulbIcon from '~icons/lucide/lightbulb';
  import XIcon from '~icons/lucide/x';
  import { Icon } from '$lib/components';
  import Shortcut from './Shortcut.svelte';
  import { store } from './store';
  import type { Tip } from './store';

  type Props = {
    tip: Tip;
  };

  let { tip }: Props = $props();

  const dismiss = () => store.update((v) => v.filter((t) => t.id !== tip.id));

  const tokens = $derived(tip.message.split(/(`[^`]+`)/g).filter(Boolean));
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
    backgroundColor: 'surface.default',
    boxShadow: 'small',
    pointerEvents: 'auto',
  })}
  in:fly={{ y: '10px', duration: 400, easing: backInOut }}
  out:fade={{ duration: 400, easing: sineInOut }}
>
  <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <Icon style={css.raw({ color: 'text.faint' })} icon={LightbulbIcon} size={12} />
      <div class={css({ fontSize: '13px', color: 'text.faint' })}>이용 팁</div>
    </div>

    <button onclick={dismiss} type="button">
      <Icon style={css.raw({ color: 'text.faint' })} icon={XIcon} size={12} />
    </button>
  </div>

  <div class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.subtle', verticalAlign: 'middle' })}>
    {#each tokens as token, index (index)}
      {#if token.startsWith('`') && token.endsWith('`')}
        <Shortcut shortcut={token.slice(1, -1)} />
      {:else}
        <span>{token}</span>
      {/if}
    {/each}
  </div>
</div>
