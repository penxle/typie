<script lang="ts">
  import { untrack } from 'svelte';
  import { PAGE_GAP } from '../constants';
  import { resolveCachedPageSpans } from '../geometry';
  import { readNativeSelection } from '../native-selection';
  import { layoutSelectionBlocks, renderSelectionText } from '../native-selection-dom';
  import { fitSelectionRuns, loadSelectionFonts, observeNativeSelection } from '../native-selection-layout';
  import ExternalElement from './ExternalElement.svelte';
  import NativeSelectionImage from './NativeSelectionImage.svelte';
  import type { Editor } from '../editor.svelte';

  let { editor }: { editor: Editor } = $props();
  let root = $state<HTMLDivElement>();
  let fontsReady = $state(false);
  const blocks = $derived(editor.published?.snapshot.selectionLayout ?? []);
  const zoom = $derived(editor.safeDisplayZoom());
  const width = $derived(editor.pageSizes[0]?.width ?? 0);
  const pages = $derived(
    resolveCachedPageSpans(editor.pageSizes, {
      displayZoom: zoom,
      scaleFactor: editor.scaleFactor,
      pageGap: editor.rootAttrs?.layout_mode.type === 'paginated' ? PAGE_GAP * zoom : 0,
    }),
  );
  // Keep this getter in its own derived value: unchanged external elements
  // must not rebuild selection DOM on viewport-only publications.
  const externalElements = $derived(editor.externalElements);
  const blockLayouts = $derived(layoutSelectionBlocks(blocks, externalElements, pages, zoom, width));

  $effect(() => {
    void blocks;
    // A new displayed layout may invalidate positions. Ordinary scrolling does
    // not replace this snapshot or any of the semantic DOM nodes.
    untrack(() => {
      const selection = window.getSelection();
      if (root && selection?.rangeCount && selection.getRangeAt(0).intersectsNode(root)) selection.removeAllRanges();
    });
  });

  $effect(() => {
    const element = root;
    if (!element) return;
    const copy = (event: ClipboardEvent) => {
      const range = readNativeSelection(element, blocks);
      if (!range) return;
      event.preventDefault();
      if (editor.protectContent || !editor.isPublished(editor.appliedRevision)) return;
      const payload = editor.copySelection(range);
      if (!payload || !event.clipboardData) return;
      event.clipboardData.setData('text/html', payload.html);
      event.clipboardData.setData('text/plain', payload.text);
    };
    element.ownerDocument.addEventListener('copy', copy);
    return () => element.ownerDocument.removeEventListener('copy', copy);
  });

  $effect(() => {
    const element = root;
    const layout = blocks;
    const pageSpans = pages;
    if (!element) return;
    let active = true;
    let dispose: (() => void) | undefined;
    fontsReady = false;
    void untrack(() => loadSelectionFonts(editor, layout))
      .then(() => {
        if (!active) return;
        fitSelectionRuns(element, layout, pageSpans);
        dispose = observeNativeSelection(element, editor, layout);
        fontsReady = true;
      })
      .catch((err: unknown) => {
        console.error('Could not prepare native selection fonts', err);
      });
    return () => {
      active = false;
      dispose?.();
    };
  });
</script>

<div
  bind:this={root}
  style:width={`${width}px`}
  style:height={`${(pages.at(-1)?.bottom ?? 0) / zoom}px`}
  style:transform={`translateX(-50%) scale(${zoom})`}
  class="native-selection-layer notranslate"
  class:fonts-ready={fontsReady}
  data-native-selection-layer
  translate="no"
>
  {#each blockLayouts as block, index (block.node)}
    <div
      style:margin-top={`${block.marginTop}px`}
      style:height={`${block.height}px`}
      style:width={`${width}px`}
      class="selection-block"
      data-selection-block={index}
    >
      {#if block.fragments.length > 0}
        <span class="selection-text-block" {@attach (element) => renderSelectionText(element, block)}></span>
      {:else if block.external}
        {@const { element, top } = block.external}
        <div style:left="0px" style:top={`${top}px`} class="selection-external" class:selectable={element.data.type !== 'image'}>
          {#if element.data.type === 'image'}
            <NativeSelectionImage {editor} {element} />
          {:else}
            <ExternalElement {element} />
          {/if}
        </div>
      {/if}
    </div>
  {/each}
</div>

<style>
  .native-selection-layer {
    z-index: 1;
    position: absolute;
    display: flex;
    flex-direction: column;
    top: 0;
    left: 50%;
    transform-origin: top center;
    pointer-events: auto;
    user-select: text;
    -webkit-user-select: text;
    -webkit-touch-callout: default;
    text-size-adjust: none;
    -webkit-text-size-adjust: none;
  }
  .selection-block {
    position: relative;
    display: block;
    flex-shrink: 0;
    pointer-events: none;
  }
  .selection-text-block {
    display: contents;
    font-size: 0;
    line-height: 0;
  }
  .native-selection-layer :global(.selection-fragment) {
    display: inline-block;
    vertical-align: top;
    pointer-events: auto;
  }
  .native-selection-layer :global(.selection-line) {
    /* Text in adjacent table cells must win over another fragment's margins. */
    z-index: 1;
    /* In-flow inline boxes preserve soft-wrap text and let native hit testing
       distinguish the beginning and end of a paragraph from its blank space. */
    position: relative;
    display: inline-block;
    pointer-events: auto;
    vertical-align: top;
    white-space: pre;
    line-height: 1;
    font-size: 0;
    cursor: text;
  }
  .native-selection-layer :global(.selection-text) {
    visibility: hidden;
    position: relative;
    display: inline-block;
    vertical-align: top;
    min-width: 1px;
    color: transparent;
    -webkit-text-fill-color: transparent;
    white-space: pre;
    transform-origin: 0 0;
    pointer-events: auto;
    cursor: text;
    user-select: text;
    -webkit-user-select: text;
    text-decoration: none;
    letter-spacing: normal;
    font-feature-settings: normal;
    font-kerning: normal;
    text-rendering: auto;
  }
  .fonts-ready :global(.selection-text) {
    visibility: visible;
  }
  .native-selection-layer :global(a.selection-text[href]) {
    cursor: pointer;
  }
  .selection-external {
    z-index: 2;
    position: absolute;
    left: 0;
    width: 100%;
  }
  .selectable :global([data-external-element]) {
    user-select: text;
    -webkit-user-select: text;
  }
  .native-selection-layer :global([data-selection-label]) {
    user-select: text;
    -webkit-user-select: text;
  }
</style>
