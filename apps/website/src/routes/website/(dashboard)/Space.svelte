<script lang="ts">
  import { fly } from 'svelte/transition';
  import LibraryBigIcon from '~icons/lucide/library-big';
  import { fragment, graphql } from '$graphql';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import EntityTree from './@tree/EntityTree.svelte';
  import SidebarButton from './SidebarButton.svelte';
  import type { DashboardLayout_Space_site } from '$graphql';

  type Props = {
    $site: DashboardLayout_Space_site;
  };

  let { $site: _site }: Props = $props();

  const site = fragment(
    _site,
    graphql(`
      fragment DashboardLayout_Space_site on Site {
        id

        ...DashboardLayout_EntityTree_site
      }
    `),
  );

  let open = $state(false);
</script>

<SidebarButton active={open} icon={LibraryBigIcon} label="내 스페이스" onclick={() => (open = true)} />

{#if open}
  <div class={css({ position: 'fixed', inset: '0', zIndex: '50' })}>
    <div class={css({ position: 'absolute', inset: '0' })} onclick={() => (open = false)} role="none"></div>

    <div
      class={flex({
        position: 'absolute',
        left: '64px',
        insetY: '0',
        flexDirection: 'column',
        borderRightWidth: '1px',
        borderColor: 'gray.100',
        borderRightRadius: '4px',
        width: '350px',
        backgroundColor: 'white',
        boxShadow: 'small',
        overflowY: 'auto',
        zIndex: '1',
      })}
      transition:fly={{ x: -5, duration: 100 }}
    >
      <div
        class={flex({
          position: 'sticky',
          top: '0',
          flexShrink: '0',
          alignItems: 'center',
          gap: '4px',
          borderBottomWidth: '1px',
          paddingX: '16px',
          paddingY: '12px',
          backgroundColor: 'white',
          zIndex: '1',
        })}
      >
        <span class={css({ fontSize: '14px', fontWeight: 'bold' })}>내 스페이스</span>
      </div>

      <div class={css({ paddingX: '16px' })}>
        <EntityTree {$site} />
      </div>
    </div>
  </div>
{/if}
