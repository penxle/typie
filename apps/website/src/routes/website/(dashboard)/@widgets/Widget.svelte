<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import MinusIcon from '~icons/lucide/minus';
  import PlusIcon from '~icons/lucide/plus';
  import { getWidgetContext } from './widget-context.svelte';
  import type { Component, Snippet } from 'svelte';
  import type { WidgetType } from './widget-context.svelte';

  type Props = {
    widgetId: string;
    title: string;
    children: Snippet;
    disabled?: boolean;
    widgetType?: WidgetType;
    editMode?: boolean;
    icon?: Component;
    headerActions?: Snippet;
    noPadding?: boolean;
    collapsed?: boolean;
  };

  let {
    widgetId,
    title,
    children,
    disabled = false,
    widgetType,
    editMode: editModeProp,
    icon,
    headerActions,
    noPadding = false,
    collapsed = false,
  }: Props = $props();

  const widgetContext = getWidgetContext();
  const { editMode: editModeContext, palette } = $derived(widgetContext.env);
  const editMode = $derived(editModeProp ?? editModeContext);

  const handleAddWidget = () => {
    if (widgetType) {
      widgetContext.createWidget?.(widgetType, 0);
    }
  };
</script>

<div
  class={cx(
    'group',
    flex({
      position: 'relative',
      flexDirection: 'column',
      borderRadius: '8px',
      backgroundColor: 'surface.default',
      borderWidth: '1px',
      borderColor: 'border.default',
      boxShadow: 'medium',
      overflow: palette || editMode ? 'visible' : 'hidden',
      opacity: widgetId.startsWith('temp-') ? '50' : '100',
      pointerEvents: widgetId.startsWith('temp-') ? 'none' : 'auto',
    }),
    palette &&
      !disabled &&
      css({
        userSelect: 'none',
      }),
  )}
  data-widget={widgetId}
>
  {#if palette && !disabled}
    <button
      class={center({
        position: 'absolute',
        top: '0',
        left: '0',
        size: '28px',
        borderRadius: 'full',
        backgroundColor: 'surface.default',
        borderWidth: '1px',
        borderColor: 'border.default',
        color: 'text.subtle',
        opacity: '0',
        transitionProperty: '[opacity]',
        transitionDuration: '200ms',
        transform: 'translate(-8px, -8px)',
        _groupHover: { opacity: '100' },
        _hover: { backgroundColor: 'surface.subtle', color: 'text.default' },
        zIndex: '10',
        cursor: 'pointer',
      })}
      onclick={(e) => {
        e.preventDefault();
        e.stopPropagation();
        handleAddWidget();
      }}
      onpointerdown={(e) => {
        e.stopPropagation();
      }}
      type="button"
    >
      <Icon icon={PlusIcon} size={16} />
    </button>
  {:else if editMode}
    <button
      class={center({
        position: 'absolute',
        top: '0',
        left: '0',
        size: '24px',
        borderRadius: 'full',
        backgroundColor: 'surface.default',
        borderWidth: '1px',
        borderColor: 'border.default',
        color: 'text.subtle',
        transitionProperty: '[opacity]',
        transitionDuration: '200ms',
        transform: 'translate(-8px, -8px)',
        _hover: { backgroundColor: 'surface.subtle', color: 'text.default' },
        zIndex: '10',
        cursor: 'pointer',
      })}
      onclick={(e) => {
        e.preventDefault();
        e.stopPropagation();
        widgetContext.deleteWidget?.(widgetId);
      }}
      onpointerdown={(e) => {
        e.stopPropagation();
      }}
      type="button"
    >
      <Icon icon={MinusIcon} size={14} />
    </button>
  {/if}

  <div
    class={flex({
      alignItems: 'center',
      gap: '8px',
      paddingX: '12px',
      paddingY: '8px',
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
    {#if headerActions}
      {@render headerActions()}
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
