<script lang="ts" module>
  export type EditorFrameSyncTestHarness = {
    extensionArea: HTMLDivElement;
    scrollRoot: HTMLDivElement;
  };
</script>

<script lang="ts">
  import { setupAppContext, setupThemeContext } from '@typie/ui/context';
  import { elementScrollViewport, windowScrollViewport } from '@typie/ui/utils';
  import { onDestroy, untrack } from 'svelte';
  import Caret from './components/Caret.svelte';
  import DocumentOverlayLayer from './components/DocumentOverlayLayer.svelte';
  import EditorPages from './components/EditorPages.svelte';
  import Input from './components/Input.svelte';
  import LineHighlight from './components/LineHighlight.svelte';
  import Scrollbar from './components/Scrollbar.svelte';
  import SelectionHandles from './components/SelectionHandles.svelte';
  import EditorZoom from './components/ui/EditorZoom.svelte';
  import ViewportOverlay from './components/ViewportOverlay.svelte';
  import { PAGE_GAP } from './constants';
  import { setupEditorContext } from './editor.svelte';
  import { setupEditorPublication } from './editor-publication.svelte';
  import { EditorSurfaceHost } from './editor-surface-host.svelte';
  import { setupEditorScroll } from './scroll.svelte';
  import { resolveContinuousLayoutViewportWidth } from './zoom';
  import type { MouseEventHandler } from 'svelte/elements';
  import type { Editor } from './editor.svelte';
  import type { DocumentZoomLayout } from './zoom';

  type Props = {
    editor: Editor;
    onReady?: (harness: EditorFrameSyncTestHarness) => void;
    onPublishedReady?: () => void;
    onclick?: MouseEventHandler<HTMLDivElement>;
    readOnly?: boolean;
    typewriterEnabled?: boolean;
    useWindowScroll?: boolean;
    userId: string;
    withZoom?: boolean;
    headerHeight?: number;
    contentInsetLeft?: number;
    contentInsetRight?: number;
  };

  let {
    editor,
    onReady,
    onPublishedReady,
    onclick,
    readOnly = false,
    typewriterEnabled = false,
    useWindowScroll = false,
    userId,
    withZoom = false,
    headerHeight = 0,
    contentInsetLeft = 0,
    contentInsetRight = 0,
  }: Props = $props();

  const app = setupAppContext(userId);
  setupThemeContext();
  app.preference.current = { ...app.preference.current, lineHighlightEnabled: true, typewriterEnabled };

  const ctx = setupEditorContext();
  ctx.editor = editor;

  let surfaceHost = $state<EditorSurfaceHost>();

  setupEditorScroll(ctx);
  setupEditorPublication(ctx, () => surfaceHost);

  let extensionArea = $state<HTMLDivElement>();
  let scrollRoot = $state<HTMLDivElement>();
  let viewportWidth = $state(360);
  let ready = false;
  let publishedReady = false;
  const isPaginated = $derived(editor.rootAttrs?.layout_mode.type === 'paginated');
  const pageWidth = $derived(editor.pageSizes[0]?.width ?? 0);
  const zoomLayout: DocumentZoomLayout | null = $derived.by(() => {
    const layout = editor.rootAttrs?.layout_mode;
    if (layout?.type === 'continuous' && layout.max_width > 0) return { type: 'continuous', maxWidth: layout.max_width };
    if (layout?.type === 'paginated' && pageWidth > 0) return { type: 'paginated', pageWidth };
    return null;
  });

  $effect(() => {
    editor.readOnly = readOnly;
  });

  $effect(() => {
    const layout = zoomLayout;
    const width = viewportWidth;
    const renderZoom = editor.renderZoom;
    if (!withZoom || layout?.type !== 'continuous' || width <= 0) return;
    untrack(() => {
      const height = editor.viewport.height;
      if (height <= 0) return;
      editor.resizeViewportNow(
        resolveContinuousLayoutViewportWidth({ viewportWidth: width, committedZoom: renderZoom }),
        height,
        editor.scaleFactor,
      );
    });
  });

  $effect(() => {
    const revision = editor.appliedRevision;
    if (publishedReady || !editor.isPublished(revision, { requireFrame: true })) return;
    publishedReady = true;
    untrack(() => onPublishedReady?.());
  });

  $effect(() => {
    const scroll = ctx.scroll;
    if (!scroll) return;
    const host = new EditorSurfaceHost(editor, (revision) => scroll.discardFailedForRevision(revision));
    surfaceHost = host;
    return () => {
      surfaceHost = undefined;
      host.destroy();
    };
  });

  $effect(() => {
    if (!extensionArea || !scrollRoot || ready) return;
    const currentExtensionArea = extensionArea;
    const currentScrollRoot = scrollRoot;
    ready = true;
    untrack(() => onReady?.({ extensionArea: currentExtensionArea, scrollRoot: currentScrollRoot }));
  });

  onDestroy(() => {
    if (editor.extensionAreaEl === extensionArea) editor.extensionAreaEl = undefined;
    if (editor.scrollContainerEl === scrollRoot) editor.scrollContainerEl = undefined;
    if (editor.scrollRootEl === scrollRoot) editor.scrollRootEl = undefined;
    editor.scrollViewport = undefined;
  });
