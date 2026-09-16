<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { createFloatingActions, tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import ToolbarPanelSurface from './ToolbarPanelSurface.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { TooltipParameter } from '@typie/ui/actions';
  import type { Snippet } from 'svelte';

  type Props = {
    style?: SystemStyleObject;
    label: string;
    keys?: TooltipParameter['keys'];
    active?: boolean;
    disabled?: boolean;
    chevron?: boolean;
    anchor: Snippet;
    panel: Snippet<[{ close: () => void }]>;
    onclose?: () => void;
  };

  let { style, label, keys, active = false, disabled = false, chevron = false, anchor, panel, onclose }: Props = $props();

  const ctx = getEditorContext();

  let opened = $state(false);

  const { anchor: anchorAction, floating } = createFloatingActions({
    placement: 'bottom-start',
    offset: 6,
    onClickOutside: () => {
      opened = false;
    },
  });

  const close = () => {
    opened = false;
  };

  let wasOpened = false;

  $effect(() => {
    const now = opened;
    if (wasOpened && !now) onclose?.();
    wasOpened = now;
  });

  $effect(() => {
    if (disabled) opened = false;
  });

  $effect(() => {
    if (!opened) return;
    return ctx.editor?.retainFocus();
  });

  $effect(() => {
    if (!opened) return;
    return pushEscapeHandler(() => {
      close();
      ctx.editor?.focus();
      return true;
    });
  });
</script>

<button
  class={css(
    {
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'center',
      gap: '2px',
      flexShrink: '0',
      borderRadius: '4px',
      paddingX: chevron ? '4px' : '0',
      width: chevron ? 'fit' : '24px',
      height: '24px',
      textAlign: 'left',
      color: active ? 'accent.default' : 'text.muted',
      backgroundColor: active ? 'surface.active' : 'transparent',
      transition: 'common',
      _enabled: {
        _hover: { color: 'text.default', backgroundColor: 'surface.hover' },
        _expanded: { color: 'accent.default', backgroundColor: 'surface.active' },
      },
      _disabled: { opacity: '40' },
    },
    style,
  )}
  aria-expanded={opened}
  aria-haspopup="listbox"
  aria-label={label}
  {disabled}
  onclick={() => (opened = !opened)}
  onpointerdown={(e) => e.preventDefault()}
  type="button"
  use:anchorAction
  use:tooltip={{ message: opened ? null : label, keys, arrow: false }}
>
  {@render anchor()}

  {#if chevron}
    <Icon
      style={css.raw({
        color: 'text.muted',
        transform: opened ? 'rotate(-180deg)' : 'rotate(0deg)',
        transitionDuration: '150ms',
        '& *': { strokeWidth: '[1.5px]' },
      })}
      icon={ChevronDownIcon}
      size={14}
    />
  {/if}
</button>

{#if opened}
  <div class={css({ zIndex: 'overEditor' })} use:floating>
    <ToolbarPanelSurface>
      {@render panel({ close })}
    </ToolbarPanelSurface>
  </div>
{/if}
