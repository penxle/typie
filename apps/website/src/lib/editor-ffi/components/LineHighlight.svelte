<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { tryAppContext } from '@typie/ui/context';
  import { getEditorContext } from '../editor.svelte';
  import { presentedPageElement, resolveCachedPageSpans } from '../geometry';

  const { editor } = getEditorContext();
  const app = tryAppContext();

  const cursor = $derived.by(() => {
    const current = editor?.cursor;
    return current && editor && presentedPageElement(editor, current.page_idx) ? current : undefined;
  });
  const show = $derived(!!editor?.focused && !!cursor);
  // app is absent in the public viewer (no AppContext provider); fall back to off.
  const lineHighlightEnabled = $derived(app?.preference.current.lineHighlightEnabled ?? false);

  const isPaginated = $derived(editor?.rootAttrs?.layout_mode.type === 'paginated');
  const displayZoom = $derived(editor?.safeDisplayZoom() ?? 1);
  const scaleFactor = $derived(editor?.scaleFactor ?? 1);
  const pageSpans = $derived(!isPaginated && editor ? resolveCachedPageSpans(editor.pageSizes, { displayZoom, scaleFactor }) : []);

  const container = $derived.by(() => {
    if (!cursor || !editor) return;
    if (isPaginated) return presentedPageElement(editor, cursor.page_idx);
    return editor.extensionAreaEl;
  });

  const top = $derived.by(() => {
    if (!cursor || !editor) return 0;
    if (isPaginated) return cursor.line.y;
    const pageTop = pageSpans[cursor.page_idx]?.top ?? 0;
    return pageTop + cursor.line.y * displayZoom;
  });

  const height = $derived((cursor?.line.height ?? 0) * (isPaginated ? 1 : displayZoom));

  let element = $state<HTMLDivElement>();

  $effect(() => {
    if (show && container && element && element.parentElement !== container) {
      container.append(element);
    }
  });
</script>

{#if lineHighlightEnabled}
  <div
    bind:this={element}
    style:display={show ? 'block' : 'none'}
    style:top={`${top}px`}
    style:height={`${height}px`}
    class={css({
      position: 'absolute',
      backgroundColor: { base: 'text.default/4', _dark: 'text.default/10' },
      insetX: '0',
      zIndex: '[-1]',
      pointerEvents: 'none',
    })}
    data-editor-line-highlight
  ></div>
{/if}
