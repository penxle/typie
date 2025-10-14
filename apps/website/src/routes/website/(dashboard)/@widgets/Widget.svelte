<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { getWidgetContext } from './widget-context.svelte';
  import type { Component, Snippet } from 'svelte';

  type Props = {
    title: string;
    children: Snippet;
    icon?: Component;
    headerActions?: Snippet;
    noPadding?: boolean;
    collapsed?: boolean;
  };

  let { title, children, icon, headerActions, noPadding = false, collapsed = false }: Props = $props();

  const widgetContext = getWidgetContext();
  const { editMode, palette } = $derived(widgetContext.env);
</script>

<div
  class={flex({
    flexDirection: 'column',
    borderRadius: '8px',
    backgroundColor: 'surface.default',
    borderWidth: '1px',
    borderColor: 'border.default',
    overflow: palette || editMode ? 'visible' : 'hidden',
  })}
>
  <div
    class={flex({
      alignItems: 'center',
      justifyContent: 'space-between',
      gap: '8px',
      height: '36px',
      borderBottomWidth: '1px',
      borderColor: 'border.subtle',
      backgroundColor: 'surface.subtle',
      borderTopLeftRadius: '8px',
      borderTopRightRadius: '8px',
      userSelect: 'none',
      cursor: palette ? 'inherit' : 'grab',
    })}
    data-drag-handle
  >
    <div class={flex({ alignItems: 'center', gap: '8px', paddingX: '12px', flexGrow: '1' })}>
      {#if icon}
        <Icon style={css.raw({ color: 'text.subtle' })} {icon} size={14} />
      {/if}
      <span
        class={css({
          fontSize: '13px',
          fontWeight: 'semibold',
          color: 'text.default',
          flexGrow: '1',
        })}
      >
        {title}
      </span>
    </div>
    {#if headerActions}
      <div class={flex({ alignItems: 'center', padding: '4px' })}>
        {@render headerActions()}
      </div>
    {/if}
  </div>

  {#if !collapsed}
    <div
      class={css({
        padding: noPadding ? '0' : '12px',
        cursor: palette ? 'inherit' : 'auto',
      })}
    >
      {@render children()}
    </div>
  {/if}
</div>
