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
  import SelectionHandles from './components/SelectionHandles.svelte';
  import EditorZoom from './components/ui/EditorZoom.svelte';
  import ViewportOverlay from './components/ViewportOverlay.svelte';
  import { PAGE_GAP } from './constants';
  import { setupEditorContext } from './editor.svelte';
  import { setupEditorPublication } from './editor-publication.svelte';
  import { EditorSurfaceHost } from './editor-surface-host.svelte';
  import { setupEditorScroll } from './scroll.svelte';
  import type { MouseEventHandler } from 'svelte/elements';
  import type { Editor } from './editor.svelte';

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

  $effect(() => {
    editor.readOnly = readOnly;
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
    style:padding-bottom={`${ctx.scroll?.bottomPadding ?? 0}px`}
    style:row-gap={`${isPaginated ? PAGE_GAP * editor.displayZoom : 0}px`}
    style="position: relative; display: flex; flex-direction: column; align-items: center; min-width: max-content;"
    {@attach (el) => {
      editor.extensionAreaEl = el;
    }}
    data-editor-extension-area
    {onclick}
  >
    <EditorPages {editor} {surfaceHost} />

    <DocumentOverlayLayer />
    <Caret />
    <LineHighlight />

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
  onscroll={() => editor.requestPublication()}
  bind:clientWidth={viewportWidth}
>
  {#if withZoom}
    <EditorZoom active {editor} {isPaginated} {pageWidth} {viewportWidth}>
      {@render editorContent()}
    </EditorZoom>
  {:else}
    {@render editorContent()}
  {/if}
</div>
