<script lang="ts">
  import ChevronsRightIcon from '~icons/lucide/chevrons-right';
  import { Icon } from '$lib/components';
  import { expandSidebar, sidebarPopoverVisible } from '$lib/stores';
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
</script>

<div>
  {#if !$expandSidebar}
    <div
      class={css({ width: $sidebarPopoverVisible ? '240px' : 'fit' })}
      onpointerenter={() => ($sidebarPopoverVisible = true)}
      onpointerleave={() => ($sidebarPopoverVisible = false)}
    >
      <button
        class={center({ size: '48px', backgroundColor: 'white' })}
        onclick={() => ($expandSidebar = true)}
        onpointerenter={() => ($sidebarPopoverVisible = true)}
        type="button"
      >
        <Icon icon={ChevronsRightIcon} />
      </button>
    </div>
  {/if}
</div>
