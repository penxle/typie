<script lang="ts">
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

<div class={flex({ alignItems: 'center', flexShrink: '0', height: '48px' })}>
  {#if !app.preference.current.sidebarExpanded}
    <button
      class={center({ size: '48px', backgroundColor: 'white' })}
      onclick={() => (app.preference.current.sidebarExpanded = true)}
      type="button"
    >
      <Icon icon={PanelLeftOpenIcon} />
    </button>
  {/if}

  {#if children}
    <div
      class={css({
        flexGrow: '1',
        paddingLeft: app.preference.current.sidebarExpanded ? '16px' : '0',
        paddingRight: '8px',
      })}
    >
      {@render children()}
    </div>
  {/if}
</div>
