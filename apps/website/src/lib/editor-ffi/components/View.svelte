<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { getThemeContext } from '@typie/ui/context';
  import { elementScrollViewport, windowScrollViewport } from '@typie/ui/utils';
  import { tick, untrack } from 'svelte';
  import { graphql } from '$mearie';
  import {
    CONTINUOUS_MIN_WIDTH,
    CONTINUOUS_VIEW_PADDING,
    PAGE_GAP,
    PAGINATED_HEADER_FOOTER_MIN_SCALE,
    PAGINATED_HEADER_FOOTER_MIN_WIDTH,
  } from '../constants';
  import { getEditorContext } from '../editor.svelte';
  import { loadFonts } from '../fonts';
  import { handle } from '../handlers';
  import { handleContextMenu } from '../handlers/contextmenu';
  import { handleDragEnd, handleDragEnter, handleDragLeave, handleDragOver, handleDragStart, handleDrop } from '../handlers/dnd';
  import { handleClick, handlePointerCancel, handlePointerDown, handlePointerMove, handlePointerUp } from '../handlers/pointer';
  import { setupEditorScroll } from '../scroll.svelte';
  import Caret from './Caret.svelte';
  import CaretPositioned from './CaretPositioned.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import Input from './Input.svelte';
  import LineHighlight from './LineHighlight.svelte';
  import LinkTooltip from './LinkTooltip.svelte';
  import Page from './Page.svelte';
  import PlaceholderOverlay from './PlaceholderOverlay.svelte';
  import RepasteAsText from './RepasteAsText.svelte';
  import Scrollbar from './Scrollbar.svelte';
  import SelectionHandles from './SelectionHandles.svelte';
  import EditorZoom from './ui/EditorZoom.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';
  import type { Editor_document$key } from '$mearie';

  type Props = {
    document$key: Editor_document$key;
    active?: boolean;
    /** window 자체를 스크롤 컨테이너로 사용한다. 페이지당 에디터 1개를 전제한다 (window에 리스너를 부착). */
    useWindowScroll?: boolean;
    style?: SystemStyleObject;
    onReady?: () => void;
    header?: Snippet;
    footer?: Snippet;
    children?: Snippet;
  };

  let { document$key, active = true, useWindowScroll = false, style, onReady, header, footer, children }: Props = $props();

  const ctx = getEditorContext();
  const theme = getThemeContext();
  setupEditorScroll(ctx);

  const document = createFragment(
    graphql(`
      fragment Editor_document on IDocument {
        id

        editorFontFamilies: fontFamilies(sources: [DEFAULT, USER, FALLBACK]) {
          id
          familyName
          source
          fonts {
            id
            weight
            path
            hash
            chunks
          }
        }
      }
    `),
    () => document$key,
  );

  let clientWidth = $state<number>();
  let clientHeight = $state<number>();
  let windowViewportHeight = $state<number>();

  $effect(() => {
    if (!useWindowScroll) return;

    const sync = () => {
      windowViewportHeight = window.visualViewport?.height ?? window.innerHeight;
    };

    sync();
    window.visualViewport?.addEventListener('resize', sync);
    window.addEventListener('resize', sync);
    return () => {
      window.visualViewport?.removeEventListener('resize', sync);
      window.removeEventListener('resize', sync);
    };
  });

  const layoutMode = $derived(ctx.editor?.rootAttrs?.layout_mode);
  const isPaginated = $derived(layoutMode?.type === 'paginated');
  const pageWidth = $derived(ctx.editor?.pageSizes[0]?.width ?? 0);
  const displayZoom = $derived(isPaginated ? (ctx.editor?.displayZoom ?? 1) : 1);
  const pageGap = $derived(PAGE_GAP * displayZoom);
  const framePadding = $derived(isPaginated ? 0 : CONTINUOUS_VIEW_PADDING);
  const editorMinWidth = $derived(isPaginated ? 'max-content' : `${CONTINUOUS_MIN_WIDTH}px`);
  const continuousMaxFrameWidth = $derived(
    layoutMode?.type === 'continuous' ? `${layoutMode.max_width + CONTINUOUS_VIEW_PADDING * 2}px` : undefined,
  );
  const paginatedContentWidth = $derived(pageWidth * displayZoom + framePadding * 2);
  const paginatedHeaderFooterMinContentWidth = $derived(
    Math.max(PAGINATED_HEADER_FOOTER_MIN_WIDTH * displayZoom, PAGINATED_HEADER_FOOTER_MIN_WIDTH * PAGINATED_HEADER_FOOTER_MIN_SCALE),
  );
  const paginatedHeaderFooterHorizontalInset = $derived(
    layoutMode?.type === 'paginated' ? (layoutMode.page_margin_left + layoutMode.page_margin_right) * displayZoom + framePadding * 2 : 0,
  );
  const paginatedHeaderFooterMaxContentWidth = $derived(Math.max(0, (clientWidth ?? 0) - paginatedHeaderFooterHorizontalInset));
  const paginatedHeaderFooterTargetWidth = $derived(
    paginatedHeaderFooterHorizontalInset + Math.min(paginatedHeaderFooterMinContentWidth, paginatedHeaderFooterMaxContentWidth),
  );
  const paginatedHeaderFooterWidth = $derived(
    layoutMode?.type === 'paginated' ? Math.max(paginatedContentWidth, paginatedHeaderFooterTargetWidth) : 0,
  );
  const headerFooterMinWidth = $derived(isPaginated && !(ctx.editor?.readOnly ?? false) ? 'max-content' : editorMinWidth);

  const cursor = $derived.by(() => {
    const editor = ctx.editor;
    if (!editor) return;
    if (editor.linkHover && (editor.readOnly || editor.modifierHeld)) return 'pointer';
    return editor.pointerStyle;
  });

  let readyFired = false;

  $effect(() => {
    const editor = ctx.editor;
    const width = clientWidth;
    const height = useWindowScroll ? windowViewportHeight : clientHeight;
    const isContinuous = !isPaginated;
    if (!editor || !width || !height) return;
    const effectiveWidth = isContinuous ? Math.max(CONTINUOUS_MIN_WIDTH, width) : width;

    untrack(() => {
      editor.resizeViewport(effectiveWidth, height, window.devicePixelRatio);

      if (!readyFired && editor.viewportResized) {
        readyFired = true;
        loadFonts(document.data.editorFontFamilies);
        onReady?.();
        if (active) {
          void tick().then(() => editor.focus());
        }
      }
    });
  });

  $effect(() => {
    ctx.editor?.setThemeVariant(theme.currentThemeVariant);
  });
