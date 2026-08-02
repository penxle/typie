<script lang="ts" module>
  export type EditorFrameSyncTestHarness = {
    extensionArea: HTMLDivElement;
    scrollRoot: HTMLDivElement;
  };
</script>

<script lang="ts">
  import { setupAppContext } from '@typie/ui/context';
  import { elementScrollViewport } from '@typie/ui/utils';
  import { onDestroy, untrack } from 'svelte';
  import Caret from './components/Caret.svelte';
  import DocumentOverlayLayer from './components/DocumentOverlayLayer.svelte';
  import EditorPages from './components/EditorPages.svelte';
  import Input from './components/Input.svelte';
  import LineHighlight from './components/LineHighlight.svelte';
  import SelectionHandles from './components/SelectionHandles.svelte';
  import ViewportOverlay from './components/ViewportOverlay.svelte';
  import { PAGE_GAP } from './constants';
  import { setupEditorContext } from './editor.svelte';
  import { setupEditorScroll } from './scroll.svelte';
  import type { Editor } from './editor.svelte';

  type Props = {
    editor: Editor;
    onReady?: (harness: EditorFrameSyncTestHarness) => void;
    readOnly?: boolean;
    typewriterEnabled?: boolean;
    userId: string;
  };

  let { editor, onReady, readOnly = false, typewriterEnabled = false, userId }: Props = $props();

  const app = setupAppContext(userId);
  app.preference.current = { ...app.preference.current, lineHighlightEnabled: true, typewriterEnabled };

  const ctx = setupEditorContext();
  ctx.editor = editor;
  setupEditorScroll(ctx);

  let extensionArea = $state<HTMLDivElement>();
  let scrollRoot = $state<HTMLDivElement>();
  let ready = false;

  $effect(() => {
    editor.readOnly = readOnly;
  });

  $effect(() => editor.activateVisualHost());

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

<div
  bind:this={scrollRoot}
  style="position: relative; width: 360px; height: 180px; overflow: auto;"
  {@attach (el) => {
    editor.scrollContainerEl = el;
    editor.scrollViewport = elementScrollViewport(el);
    editor.scrollRootEl = el;
  }}
  data-editor-scroll-root
>
  <div
    bind:this={extensionArea}
    style:padding-bottom={`${ctx.scroll?.bottomPadding ?? 0}px`}
    style:row-gap={`${PAGE_GAP}px`}
    style="position: relative; display: flex; flex-direction: column; align-items: center; min-width: max-content;"
    {@attach (el) => {
      editor.extensionAreaEl = el;
    }}
    data-editor-extension-area
  >
    <EditorPages {editor} />

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
</div>
