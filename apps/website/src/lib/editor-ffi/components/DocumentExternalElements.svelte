<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { SvelteSet } from 'svelte/reactivity';
  import { PAGE_GAP } from '../constants';
  import { resolveCachedPageSpans, roundToScale } from '../geometry';
  import ExternalElement from './ExternalElement.svelte';
  import type { Editor } from '../editor.svelte';

  type Props = {
    editor: Editor;
  };

  let { editor }: Props = $props();

  // Embeds mount independently of page frames. Images only outlive their
  // rendered pages while an interaction or its publication is still active.
  const keepMountedImageNodes = new SvelteSet<string>();
  const layoutMode = $derived(editor.rootAttrs?.layout_mode);
  const isPaginated = $derived(layoutMode?.type === 'paginated');
  const displayZoom = $derived(editor.safeDisplayZoom());
  const pageSpans = $derived(
    resolveCachedPageSpans(editor.pageSizes, {
      displayZoom,
      scaleFactor: editor.scaleFactor,
      pageGap: isPaginated ? PAGE_GAP * displayZoom : 0,
    }),
  );
  const documentElements = $derived.by(() => {
    void editor.publishedRevision;
    return editor.externalElements.filter((element) => element.data.type === 'embed' || element.data.type === 'image');
  });
  const elements = $derived(
    documentElements.filter(
      (element) =>
        element.data.type === 'embed' ||
        keepMountedImageNodes.has(element.node) ||
        editor.pageExternalElements(element.page_idx).some((candidate) => candidate.node === element.node),
    ),
  );

  $effect(() => {
    const documentNodes = new Set(documentElements.map((element) => element.node));
    for (const node of keepMountedImageNodes) {
      if (documentNodes.has(node)) continue;
      keepMountedImageNodes.delete(node);
    }
  });
</script>

<div class={css({ position: 'absolute', inset: '0', pointerEvents: 'none' })}>
  {#each elements as element (element.node)}
    {@const size = editor.pageSizes[element.page_idx]}
    {@const pageSpan = pageSpans[element.page_idx]}
    {#if size && pageSpan}
      {@const pagePresented = editor.published?.frames.has(element.page_idx) === true}
      {@const scaleFactor = editor.scaleFactor}
      {@const cssWidth = roundToScale(size.width, scaleFactor)}
      {@const cssHeight = roundToScale(size.height, scaleFactor)}
      {@const slotWidth = roundToScale(size.width * displayZoom, scaleFactor)}
      <div
        style:top={`${pageSpan.top}px`}
        style:left="50%"
        style:width={`${slotWidth}px`}
        style:visibility={pagePresented ? 'visible' : 'hidden'}
        style:pointer-events={pagePresented ? undefined : 'none'}
        class={css({ position: 'absolute', transform: 'translateX(-50%)' })}
        aria-hidden={!pagePresented}
        data-document-embed={element.data.type === 'embed' ? true : undefined}
        inert={!pagePresented}
      >
        <div
          style:width={`${cssWidth}px`}
          style:height={`${cssHeight}px`}
          style:transform={displayZoom === 1 ? undefined : `scale(${displayZoom})`}
          style:transform-origin={displayZoom === 1 ? undefined : 'top left'}
          class={css({ position: 'relative' })}
        >
          <ExternalElement
            {element}
            onKeepMountedChange={(keepMounted) => {
              if (keepMounted) keepMountedImageNodes.add(element.node);
              else keepMountedImageNodes.delete(element.node);
            }}
          />
        </div>
      </div>
    {/if}
  {/each}
</div>
