<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { getThemeContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import { elementScrollViewport, windowScrollViewport } from '@typie/ui/utils';
  import { onDestroy, untrack } from 'svelte';
  import { graphql } from '$mearie';
  import {
    CONTINUOUS_MIN_WIDTH,
    CONTINUOUS_VIEW_PADDING,
    PAGE_GAP,
    PAGINATED_HEADER_FOOTER_MIN_SCALE,
    PAGINATED_HEADER_FOOTER_MIN_WIDTH,
  } from '../constants';
  import { browserScaleFactor, getEditorContext } from '../editor.svelte';
  import { loadFonts } from '../fonts';
  import { handle } from '../handlers';
  import { handleContextMenu } from '../handlers/contextmenu';
  import { handleDragEnd, handleDragEnter, handleDragLeave, handleDragOver, handleDragStart, handleDrop } from '../handlers/dnd';
  import {
    cancelPointerInteraction,
    handleClick,
    handlePointerCancel,
    handlePointerCaptureLost,
    handlePointerDown,
    handlePointerMove,
    handlePointerUp,
  } from '../handlers/pointer';
  import { setupEditorScroll } from '../scroll.svelte';
  import { touchPanLock } from '../touch-pan-lock';
  import Caret from './Caret.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import DocumentOverlayLayer from './DocumentOverlayLayer.svelte';
  import Input from './Input.svelte';
  import LineHighlight from './LineHighlight.svelte';
  import LinkTooltip from './LinkTooltip.svelte';
  import Page from './Page.svelte';
  import PlaceholderOverlay from './PlaceholderOverlay.svelte';
  import RepasteAsText from './RepasteAsText.svelte';
  import Scrollbar from './Scrollbar.svelte';
  import SelectionHandles from './SelectionHandles.svelte';
  import EditorZoom from './ui/EditorZoom.svelte';
  import ViewportOverlay from './ViewportOverlay.svelte';
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
  onDestroy(() => {
    if (ctx.editor) cancelPointerInteraction(ctx.editor);
  });

  $effect(() => {
    const editor = ctx.editor;
    return () => {
      ctx.attachmentDropTargetNodeId = null;
      if (editor) ctx.attachmentImporter.cancelEditor(editor);
    };
  });

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
            url
            hash
          }
        }
      }
    `),
    () => document$key,
  );

  let clientWidth = $state<number>();
  let clientHeight = $state<number>();
  let windowViewportHeight = $state<number>();
  let scaleFactor = $state(browserScaleFactor());

  $effect(() => {
    const sync = () => {
      scaleFactor = browserScaleFactor();
    };

    sync();
    window.visualViewport?.addEventListener('resize', sync);
    window.addEventListener('resize', sync);
    return () => {
      window.visualViewport?.removeEventListener('resize', sync);
      window.removeEventListener('resize', sync);
    };
  });

  $effect(() => {
    if (!useWindowScroll) return;

    const sync = () => {
      windowViewportHeight = window.visualViewport?.height ?? window.innerHeight;
    };

    sync();
    window.visualViewport?.addEventListener('resize', sync);
    // eslint-disable-next-line unicorn/prefer-observer-apis -- tracks visualViewport/window height, not element resize
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
    const viewportScaleFactor = scaleFactor;
    const isContinuous = !isPaginated;
    if (!editor || !width || !height) return;
    const effectiveWidth = isContinuous ? Math.max(CONTINUOUS_MIN_WIDTH, width) : width;

    untrack(() => {
      editor.resizeViewport(effectiveWidth, height, viewportScaleFactor);

      if (!readyFired && editor.viewportResized) {
        readyFired = true;
        loadFonts(document.data.editorFontFamilies);
        onReady?.();
      }
    });
  });

  $effect(() => {
    const families = document.data.editorFontFamilies;
    untrack(() => {
      if (readyFired) {
        loadFonts(families);
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
          editor.scrollRootEl = useWindowScroll ? null : el;
          return () => {
            if (editor.scrollContainerEl !== el) {
              return;
            }

            editor.scrollContainerEl = undefined;
            editor.scrollViewport = undefined;
            editor.scrollRootEl = undefined;
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
          bind:this={ctx.editor.extensionAreaEl}
          style:cursor
          style:min-width={editorMinWidth}
          style:padding-bottom={`${ctx.scroll?.bottomPadding ?? 0}px`}
          class={css(
            {
              position: 'relative',
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              flexGrow: '1',
              width: 'full',
              userSelect: 'none',
              ...(isPaginated && {
                rowGap: 'var(--page-gap)',
              }),
            },
            ctx.editor.readOnly && {
              WebkitUserSelect: 'none',
              WebkitTouchCallout: 'none',
            },
          )}
          draggable={ctx.editor.isSelectionCollapsed ? undefined : true}
          onclick={handle(ctx.editor, handleClick)}
          oncontextmenu={handle(ctx.editor, handleContextMenu)}
          ondragend={() => handleDragEnd(ctx)}
          ondragenter={(event) => handleDragEnter(ctx, event)}
          ondragleave={(event) => handleDragLeave(ctx, event)}
          ondragover={(event) => handleDragOver(ctx, event)}
          ondragstart={(event) => handleDragStart(ctx, event)}
          ondrop={(event) =>
            handleDrop(ctx, event, ({ file, kind }) => {
              Toast.error(`${file.name} ${kind === 'image' ? '이미지' : '파일'} 업로드에 실패했습니다.`);
            })}
          onfocusin={() => ctx.editor?.focus()}
          onfocusout={(event) => {
            if (!window.document.hasFocus()) return;
            if (event.relatedTarget === ctx.editor?.inputEl) return;
            ctx.editor?.blur();
          }}
          onlostpointercapture={handle(ctx.editor, handlePointerCaptureLost)}
          onpointercancel={handle(ctx.editor, handlePointerCancel)}
          onpointerdown={handle(ctx.editor, handlePointerDown)}
          onpointerleave={() => ctx.editor?.clearLinkHover()}
          onpointermove={handle(ctx.editor, handlePointerMove)}
          onpointerup={handle(ctx.editor, handlePointerUp)}
          role="textbox"
          tabindex={0}
          use:touchPanLock={ctx.editor.gesture.panLockActive}
        >
          {#each ctx.editor.pageSizes as { width, height }, i (i)}
            <Page backingHeight={ctx.editor.pageBackingSizes[i]?.height ?? height} {height} page={i} {width} />
          {/each}

          <DocumentOverlayLayer />

          <Caret />

          <LineHighlight />

          <PlaceholderOverlay />

          <ViewportOverlay>
            <Input />

            <RepasteAsText />

            {#if ctx.editor.readOnly}
              <SelectionHandles />
            {/if}
          </ViewportOverlay>

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
