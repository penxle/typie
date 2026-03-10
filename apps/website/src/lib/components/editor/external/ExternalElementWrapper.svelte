<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getThemeContext } from '@typie/ui/context';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import { THEME_COLORS } from '$lib/editor/theme';
  import type { Snippet } from 'svelte';
  import type { ThemeVariant } from '$lib/editor/theme';
  import type { ExternalElement } from '$lib/editor/types';

  type Props = {
    el: ExternalElement;
    minHeight?: string;
    containerEl?: HTMLDivElement;
    children: Snippet;
  };

  let { el, minHeight = '48px', containerEl = $bindable(), children }: Props = $props();

  const { editor } = getEditorContext();
  const theme = getThemeContext();

  const selectionColor = $derived(
    THEME_COLORS[(theme.effectiveTheme === 'light' ? `light-${theme.lightVariant}` : `dark-${theme.darkVariant}`) as ThemeVariant]
      .selection,
  );

  let reportedHeight = $state<number>();
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    if (!containerEl) return;

    const observer = new ResizeObserver((entries) => {
      const height = entries[0].contentRect.height;
      if (height <= 0 || height === reportedHeight) return;

      if (debounceTimer) {
        clearTimeout(debounceTimer);
      }

      debounceTimer = setTimeout(() => {
        reportedHeight = height;
        editor.dispatch({
          type: 'setExternalElementHeight',
          nodeId: el.nodeId,
          height,
        });
        debounceTimer = null;
      }, 100);
    });

    observer.observe(containerEl);
    return () => {
      observer.disconnect();
      if (debounceTimer) {
        clearTimeout(debounceTimer);
      }
    };
  });
</script>

<div
  style:left="{el.bounds.x}px"
  style:top="{el.bounds.y}px"
  style:width="{el.bounds.width}px"
  style:min-height={minHeight}
  class={css({
    position: 'absolute',
    userSelect: 'none',
    display: 'flex',
    justifyContent: 'center',
    visibility: reportedHeight === undefined ? 'hidden' : 'visible',
  })}
  data-external-element
  data-node-id={el.nodeId}
>
  <div bind:this={containerEl} class={css({ width: 'full' })}>
    {@render children()}
  </div>

  {#if el.isSelected}
    <div style:background-color={selectionColor} class={css({ position: 'absolute', inset: '0', pointerEvents: 'none' })}></div>
  {/if}
</div>
