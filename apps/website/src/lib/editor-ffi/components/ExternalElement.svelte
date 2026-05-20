<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
  import ArchiveIcon from '~icons/lucide/archive';
  import FileIcon from '~icons/lucide/file';
  import FileUpIcon from '~icons/lucide/file-up';
  import { THEME_COLORS } from '$lib/editor/theme';
  import { getExternalElementPlaceholderLabel } from '$lib/editor-ffi/image';
  import { getEditorContext } from '../editor.svelte';
  import ExternalImage from './ExternalImage.svelte';
  import type { ExternalElement } from '@typie/editor-ffi/browser';
  import type { Component } from 'svelte';

  type Props = {
    element: ExternalElement;
  };

  let { element }: Props = $props();

  const SELECTION_FOCUSED_ALPHA = 77 / 255;
  const SELECTION_UNFOCUSED_ALPHA = 48 / 255;

  const ctx = getEditorContext();
  const theme = getThemeContext();

  let containerEl: HTMLDivElement | null = $state(null);
  let reportedHeight = $state<number>();
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  const themeVariant = $derived(theme.currentThemeVariant);
  const selectionColor = $derived(THEME_COLORS[themeVariant].selection);
  const selectionOpacity = $derived(ctx.editor?.focused ? SELECTION_FOCUSED_ALPHA : SELECTION_UNFOCUSED_ALPHA);

  const meta = $derived.by<{ icon: Component; label: string } | null>(() => {
    const label = getExternalElementPlaceholderLabel(element.data);
    if (!label) {
      return null;
    }

    switch (element.data.type) {
      case 'image': {
        return null;
      }
      case 'file': {
        return { icon: FileIcon, label };
      }
      case 'embed': {
        return { icon: FileUpIcon, label };
      }
      case 'archived': {
        return { icon: ArchiveIcon, label };
      }
    }
  });

  $effect(() => {
    const editor = ctx.editor;
    const node = containerEl;
    if (!editor || !node) return;

    const observer = new ResizeObserver((entries) => {
      const height = entries[0]?.contentRect.height ?? 0;
      if (height <= 0 || height === reportedHeight) return;

      if (debounceTimer) {
        clearTimeout(debounceTimer);
      }

      debounceTimer = setTimeout(() => {
        reportedHeight = height;
        editor.setExternalElementHeight(element.node_id, height);
        debounceTimer = null;
      }, 100);
    });

    observer.observe(node);
    return () => {
      observer.disconnect();
      if (debounceTimer) {
        clearTimeout(debounceTimer);
      }
    };
  });
</script>

<div
  style:left={`${element.bounds.x}px`}
  style:top={`${element.bounds.y}px`}
  style:width={`${element.bounds.width}px`}
  class={css({
    position: 'absolute',
    userSelect: 'none',
    display: 'flex',
    justifyContent: 'center',
    visibility: reportedHeight === undefined ? 'hidden' : 'visible',
  })}
  data-external-element
  data-node-id={element.node_id}
>
  <div bind:this={containerEl} class={css({ width: 'full' })}>
    {#if element.data.type === 'image'}
      <ExternalImage {element} />
    {:else if meta}
      <div
        class={flex({
          align: 'center',
          gap: '12px',
          width: 'full',
          minHeight: '48px',
          paddingX: '14px',
          paddingY: '12px',
          borderRadius: '4px',
          backgroundColor: 'surface.muted',
          color: 'text.disabled',
          fontSize: '14px',
        })}
      >
        <Icon class={css({ flexShrink: '0' })} icon={meta.icon} size={20} />
        <span
          class={css({
            minWidth: '0',
            overflow: 'hidden',
            whiteSpace: 'nowrap',
            textOverflow: 'ellipsis',
          })}
        >
          {meta.label}
        </span>
      </div>
    {/if}
  </div>

  {#if element.is_selected}
    <div
      style:background-color={selectionColor}
      style:opacity={selectionOpacity}
      class={css({
        position: 'absolute',
        inset: '0',
        pointerEvents: 'none',
      })}
    ></div>
  {/if}
</div>
