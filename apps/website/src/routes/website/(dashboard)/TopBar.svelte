<script lang="ts">
  import ChevronsRightIcon from '~icons/lucide/chevrons-right';
  import { Icon } from '$lib/components';
  import { getAppContext } from '$lib/context/app.svelte';
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';

  const app = getAppContext();
</script>

<div>
  {#if !app.preference.current.sidebarExpanded}
    <div
      class={css({ width: app.state.sidebarPopoverVisible ? '240px' : 'fit' })}
      onpointerenter={() => (app.state.sidebarPopoverVisible = true)}
      onpointerleave={() => (app.state.sidebarPopoverVisible = false)}
    >
      <button
        class={center({ size: '48px', backgroundColor: 'white' })}
        onclick={() => (app.preference.current.sidebarExpanded = true)}
        onpointerenter={() => (app.state.sidebarPopoverVisible = true)}
        type="button"
      >
        <Icon icon={ChevronsRightIcon} />
      </button>
    </div>
  {/if}
</div>
