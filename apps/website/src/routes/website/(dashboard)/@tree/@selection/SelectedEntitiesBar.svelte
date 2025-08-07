<script lang="ts">
  import { fade } from 'svelte/transition';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import XIcon from '~icons/lucide/x';
  import { tooltip } from '$lib/actions';
  import { Icon, Menu } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import { getTreeContext } from '../state.svelte';
  import MultiEntitiesMenu from './MultiEntitiesMenu.svelte';

  const treeState = getTreeContext();
</script>

<div
  class={css({
    position: 'sticky',
    bottom: '0',
    marginTop: '32px',
    left: '16px',
    right: '16px',
    display: 'flex',
    alignSelf: 'center',
    alignItems: 'center',
    gap: '8px',
    paddingLeft: '16px',
    paddingY: '6px',
    paddingRight: '8px',
    backgroundColor: 'surface.subtle',
    borderRadius: '8px',
    boxShadow: 'medium',
    border: '1px solid',
    borderColor: 'border.default',
  })}
  transition:fade={{ duration: 100 }}
>
  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <div class={flex({ fontSize: '14px', fontWeight: 'medium', color: 'text.faint' })}>
      <span class={css({ color: 'text.subtle' })}>{treeState.selectedEntityIds.size}</span>
      <span>개 선택됨</span>
    </div>
    <button
      class={center({
        size: '24px',
        borderRadius: '4px',
        color: 'text.faint',
        transition: 'common',
        _hover: {
          backgroundColor: 'surface.muted',
        },
      })}
      onclick={() => {
        treeState.selectedEntityIds.clear();
        treeState.lastSelectedEntityId = undefined;
      }}
      type="button"
      use:tooltip={{ message: '선택 해제' }}
    >
      <Icon style={css.raw({ color: 'text.faint' })} icon={XIcon} size={16} />
    </button>
  </div>
  <div class={css({ width: '1px', height: '24px', backgroundColor: 'border.default' })}></div>
  <Menu placement="bottom-start">
    {#snippet button({ open })}
      <div
        class={center({
          borderRadius: '4px',
          size: '24px',
          color: 'text.faint',
          transition: 'common',
          _hover: { backgroundColor: 'surface.muted' },
          _pressed: { backgroundColor: 'surface.muted' },
        })}
        aria-pressed={open}
      >
        <Icon style={css.raw({ color: 'text.faint' })} icon={EllipsisIcon} size={16} />
      </div>
    {/snippet}

    <MultiEntitiesMenu {treeState} />
  </Menu>
</div>
