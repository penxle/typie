<script lang="ts">
  import { flex, grid } from '@typie/styled-system/patterns';
  import { getThemeContext } from '@typie/ui/context';
  import { handleDragScroll } from '@typie/ui/utils';
  import { tick, untrack } from 'svelte';
  import { CONTINUOUS_MIN_WIDTH, CONTINUOUS_PAGE_MARGIN, CONTINUOUS_VIEW_PADDING, PAGINATED_VIEW_PADDING } from '$lib/editor/constants';
  import { getEditorTheme } from '$lib/editor/theme';
  import View from './core/View.svelte';
  import EditorZoom from './ui/EditorZoom.svelte';
  import RulerFrame from './ui/RulerFrame.svelte';
  import Scrollbar from './ui/Scrollbar.svelte';
  import type { Snippet } from 'svelte';
  import type { Editor } from '$lib/editor/editor.svelte';
  import type { FontFamily } from '$lib/editor/fonts';
  import type { Position } from '$lib/editor/types';

  const PAGINATED_HEADER_FOOTER_MIN_WIDTH = 320;

  type Props = {
    unit?: 'px' | 'cm';
    rulerThickness?: number;
    snapshot?: Uint8Array;
    readOnly?: boolean;
    resizing?: boolean;
    useWindowScroll?: boolean;
    active?: boolean;
    editor: Editor;
    fontFamilies: readonly FontFamily[];
    onDocChanged?: () => void;
    onSelectionChanged?: (anchor: Position, head: Position) => void;
    onExitedDocumentStart?: () => void;
    onEditorReady?: (editor: Editor) => void;
    header?: Snippet;
    footer?: Snippet;
    children?: Snippet;
  };

  let {
    unit = 'px',
    rulerThickness = 24,
    snapshot,
    readOnly = false,
    resizing = false,
    useWindowScroll = false,
    active = true,
    editor,
    fontFamilies,
    onDocChanged,
    onSelectionChanged,
    onExitedDocumentStart,
    onEditorReady,
    header,
    footer,
    children,
  }: Props = $props();

  const theme = getThemeContext();

  let containerClientWidth = $state(0);
  let containerClientHeight = $state(0);
  let scaleFactor = $state(1);
  let zoomRenderScale = $state(1);
  let headerHeight = $state(0);
  let scrollLeft = $state(0);
  let scrollTop = $state(0);
  let scrollContainerEl: HTMLElement | null = $state(null);
  let rootContainerEl: HTMLDivElement | null = $state(null);
  let initialized = $state(false);

  $effect(() => {
    untrack(() => {
      const initialViewport = {
        width: Math.floor(rootContainerEl?.clientWidth || window.document.documentElement.clientWidth || window.innerWidth),
        height: Math.floor(rootContainerEl?.clientHeight || window.document.documentElement.clientHeight || window.innerHeight),
      };

      void editor.initialize({
        theme: getEditorTheme(theme.effectiveTheme, theme.lightVariant, theme.darkVariant),
        snapshot,
        fontFamilies,
        initialViewportWidth: initialViewport.width,
        initialViewportHeight: initialViewport.height,
        readOnly,
        onDocChanged,
        onSelectionChanged,
        onExitedDocumentStart,
      });
    });

    return () => {
      editor.destroy();
    };
  });

  $effect(() => {
    if ((editor.layout?.pages.length ?? 0) > 0 && editor.contentReady && !initialized) {
      initialized = true;
      tick().then(() => onEditorReady?.(editor));
    }
  });

  $effect(() => {
    if (!initialized) {
      return;
    }
    if (width > 0 && containerClientHeight > 0 && scaleFactor > 0) {
      editor.dispatch({ type: 'resize', width, height: containerClientHeight, scaleFactor: scaleFactor * zoomRenderScale });
    }
  });

  $effect(() => {
    if (initialized) {
      editor.dispatch({
        type: 'setTheme',
        theme: getEditorTheme(theme.effectiveTheme, theme.lightVariant, theme.darkVariant),
      });
    }
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

  const layoutMode = $derived(editor.layout?.layoutMode);
  const isPaginated = $derived(layoutMode?.type === 'paginated');
  const pageWidth = $derived(editor.layout?.pages[0]?.width ?? 0);
  const effectiveDisplayZoom = $derived(isPaginated ? editor.displayZoom : 1);
  const showRuler = $derived(!readOnly && layoutMode?.type === 'paginated');
  const continuousPageMargin = $derived(layoutMode?.type === 'paginated' ? 0 : CONTINUOUS_PAGE_MARGIN);
  const viewPadding = $derived(layoutMode?.type === 'paginated' ? PAGINATED_VIEW_PADDING : readOnly ? 0 : CONTINUOUS_VIEW_PADDING);
  const paginatedContentWidth = $derived(pageWidth * effectiveDisplayZoom + viewPadding * 2);
  const paginatedHeaderFooterWidth = $derived(
    layoutMode?.type === 'paginated'
      ? Math.max(
          paginatedContentWidth,
          Math.max(0, Math.min(PAGINATED_HEADER_FOOTER_MIN_WIDTH, containerClientWidth)) +
            (layoutMode.pageMarginLeft + layoutMode.pageMarginRight) * effectiveDisplayZoom +
            viewPadding * 2,
        )
      : 0,
  );
  const width = $derived(
    layoutMode?.type === 'continuous'
      ? Math.max(CONTINUOUS_MIN_WIDTH - continuousPageMargin * 2, containerClientWidth - viewPadding * 2)
      : containerClientWidth - viewPadding * 2,
  );

  $effect(() => {
    return handleDragScroll(editor.scrollViewport, editor.pointerState !== 0, {
      onScroll: (clientX, clientY) => {
        editor.handlePointerMoveFromCoordinate(clientX, clientY);
      },
    });
  });

  $effect(() => {
    if (initialized) {
      editor.dispatch({ type: 'setFocused', focused: editor.isFocused });
    }
  });
</script>

<div bind:this={rootContainerEl} class={flex({ flex: '1', direction: 'column', height: 'full', width: 'full' })}>
  {#if initialized}
    <div
      style:grid-template-columns={showRuler ? `${rulerThickness}px 1fr` : '1fr'}
      style:grid-template-rows={showRuler ? `${rulerThickness}px 1fr` : '1fr'}
      class={grid({
        flex: '1',
        gap: '0',
        overflow: 'hidden',
        ...(layoutMode?.type === 'paginated' && { backgroundColor: 'surface.subtle' }),
      })}
    >
      {#if showRuler}
        <RulerFrame headerPadding={headerHeight} pagePadding={viewPadding} {scrollLeft} {scrollTop} thickness={rulerThickness} {unit} />
      {/if}

      <EditorZoom
        {active}
        {editor}
        {resizing}
        {useWindowScroll}
        bind:containerClientWidth
        bind:containerClientHeight
        bind:renderZoom={zoomRenderScale}
        bind:scrollContainer={scrollContainerEl}
        bind:scrollLeft
        bind:scrollTop
      >
        <div
          style:min-width={layoutMode?.type === 'paginated' ? 'max-content' : `${CONTINUOUS_MIN_WIDTH}px`}
          class={flex({
            direction: 'column',
            position: 'relative',
            height: 'full',
          })}
        >
          {#if header}
            <div
              style:width={layoutMode?.type === 'paginated' ? `${paginatedHeaderFooterWidth}px` : '100%'}
              style:min-width={layoutMode?.type === 'paginated' && !readOnly ? 'max-content' : undefined}
              style:max-width={layoutMode?.type === 'paginated'
                ? 'none'
                : `${(layoutMode?.type === 'continuous' ? layoutMode.maxWidth : 0) + (viewPadding + continuousPageMargin) * 2}px`}
              style:padding-inline={`${viewPadding + continuousPageMargin}px`}
              class={flex({
                flexDirection: 'column',
                flexShrink: '0',
                width: 'full',
                marginX: 'auto',
              })}
              {@attach (el) => {
                const observer = new ResizeObserver((entries) => {
                  const entry = entries[0];
                  if (entry) {
                    headerHeight = entry.contentRect.height;
                  }
                });

                observer.observe(el);
                return () => observer.disconnect();
              }}
            >
              {@render header()}
            </div>
          {/if}
          <View />
          {#if footer}
            <div
              style:width={layoutMode?.type === 'paginated' ? `${paginatedHeaderFooterWidth}px` : '100%'}
              style:min-width={layoutMode?.type === 'paginated' && !readOnly ? 'max-content' : undefined}
              style:max-width={layoutMode?.type === 'paginated'
                ? 'none'
                : `${(layoutMode?.type === 'continuous' ? layoutMode.maxWidth : 0) + (viewPadding + continuousPageMargin) * 2}px`}
              style:padding-inline={`${viewPadding + continuousPageMargin}px`}
              class={flex({
                flexDirection: 'column',
                flexShrink: '0',
                width: 'full',
                marginX: 'auto',
              })}
            >
              {@render footer()}
            </div>
          {/if}
          {#if children}
            {@render children()}
          {/if}
        </div>
      </EditorZoom>
      {#if !useWindowScroll}
        <Scrollbar scrollContainer={scrollContainerEl} />
      {/if}
    </div>
  {/if}
</div>
