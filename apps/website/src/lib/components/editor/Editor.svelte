<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex, grid } from '@typie/styled-system/patterns';
  import { getThemeContext } from '@typie/ui/context';
  import { setEditor } from '$lib/editor/context';
  import { Editor } from '$lib/editor/editor.svelte';
  import { getEditorTheme } from '$lib/editor/theme';
  import View from './core/View.svelte';
  import HorizontalRuler from './ui/HorizontalRuler.svelte';
  import Loading from './ui/Loading.svelte';
  import Scrollbar from './ui/Scrollbar.svelte';
  import VerticalRuler from './ui/VerticalRuler.svelte';
  import type { Snippet } from 'svelte';
  import type { LayoutMode } from '$lib/editor/types';

  type Props = {
    unit?: 'px' | 'cm';
    rulerThickness?: number;
    contentPadding?: number;
    snapshot?: Uint8Array;
    editor?: Editor;
    onDocChanged?: () => void;
    onEditorReady?: (editor: Editor) => void;
    header?: Snippet;
  };

  let {
    unit = 'px',
    rulerThickness = 24,
    contentPadding = 48,
    snapshot,
    editor: externalEditor,
    onDocChanged,
    onEditorReady,
    header,
  }: Props = $props();

  const editor = externalEditor ?? new Editor();
  if (!externalEditor) {
    setEditor(editor);
  }

  const theme = getThemeContext();

  let width = $state(0);
  let scaleFactor = $state(1);
  let horizontalRulerEl: HTMLDivElement | null = $state(null);
  let verticalRulerEl: HTMLDivElement | null = $state(null);
  let scrollContainerEl: HTMLElement | null = $state(null);
  let initialized = $state(false);

  $effect(() => {
    editor.initialize({ theme: getEditorTheme(theme.effective), snapshot, onDocChanged });

    return () => {
      editor.destroy();
    };
  });

  $effect(() => {
    if (editor.layout.pageCount > 0) {
      initialized = true;
      onEditorReady?.(editor);
    }
  });

  $effect(() => {
    editor.dispatch({ type: 'resize', width, scaleFactor });
  });

  $effect(() => {
    const handleResize = () => {
      scaleFactor = window.devicePixelRatio * (window.visualViewport?.scale || 1);
    };

    window.visualViewport?.addEventListener('resize', handleResize);
    handleResize();
    return () => {
      window.visualViewport?.removeEventListener('resize', handleResize);
    };
  });

  const layoutMode = $derived<LayoutMode>(editor.layout.layoutMode);
  const pageWidth = $derived(editor.layout.pageWidth);
  const pageMargin = $derived(editor.layout.pageMargin);
  const pageHeights = $derived(editor.layout.pageHeights);
  const effectiveMargin = $derived(layoutMode.type === 'paginated' ? pageMargin : 0);
  const pageGap = $derived(layoutMode.type === 'paginated' ? 24 : 0);
</script>

<div class={flex({ direction: 'column', height: 'full', width: 'full' })}>
  {#if !initialized}
    <div class={center({ height: 'full', width: 'full', backgroundColor: 'surface.muted' })}>
      <Loading />
    </div>
  {:else}
    <div
      style:grid-template-columns={layoutMode.type === 'paginated' ? `${rulerThickness}px 1fr` : '1fr'}
      style:grid-template-rows={layoutMode.type === 'paginated' ? `${rulerThickness}px 1fr` : '1fr'}
      class={grid({ flex: '1', gap: '0', overflow: 'hidden' })}
    >
      {#if layoutMode.type === 'paginated'}
        <div
          class={css({
            borderRightWidth: '1px',
            borderBottomWidth: '1px',
            borderColor: 'border.strong',
            backgroundColor: 'surface.default',
          })}
        ></div>

        <div class={css({ overflow: 'hidden' })}>
          {#if pageWidth && pageMargin}
            <HorizontalRuler
              margin={pageMargin}
              padding={contentPadding}
              {pageWidth}
              thickness={rulerThickness}
              {unit}
              bind:ref={horizontalRulerEl}
            />
          {/if}
        </div>

        <div class={css({ overflow: 'hidden' })}>
          {#if pageHeights.length > 0}
            <VerticalRuler
              margin={effectiveMargin}
              padding={contentPadding}
              {pageGap}
              {pageHeights}
              thickness={rulerThickness}
              {unit}
              bind:ref={verticalRulerEl}
            />
          {/if}
        </div>
      {/if}

      <div
        bind:this={scrollContainerEl}
        class={css({
          overflow: 'auto',
          scrollbarWidth: 'none',
          '&::-webkit-scrollbar': { display: 'none' },
          ...(layoutMode.type === 'continuous' && { overflowX: 'hidden' }),
        })}
        {@attach (el) => {
          const observer = new ResizeObserver((entries) => {
            const entry = entries[0];
            if (entry) {
              width = Math.max(0, Math.round(entry.contentRect.width) - contentPadding * 2);
            }
          });

          observer.observe(el);
          return () => observer.disconnect();
        }}
        onscroll={(e) => {
          const target = e.currentTarget;
          if (horizontalRulerEl) {
            horizontalRulerEl.style.transform = `translateX(-${target.scrollLeft}px)`;
          }
          if (verticalRulerEl) {
            verticalRulerEl.style.transform = `translateY(-${target.scrollTop}px)`;
          }
        }}
      >
        <div class={css({ position: 'relative', height: 'full', minWidth: 'max' })}>
          {#if header}
            {@render header()}
          {/if}
          <View />
        </div>
      </div>
      <Scrollbar scrollContainer={scrollContainerEl} />
    </div>
  {/if}
</div>
