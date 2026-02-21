<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex, grid } from '@typie/styled-system/patterns';
  import { getThemeContext } from '@typie/ui/context';
  import { elementScrollViewport, handleDragScroll, windowScrollViewport } from '@typie/ui/utils';
  import { tick, untrack } from 'svelte';
  import Logo from '$assets/logos/logo.svg?component';
  import {
    CONTINUOUS_MIN_WIDTH,
    CONTINUOUS_PAGE_MARGIN,
    CONTINUOUS_VIEW_PADDING,
    PAGE_GAP,
    PAGINATED_VIEW_PADDING,
  } from '$lib/editor/constants';
  import { setupEditorContext } from '$lib/editor/context.svelte';
  import { Editor } from '$lib/editor/editor.svelte';
  import { getEditorTheme } from '$lib/editor/theme';
  import View from './core/View.svelte';
  import HorizontalRuler from './ui/HorizontalRuler.svelte';
  import Scrollbar from './ui/Scrollbar.svelte';
  import VerticalRuler from './ui/VerticalRuler.svelte';
  import type { Snippet } from 'svelte';
  import type { FontFamily } from '$lib/editor/fonts';
  import type { LayoutMode, Position } from '$lib/editor/types';

  type Props = {
    unit?: 'px' | 'cm';
    rulerThickness?: number;
    snapshot?: Uint8Array;
    readOnly?: boolean;
    useWindowScroll?: boolean;
    editor?: Editor;
    fontFamilies: FontFamily[];
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
    useWindowScroll = false,
    editor: externalEditor,
    fontFamilies,
    onDocChanged,
    onSelectionChanged,
    onExitedDocumentStart,
    onEditorReady,
    header,
    footer,
    children,
  }: Props = $props();

  const editor = externalEditor ?? new Editor();
  if (!externalEditor) {
    const ctx = setupEditorContext();
    ctx.editor = editor;
  }

  const theme = getThemeContext();

  let containerClientWidth = $state(0);
  let containerClientHeight = $state(0);
  let scaleFactor = $state(1);
  let headerHeight = $state(0);
  let horizontalRulerEl: HTMLDivElement | null = $state(null);
  let verticalRulerEl: HTMLDivElement | null = $state(null);
  let scrollContainerEl: HTMLElement | null = $state(null);
  let initialized = $state(false);

  $effect(() => {
    editor.scrollContainerEl = scrollContainerEl;
  });

  $effect(() => {
    if (useWindowScroll) {
      editor.scrollViewport = windowScrollViewport();
    } else if (scrollContainerEl) {
      editor.scrollViewport = elementScrollViewport(scrollContainerEl);
    } else {
      editor.scrollViewport = null;
    }
  });

  $effect(() => {
    untrack(() => {
      editor.initialize({
        theme: getEditorTheme(theme.effectiveTheme, theme.lightVariant, theme.darkVariant),
        snapshot,
        fontFamilies,
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
    if (editor.layout.pages.length > 0 && !initialized) {
      initialized = true;
      tick().then(() => onEditorReady?.(editor));
    }
  });

  $effect(() => {
    if (width > 0 && containerClientHeight > 0 && scaleFactor > 0) {
      editor.dispatch({ type: 'resize', width, height: containerClientHeight, scaleFactor });
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

  const layoutMode = $derived<LayoutMode>(editor.layout.layoutMode);
  const showRuler = $derived(!readOnly && layoutMode.type === 'paginated');
  const pages = $derived(editor.layout.pages);
  const pageWidth = $derived(pages[0]?.width ?? 0);
  const marginTop = $derived(layoutMode.type === 'paginated' ? layoutMode.pageMarginTop : 0);
  const marginBottom = $derived(layoutMode.type === 'paginated' ? layoutMode.pageMarginBottom : 0);
  const marginLeft = $derived(layoutMode.type === 'paginated' ? layoutMode.pageMarginLeft : 0);
  const marginRight = $derived(layoutMode.type === 'paginated' ? layoutMode.pageMarginRight : 0);

  const pageGap = $derived(layoutMode.type === 'paginated' ? PAGE_GAP : 0);
  const continuousPageMargin = $derived(layoutMode.type === 'paginated' ? 0 : CONTINUOUS_PAGE_MARGIN);
  const viewPadding = $derived(layoutMode.type === 'paginated' ? PAGINATED_VIEW_PADDING : readOnly ? 0 : CONTINUOUS_VIEW_PADDING);
  const width = $derived(
    layoutMode.type === 'continuous'
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

<div class={flex({ flex: '1', direction: 'column', height: 'full', width: 'full' })}>
  {#if initialized}
    <div
      style:grid-template-columns={showRuler ? `${rulerThickness}px 1fr` : '1fr'}
      style:grid-template-rows={showRuler ? `${rulerThickness}px 1fr` : '1fr'}
      class={grid({
        flex: '1',
        gap: '0',
        overflow: 'hidden',
        ...(layoutMode.type === 'paginated' && { backgroundColor: 'surface.subtle' }),
      })}
    >
      {#if showRuler}
        <div
          class={css({
            borderRightWidth: '1px',
            borderBottomWidth: '1px',
            borderColor: 'border.strong',
            backgroundColor: 'surface.default',
          })}
        ></div>

        <div class={css({ overflow: 'hidden' })}>
          {#if pageWidth}
            <HorizontalRuler
              {marginLeft}
              {marginRight}
              padding={viewPadding}
              {pageWidth}
              thickness={rulerThickness}
              {unit}
              bind:ref={horizontalRulerEl}
            />
          {/if}
        </div>

        <div class={css({ overflow: 'hidden' })}>
          {#if pages.length > 0}
            <VerticalRuler
              {marginBottom}
              {marginTop}
              padding={headerHeight}
              {pageGap}
              {pages}
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
        })}
        {@attach (el) => {
          const observer = new ResizeObserver(() => {
            containerClientWidth = el.clientWidth;
            containerClientHeight = el.clientHeight;
          });
          observer.observe(el);
          containerClientWidth = el.clientWidth;
          containerClientHeight = el.clientHeight;

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
        <div
          style:min-width={layoutMode.type === 'paginated' ? 'max-content' : `${CONTINUOUS_MIN_WIDTH}px`}
          class={flex({
            direction: 'column',
            position: 'relative',
            height: 'full',
          })}
        >
          {#if header}
            <div
              style:width={layoutMode.type === 'paginated' ? `${pageWidth + viewPadding * 2}px` : '100%'}
              style:max-width={layoutMode.type === 'paginated'
                ? 'none'
                : `${layoutMode.maxWidth + (viewPadding + continuousPageMargin) * 2}px`}
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
              style:width={layoutMode.type === 'paginated' ? `${pageWidth + viewPadding * 2}px` : '100%'}
              style:max-width={layoutMode.type === 'paginated'
                ? 'none'
                : `${layoutMode.maxWidth + (viewPadding + continuousPageMargin) * 2}px`}
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
      </div>
      {#if !useWindowScroll}
        <Scrollbar scrollContainer={scrollContainerEl} />
      {/if}
    </div>
  {:else}
    <div class={center({ flex: '1', size: 'full' })}>
      <Logo
        class={css({
          size: '32px',
          filter: '[grayscale(100%)]',
          animation: 'pulse 2s ease-in-out infinite',
        })}
      />
    </div>
  {/if}
</div>
