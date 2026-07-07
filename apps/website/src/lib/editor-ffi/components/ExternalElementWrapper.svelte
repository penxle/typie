<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getThemeContext } from '@typie/ui/context';
  import { THEME_COLORS } from '$lib/editor/theme';
  import { getEditorContext } from '../editor.svelte';
  import type { ExternalElement } from '@typie/editor-ffi/browser';
  import type { Snippet } from 'svelte';

  type Props = {
    element: ExternalElement;
    minHeight?: string;
    containerEl?: HTMLDivElement;
    children: Snippet;
  };

  let { element, minHeight = '48px', containerEl = $bindable(), children }: Props = $props();

  const SELECTION_FOCUSED_ALPHA = 77 / 255;
  const SELECTION_UNFOCUSED_ALPHA = 48 / 255;

  const ctx = getEditorContext();
  const theme = getThemeContext();

  let reportedHeight = $state<number>();

  const themeVariant = $derived(theme.currentThemeVariant);
  const selectionColor = $derived(THEME_COLORS[themeVariant].selection);
  const selectionOpacity = $derived(ctx.editor?.focused ? SELECTION_FOCUSED_ALPHA : SELECTION_UNFOCUSED_ALPHA);

  $effect(() => {
    const editor = ctx.editor;
    const node = containerEl;
    if (!editor || !node) return;

    const observer = new ResizeObserver((entries) => {
      const height = entries[0]?.contentRect.height ?? 0;
      if (!Number.isFinite(height) || height <= 0 || height === reportedHeight) return;

      reportedHeight = height;
      editor.setExternalElementHeight(element.node, height);
    });

    observer.observe(node);

    return () => {
      observer.disconnect();
    };
  });
</script>

<div
  style:left={`${element.bounds.x}px`}
  style:top={`${element.bounds.y}px`}
  style:width={`${element.bounds.width}px`}
  style:min-height={minHeight}
  class={css({
    position: 'absolute',
    userSelect: 'none',
    display: 'flex',
    justifyContent: 'center',
    visibility: reportedHeight === undefined ? 'hidden' : 'visible',
  })}
  data-external-element
  data-node-id={element.node}
>
  <div bind:this={containerEl} class={css({ width: 'full' })}>
    {@render children()}
  </div>

  {#if element.is_selected}
    <div
      style:background-color={selectionColor}
      style:opacity={selectionOpacity}
      class={css({ position: 'absolute', inset: '0', pointerEvents: 'none' })}
    ></div>
  {/if}
</div>
