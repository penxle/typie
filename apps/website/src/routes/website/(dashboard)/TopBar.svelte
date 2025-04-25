<script lang="ts">
  import { sineInOut } from 'svelte/easing';
  import { fade } from 'svelte/transition';
  import PanelLeftOpenIcon from '~icons/lucide/panel-left-open';
  import { Icon } from '$lib/components';
  import { getAppContext } from '$lib/context/app.svelte';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import type { Snippet } from 'svelte';

  type Props = {
    children?: Snippet;
  };

  let { children }: Props = $props();

  const app = getAppContext();
</script>

<div class={flex({ alignItems: 'center', flexShrink: '0', position: 'sticky', top: '0', paddingLeft: '16px', height: '36px' })}>
  {#if !app.preference.current.sidebarExpanded}
    <button
      class={center({ borderRadius: '6px', size: '24px', _hover: { backgroundColor: 'gray.100' } })}
      onclick={() => (app.preference.current.sidebarExpanded = true)}
      type="button"
      transition:fade={{ duration: 100, easing: sineInOut }}
    >
      <Icon icon={PanelLeftOpenIcon} size={16} />
    </button>
  {/if}

  {#if children}
    <div class={css({ flexGrow: '1' })}>
      {@render children()}
    </div>
  {/if}
</div>
