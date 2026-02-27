<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getAppContext } from '@typie/ui/context';
  import { getEditorContext } from '$lib/editor/context.svelte';

  const { editor } = getEditorContext();
  const app = getAppContext();

  const PADDING = 4;

  const show = $derived(editor.isFocused && editor.cursor.visible && !!editor.cursor.bounds);
  const pageIdx = $derived(editor.cursor.pageIdx);
  const bounds = $derived(editor.cursor.bounds);

  const isPaginated = $derived(editor.layout?.layoutMode.type === 'paginated');

  const top = $derived.by(() => {
    if (!bounds) return 0;
    if (isPaginated) {
      return bounds.y - PADDING;
    }
    const pageEl = editor.pageContainerEls[pageIdx];
    const wrapperEl = pageEl?.parentElement;
    return (wrapperEl?.offsetTop ?? 0) + bounds.y - PADDING;
  });

  const height = $derived(bounds ? bounds.height + PADDING * 2 : 0);

  const container = $derived(isPaginated ? editor.pageContainerEls[pageIdx] : editor.extensionArea.containerEl);

  let element = $state<HTMLDivElement>();

  $effect(() => {
    if (show && container && element && element.parentElement !== container) {
      container.append(element);
    }
  });
</script>

{#if app.preference.current.lineHighlightEnabled}
  <div
    bind:this={element}
    style:display={show ? 'block' : 'none'}
    style:top={`${top}px`}
    style:height={`${height}px`}
    class={css({
      position: 'absolute',
      backgroundColor: 'surface.muted',
      insetX: '0',
      zIndex: '[-1]',
      pointerEvents: 'none',
    })}
  ></div>
{/if}