</script>

<svelte:window onscroll={useWindowScroll ? () => ctx.editor?.refreshPointerStyle() : undefined} />

<div
  class={css(
    {
      position: 'relative',
      display: 'flex',
      flexDirection: 'column',
      minHeight: '0',
      ...(!useWindowScroll && {
        overflow: 'hidden',
      }),
      ...(isPaginated && {
        backgroundColor: 'surface.subtle',
      }),
    },
    style,
  )}
>
  <div
    style:--page-gap={isPaginated ? `${pageGap}px` : undefined}
    class={css({
      display: 'flex',
      flexDirection: 'column',
      flexGrow: '1',
      position: 'relative',
      ...(!useWindowScroll && {
        minHeight: '0',
        overflow: 'auto',
        overflowAnchor: 'none',
        scrollbar: 'hidden',
      }),
    })}
    {@attach (el) => {
      const teardown = $effect.root(() => {
        $effect(() => {
          const editor = ctx.editor;
          if (!editor) return;

          editor.scrollContainerEl = el;
          editor.scrollViewport = useWindowScroll ? windowScrollViewport() : elementScrollViewport(el);
          return () => {
            if (editor.scrollContainerEl === el) {
              editor.scrollContainerEl = undefined;
              editor.scrollViewport = undefined;
            }
          };
        });
      });

      return () => teardown();
    }}
    onscroll={() => ctx.editor?.refreshPointerStyle()}
    bind:clientWidth
    bind:clientHeight
  >
    {#if ctx.editor && header}
      <div
        style:width={layoutMode?.type === 'paginated' ? `${paginatedHeaderFooterWidth}px` : '100%'}
        style:min-width={headerFooterMinWidth}
        style:max-width={continuousMaxFrameWidth}
        style:padding-inline={`${framePadding}px`}
        class={css({ flexShrink: '0', marginX: 'auto' })}
      >
        {@render header()}
      </div>
    {/if}

    {#if ctx.editor}
      <EditorZoom {active} editor={ctx.editor} {isPaginated} {pageWidth} viewportWidth={clientWidth ?? 0}>
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          bind:this={ctx.editor.surfaceEl}
          style:cursor
          style:min-width={editorMinWidth}
          style:max-width={continuousMaxFrameWidth}
          style:padding-bottom={`${ctx.scroll?.bottomPadding ?? 0}px`}
          class={css({
            position: 'relative',
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            flexGrow: '1',
            width: 'full',
            marginX: 'auto',
            userSelect: 'none',
            ...(isPaginated && {
              rowGap: 'var(--page-gap)',
            }),
          })}
          draggable={ctx.editor.isSelectionCollapsed ? undefined : true}
          onclick={handle(ctx.editor, handleClick)}
          oncontextmenu={handle(ctx.editor, handleContextMenu)}
          ondragend={() => handleDragEnd(ctx)}
          ondragenter={(event) => handleDragEnter(ctx, event)}
          ondragleave={(event) => handleDragLeave(ctx, event)}
          ondragover={(event) => handleDragOver(ctx, event)}
          ondragstart={(event) => handleDragStart(ctx, event)}
          ondrop={(event) => handleDrop(ctx, event)}
          onfocusin={() => ctx.editor?.focus()}
          onfocusout={(event) => {
            if (!window.document.hasFocus()) return;
            if (event.relatedTarget === ctx.editor?.inputEl) return;
            ctx.editor?.blur();
          }}
          onpointercancel={handle(ctx.editor, handlePointerCancel)}
          onpointerdown={handle(ctx.editor, handlePointerDown)}
          onpointerleave={() => ctx.editor?.clearLinkHover()}
          onpointermove={handle(ctx.editor, handlePointerMove)}
          onpointerup={handle(ctx.editor, handlePointerUp)}
          role="textbox"
          tabindex={0}
        >
          {#each ctx.editor.pageSizes as { width, height }, i (i)}
            <Page {height} page={i} {width} />
          {/each}

          <CaretPositioned>
            <Caret />
            <Input />
          </CaretPositioned>

          <LineHighlight />

          <PlaceholderOverlay />

          <RepasteAsText />

          {#if ctx.editor.readOnly}
            <SelectionHandles />
          {/if}

          <ContextMenu />

          <LinkTooltip />

          {#if children}
            {@render children()}
          {/if}
        </div>
      </EditorZoom>
    {/if}

    {#if ctx.editor && footer}
      <div
        style:width={layoutMode?.type === 'paginated' ? `${paginatedHeaderFooterWidth}px` : '100%'}
        style:min-width={headerFooterMinWidth}
        style:max-width={continuousMaxFrameWidth}
        style:padding-inline={`${framePadding}px`}
        class={css({ flexShrink: '0', marginX: 'auto' })}
      >
        {@render footer()}
      </div>
    {/if}
  </div>

  {#if ctx.editor && !useWindowScroll}
    <Scrollbar />
  {/if}
</div>