</script>

<svelte:window
  onscroll={useWindowScroll
    ? () => {
        ctx.scroll?.observeViewportScroll();
        editor.requestPublication();
      }
    : undefined}
/>

{#snippet editorContent()}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    bind:this={extensionArea}
    style:min-width="max-content"
    style:padding-left={`${contentInsetLeft}px`}
    style:padding-right={`${contentInsetRight}px`}
    style:padding-bottom={`${ctx.scroll?.bottomPadding ?? 0}px`}
    style="position: relative; display: flex; flex-direction: column; align-items: center; isolation: isolate; width: 100%;"
    {@attach (el) => {
      editor.extensionAreaEl = el;
    }}
    data-editor-extension-area
    {onclick}
  >
    <div
      style:row-gap={`${isPaginated ? PAGE_GAP * editor.displayZoom : 0}px`}
      style="position: relative; display: flex; flex-direction: column; align-items: center; flex: 1 0 auto; width: 100%;"
      data-editor-document-track
    >
      <EditorPages {editor} {surfaceHost} />

      <DocumentOverlayLayer />
      <Caret />
      <LineHighlight />
    </div>

    <ViewportOverlay>
      <Input />
      {#if editor.readOnly}
        <SelectionHandles />
      {/if}
    </ViewportOverlay>
  </div>
{/snippet}

<div
  bind:this={scrollRoot}
  style:height={useWindowScroll ? '4000px' : '180px'}
  style:overflow={useWindowScroll ? 'visible' : 'auto'}
  style="position: relative; width: 360px; overflow-anchor: none;"
  {@attach (el) => {
    editor.scrollContainerEl = el;
    editor.scrollViewport = useWindowScroll ? windowScrollViewport() : elementScrollViewport(el);
    editor.scrollRootEl = useWindowScroll ? null : el;
  }}
  data-editor-scroll-root
  onscroll={() => {
    ctx.scroll?.observeViewportScroll();
    editor.requestPublication();
  }}
  bind:clientWidth={viewportWidth}
>
  {#if headerHeight > 0}
    <div style:height={`${headerHeight}px`} data-editor-test-header></div>
  {/if}
  {#if withZoom}
    <EditorZoom
      active
      attachViewportAnchor={(point) => ctx.scroll?.attachViewportAnchorAt(point)}
      {editor}
      layout={zoomLayout}
      {viewportWidth}
    >
      {@render editorContent()}
    </EditorZoom>
  {:else}
    {@render editorContent()}
  {/if}
</div>

{#if withZoom}
  <Scrollbar />
{/if}
