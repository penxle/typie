<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import PlusIcon from '~icons/lucide/plus';
  import XIcon from '~icons/lucide/x';
  import Home from './@pages/Home.svelte';
  import { tabState } from './tabs.svelte';
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === 'w' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      e.stopPropagation();

      if (tabState.tabs.length === 1) {
        const window = getCurrentWindow();
        window.hide();
      }

      if (tabState.active) {
        tabState.remove(tabState.active.id);
      }
    }

    if (e.key === 't' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      e.stopPropagation();

      tabState.add(Home, {});
    }
  }}
/>

<div
  style:-webkit-app-region="drag"
  class={flex({
    justifyContent: 'space-between',
    alignItems: 'center',
    height: '40px',
    paddingY: '4px',
    paddingRight: '8px',
    backgroundColor: 'surface.subtle',
  })}
  data-tauri-drag-region
>
  <div
    style:-webkit-app-region="drag"
    class={flex({ flexGrow: '1', gap: '4px', height: 'full', overflowX: 'scroll', scrollbarWidth: 'none' })}
    data-tauri-drag-region
    role="tablist"
  >
    {#each tabState.tabs as tab (tab.id)}
      <div
        class={cx(
          'group',
          flex({
            flexGrow: '1',
            alignItems: 'center',
            gap: '8px',
            borderWidth: '[0.5px]',
            borderRadius: '4px',
            paddingX: '12px',
            minWidth: '80px',
            maxWidth: '200px',
            cursor: 'pointer',
            boxShadow: '[0 3px 6px -2px {colors.shadow.default/3}, 0 1px 1px {colors.shadow.default/5}]',
            transition: 'common',
            _selected: {
              backgroundColor: 'surface.default',
              _hover: {
                backgroundColor: 'surface.default',
              },
            },
            _hover: {
              backgroundColor: 'surface.muted',
            },
          }),
        )}
        aria-selected={tab.active}
        onclick={() => tabState.switch(tab.id)}
        onkeydown={null}
        role="tab"
        tabindex={tab.active ? 0 : -1}
      >
        <span
          class={css({
            flexGrow: '1',
            fontSize: '13px',
            fontWeight: 'medium',
            color: 'text.faint',
            lineClamp: '1',
            transition: 'common',
            _groupSelected: {
              color: 'text.default',
            },
          })}
        >
          {tab.title}
        </span>

        {#if tabState.tabs.length > 1}
          <button
            class={center({
              display: 'none',
              size: '16px',
              borderRadius: '4px',
              color: 'text.faint',
              transition: 'common',
              _groupHover: {
                display: 'flex',
              },
              _hover: {
                backgroundColor: 'surface.muted',
                color: 'text.default',
              },
            })}
            onclick={() => tabState.remove(tab.id)}
            type="button"
          >
            <Icon icon={XIcon} size={12} />
          </button>
        {/if}
      </div>
    {/each}
  </div>

  <button
    class={center({
      flexShrink: 0,
      size: '28px',
      borderRadius: '6px',
      color: 'text.subtle',
      transition: 'common',
      _hover: {
        backgroundColor: 'surface.muted',
        color: 'text.default',
      },
    })}
    onclick={() => tabState.add(Home, {})}
    type="button"
  >
    <Icon icon={PlusIcon} size={16} />
  </button>
</div>
