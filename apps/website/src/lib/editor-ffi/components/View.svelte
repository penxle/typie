<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { token } from '@typie/styled-system/tokens';
  import { getThemeContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import { elementScrollViewport, windowScrollViewport } from '@typie/ui/utils';
  import { onDestroy, untrack } from 'svelte';
  import { graphql } from '$mearie';
  import { CONTINUOUS_MIN_WIDTH, CONTINUOUS_VIEW_PADDING, PAGE_GAP } from '../constants';
  import { browserScaleFactor, getEditorContext } from '../editor.svelte';
  import { setupEditorPublication } from '../editor-publication.svelte';
  import { EditorSurfaceHost } from '../editor-surface-host.svelte';
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
  import { resolveContinuousLayoutViewportWidth } from '../zoom';
  import Caret from './Caret.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import DocumentOverlayLayer from './DocumentOverlayLayer.svelte';
  import EditorPages from './EditorPages.svelte';
  import { resolveHeaderGeometry } from './header-geometry';
  import Input from './Input.svelte';
  import LineHighlight from './LineHighlight.svelte';
  import LinkTooltip from './LinkTooltip.svelte';
  import PlaceholderOverlay from './PlaceholderOverlay.svelte';
  import RepasteAsText from './RepasteAsText.svelte';
  import Scrollbar from './Scrollbar.svelte';
  import SelectionHandles from './SelectionHandles.svelte';
  import EditorContextBar from './ui/EditorContextBar.svelte';
  import EditorZoom from './ui/EditorZoom.svelte';
  import EditorZoomControls from './ui/EditorZoomControls.svelte';
  import ViewportOverlay from './ViewportOverlay.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';
  import type { Editor_document$key } from '$mearie';
  import type { DocumentZoomLayout } from '../zoom';
  import type { EditorContextBarSegmentRenderProps } from './ui/EditorContextBar.svelte';

  type Props = {
    document$key: Editor_document$key;
    active?: boolean;
    viewer?: boolean;
    /** window 자체를 스크롤 컨테이너로 사용한다. 페이지당 에디터 1개를 전제한다 (window에 리스너를 부착). */
    useWindowScroll?: boolean;
    style?: SystemStyleObject;
    /** 본문 좌우에 비워 둘 폭. 여백 레이어(프리즘 리뷰)가 레일·카드 컬럼을 놓는 자리다. */
    contentInsetLeft?: number;
    contentInsetRight?: number;
    /** 최종 레이아웃으로 바뀐 본문을 이전 화면 위치에서 합성하는 일회성 모션. */
    contentMotion?: { fromX: number; duration: number; easing: string };
    breadcrumb?: Snippet<[EditorContextBarSegmentRenderProps]>;
    onReady?: () => void;
    header?: Snippet;
    footer?: Snippet;
    children?: Snippet;
  };

  let {
    document$key,
    active = true,
    viewer = false,
    useWindowScroll = false,
    style,
    contentInsetLeft = 0,
    contentInsetRight = 0,
    contentMotion,
    breadcrumb,
    onReady,
    header,
    footer,
    children,
  }: Props = $props();

  const ctx = getEditorContext();
  const theme = getThemeContext();

  // 이 View 인스턴스가 소유한다. 공유 컨텍스트에 두면 {#key ctx.editor}로 View가 교체될 때
  // 새 인스턴스가 옛 인스턴스의 host를 읽어버린다 (새 브랜치 생성이 옛 브랜치 파괴보다 먼저다).
  let surfaceHost = $state<EditorSurfaceHost>();
  let editorViewSurface = $state<HTMLElement>();

  setupEditorScroll(ctx);
  setupEditorPublication(ctx, () => surfaceHost);
  onDestroy(() => {
    if (ctx.editor) cancelPointerInteraction(ctx.editor);
  });

  $effect(() => {
    const editor = ctx.editor;
    const scroll = ctx.scroll;
    if (!editor || !scroll) return;
    const host = new EditorSurfaceHost(editor, (revision) => scroll.discardFailedForRevision(revision));
    surfaceHost = host;
    return () => {
      ctx.scroll?.cancel();
      surfaceHost = undefined;
      host.destroy();
    };
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
  const layoutBackground = $derived(token(isPaginated ? 'colors.surface.subtle' : 'colors.surface.default'));
  const pageWidth = $derived(ctx.editor?.pageSizes[0]?.width ?? 0);
  const zoomLayout: DocumentZoomLayout | null = $derived.by(() => {
    if (layoutMode?.type === 'continuous' && layoutMode.max_width > 0) {
      return { type: 'continuous', maxWidth: layoutMode.max_width };
    }
    if (layoutMode?.type === 'paginated' && pageWidth > 0) {
      return { type: 'paginated', pageWidth };
    }
    return null;
  });
  const displayZoom = $derived(zoomLayout ? (ctx.editor?.displayZoom ?? 1) : 1);
  const pageGap = $derived(PAGE_GAP * displayZoom);
  const editorMinWidth = $derived(isPaginated ? 'max-content' : `${CONTINUOUS_MIN_WIDTH}px`);
  const continuousMaxFrameWidth = $derived(
    layoutMode?.type === 'continuous' ? `${layoutMode.max_width + CONTINUOUS_VIEW_PADDING * 2}px` : undefined,
  );
  const contentAnimation = $derived.by(() => {
    if (!contentMotion || contentMotion.fromX === 0) return;
    const name = contentMotion.fromX > 0 ? 'editor-content-from-right' : 'editor-content-from-left';
    return `${name} ${contentMotion.duration}ms ${contentMotion.easing} both`;
  });
  // 헤더 폴백(판 없음)에서도 인셋은 래퍼 패딩으로 들어간다 — 상한도 그만큼 넓어야 안쪽 트랙이 그대로 남는다
  const headerFallbackMaxWidth = $derived(
    layoutMode?.type === 'continuous'
      ? `${layoutMode.max_width + CONTINUOUS_VIEW_PADDING * 2 + contentInsetLeft + contentInsetRight}px`
      : undefined,
  );
  const bodyTrackWidth = $derived(pageWidth * displayZoom);
  const headerGeometry = $derived.by(() => {
    if (!layoutMode) return;

    // 페이지 제 여백과 프리즘 인셋은 다른 값이다 — 같은 이름으로 가리면 인셋이 헤더에 닿지 않고 여백이 두 번 빠진다
    const [pageInsetLeft, pageInsetRight] =
      layoutMode.type === 'paginated'
        ? [layoutMode.page_margin_left, layoutMode.page_margin_right]
        : [CONTINUOUS_VIEW_PADDING, CONTINUOUS_VIEW_PADDING];

    return resolveHeaderGeometry({
      viewportWidth: Math.max(0, (clientWidth ?? 0) - contentInsetLeft - contentInsetRight),
      displayZoom,
      bodyTrackWidth,
      contentInsetLeft: pageInsetLeft,
      contentInsetRight: pageInsetRight,
    });
  });

  const cursor = $derived.by(() => {
    const editor = ctx.editor;
    if (!editor) return;
    if (editor.linkHover && (editor.readOnly || editor.modifierHeld)) return 'pointer';
    return editor.pointerStyle;
  });

  let readyFired = false;
  let viewportEditor: (typeof ctx)['editor'];
  let viewportLayoutType: 'continuous' | 'paginated' | undefined;
  let viewportClientWidth: number | undefined;
  let viewportClientHeight: number | undefined;
  let lastViewportScaleFactor: number | undefined;

  $effect(() => {
    const editor = ctx.editor;
    const width = clientWidth;
    const height = useWindowScroll ? windowViewportHeight : clientHeight;
    const viewportScaleFactor = scaleFactor;
    const isContinuous = layoutMode?.type === 'continuous';
    const renderZoom = editor?.renderZoom ?? 1;
    if (!editor || !width || !height) return;
    const physicalWidth = isContinuous ? Math.max(CONTINUOUS_MIN_WIDTH, width) : width;
    const effectiveWidth = isContinuous
      ? resolveContinuousLayoutViewportWidth({ viewportWidth: physicalWidth, committedZoom: renderZoom })
      : physicalWidth;

    untrack(() => {
      const sameEditor = viewportEditor === editor;
      const committedLayoutChanged = sameEditor && viewportLayoutType !== layoutMode?.type;
      const physicalViewportChanged =
        sameEditor && (viewportClientWidth !== width || viewportClientHeight !== height || lastViewportScaleFactor !== viewportScaleFactor);
      viewportLayoutType = layoutMode?.type;
      viewportClientWidth = width;
      viewportClientHeight = height;
      lastViewportScaleFactor = viewportScaleFactor;
      if (sameEditor && !committedLayoutChanged) {
        editor.resizeViewport(effectiveWidth, height, viewportScaleFactor);
      } else {
        viewportEditor = editor;
        editor.resizeViewportNow(effectiveWidth, height, viewportScaleFactor);
      }
      if (physicalViewportChanged) ctx.scroll?.reconcileViewportResize();
    });

    if (!readyFired && editor.isPublished(editor.appliedRevision, { requireFrame: true })) {
      readyFired = true;
      untrack(() => {
        loadFonts(document.data.editorFontFamilies);
        onReady?.();
      });
    }
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

  // 화면 좌표 소비자는 줌 자체가 아니라 현재 제시된 문서 geometry를 관찰한다.
  // 선언 입력과 페이지 요소 설치가 한 프레임에 여러 번 바뀌어도 Editor가 한 번만 발행한다.
  $effect(() => {
    const editor = ctx.editor;
    if (!editor) return;
    void editor.publishedRevision;
    void editor.safeDisplayZoom();
    void editor.scaleFactor;
    void editor.extensionAreaEl;
    void editor.documentTrackEl;
    void clientWidth;
    void clientHeight;
    void windowViewportHeight;
    void contentInsetLeft;
    void contentInsetRight;
    for (const pageEl of Object.values(editor.pageEls)) void pageEl;
    untrack(() => editor.requestPresentationGeometryUpdate());
  });

  // 바깥 CSS나 내재 크기 변화처럼 선언 입력으로 드러나지 않는 geometry 변화의 폴백이다.
  $effect(() => {
    const editor = ctx.editor;
    const area = editor?.extensionAreaEl;
    const track = editor?.documentTrackEl;
    if (!editor || !area) return;
    const observer = new ResizeObserver(() => editor.requestPresentationGeometryUpdate());
    observer.observe(area);
    if (track) observer.observe(track);
    return () => observer.disconnect();
  });
</script>

<svelte:window
  onkeydowncapture={useWindowScroll ? () => ctx.scroll?.cancel() : undefined}
  onpointerdowncapture={useWindowScroll ? () => ctx.scroll?.cancel() : undefined}
  onscroll={useWindowScroll
    ? () => {
        ctx.editor?.refreshPointerStyle();
        ctx.scroll?.observeViewportScroll();
        ctx.editor?.requestPublication();
      }
    : undefined}
  onwheel={useWindowScroll ? () => ctx.scroll?.cancel() : undefined}
/>

<div
  bind:this={editorViewSurface}
  style:--editor-layout-background={layoutBackground}
  class={css(
    {
      position: 'relative',
      display: 'flex',
      flexDirection: 'column',
      minHeight: '0',
      backgroundColor: '[var(--editor-layout-background)]',
      ...(!useWindowScroll && {
        overflow: 'hidden',
      }),
    },
    style,
  )}
>
  <div
    style:--page-gap={isPaginated ? `${pageGap}px` : undefined}
    style:overflow-x={!useWindowScroll && contentAnimation ? 'clip' : undefined}
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
    onkeydowncapture={() => ctx.scroll?.cancel()}
    onpointerdowncapture={() => ctx.scroll?.cancel()}
    onscroll={() => {
      ctx.editor?.refreshPointerStyle();
      ctx.scroll?.observeViewportScroll();
      ctx.editor?.requestPublication();
    }}
    onwheel={() => ctx.scroll?.cancel()}
    bind:clientWidth
    bind:clientHeight
  >
    {#if ctx.editor && header}
      <div
        style:--editor-content-from-x={`${contentMotion?.fromX ?? 0}px`}
        style:animation={contentAnimation}
        style:width={headerGeometry ? `${headerGeometry.trackWidth + contentInsetLeft + contentInsetRight}px` : '100%'}
        style:min-width={headerGeometry ? undefined : editorMinWidth}
        style:max-width={headerGeometry ? undefined : headerFallbackMaxWidth}
        style:padding-inline={headerGeometry
          ? `${contentInsetLeft}px ${contentInsetRight}px`
          : `${contentInsetLeft + CONTINUOUS_VIEW_PADDING}px ${contentInsetRight + CONTINUOUS_VIEW_PADDING}px`}
        class={css({ flexShrink: '0', marginX: 'auto' })}
      >
        <div
          style:margin-left={headerGeometry ? `${headerGeometry.contentInsetLeft}px` : undefined}
          style:margin-right={headerGeometry ? `${headerGeometry.contentInsetRight}px` : undefined}
        >
          <div
            style:width={headerGeometry ? `${headerGeometry.fieldWidth}px` : '100%'}
            style:left={headerGeometry ? `${headerGeometry.stickyLeft}px` : undefined}
            class={css({ position: 'sticky' })}
          >
            {@render header()}
          </div>
        </div>
      </div>
    {/if}

    {#if ctx.editor}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div
        bind:this={ctx.editor.extensionAreaEl}
        style:cursor
        style:min-width="max-content"
        style:padding-left={`${contentInsetLeft}px`}
        style:padding-right={`${contentInsetRight}px`}
        style:padding-bottom={viewer ? '0px' : `${ctx.scroll?.bottomPadding ?? 0}px`}
        class={css(
          {
            position: 'relative',
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            flexGrow: '1',
            isolation: 'isolate',
            width: 'full',
            userSelect: 'none',
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
        onpointerdown={(event) => {
          ctx.scroll?.cancel();
          const editor = ctx.editor;
          if (editor) handlePointerDown(editor, event);
        }}
        onpointerleave={() => ctx.editor?.clearLinkHover()}
        onpointermove={handle(ctx.editor, handlePointerMove)}
        onpointerup={handle(ctx.editor, handlePointerUp)}
        role="textbox"
        tabindex={0}
        use:touchPanLock={ctx.editor.gesture.panLockActive}
      >
        <div
          bind:this={ctx.editor.documentTrackEl}
          style:--editor-content-from-x={`${contentMotion?.fromX ?? 0}px`}
          style:animation={contentAnimation}
          class={css({
            position: 'relative',
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            flexGrow: '1',
            flexShrink: '0',
            width: 'full',
            ...(isPaginated && {
              rowGap: 'var(--page-gap)',
            }),
          })}
          data-editor-document-track
        >
          <EditorPages editor={ctx.editor} {surfaceHost} />

          <DocumentOverlayLayer />

          <Caret />

          <LineHighlight />

          <PlaceholderOverlay />
        </div>

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
    {/if}

    {#if ctx.editor && footer}
      <div
        style:width={isPaginated && headerGeometry ? `${headerGeometry.trackWidth}px` : '100%'}
        style:min-width={isPaginated ? undefined : editorMinWidth}
        style:max-width={continuousMaxFrameWidth}
        style:padding-inline={isPaginated ? undefined : `${CONTINUOUS_VIEW_PADDING}px`}
        class={css({ flexShrink: '0', marginX: 'auto' })}
      >
        {@render footer()}
      </div>
    {/if}
  </div>

  {#if ctx.editor}
    <EditorZoom {active} editor={ctx.editor} {editorViewSurface} layout={zoomLayout} scroll={ctx.scroll} viewportWidth={clientWidth ?? 0}>
      {#snippet zoomControls({ controls, showViewControlsOnPaneEntry })}
        {#if editorViewSurface}
          <EditorContextBar {breadcrumb} {editorViewSurface} interactiveViewControlsWhenHidden {showViewControlsOnPaneEntry}>
            {#snippet viewControls({ state, presentation }: EditorContextBarSegmentRenderProps)}
              <div class={css({ display: 'flex', alignItems: 'center' })} aria-label="보기 제어" role="group">
                <EditorZoomControls {...controls} segment={state} visible={presentation.visible} />
              </div>
            {/snippet}
          </EditorContextBar>
        {/if}
      {/snippet}
    </EditorZoom>
  {/if}

  {#if ctx.editor && !useWindowScroll}
    <Scrollbar />
  {/if}
</div>

<style>
  @keyframes -global-editor-content-from-right {
    from {
      translate: var(--editor-content-from-x) 0;
    }
    to {
      translate: 0 0;
    }
  }

  @keyframes -global-editor-content-from-left {
    from {
      translate: var(--editor-content-from-x) 0;
    }
    to {
      translate: 0 0;
    }
  }
</style>
