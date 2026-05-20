<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
  import ArchiveIcon from '~icons/lucide/archive';
  import FileIcon from '~icons/lucide/file';
  import FileUpIcon from '~icons/lucide/file-up';
  import ImageIcon from '~icons/lucide/image';
  import { THEME_COLORS } from '$lib/editor/theme';
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

  const meta = $derived.by<{ icon: Component; label: string }>(() => {
    switch (element.data.type) {
      case 'image': {
        return { icon: ImageIcon, label: '이미지' };
      }
      case 'file': {
        return { icon: FileIcon, label: '파일' };
      }
      case 'embed': {
        return { icon: FileUpIcon, label: '임베드' };
      }
      case 'archived': {
        return { icon: ArchiveIcon, label: '보관된 블록' };
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

{#if element.data.type === 'image'}
  <ExternalImage {element} />
{:else}
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
    <div bind:this={containerEl} class={css({ width: 'full', minHeight: '48px' })}>
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
{/if}
