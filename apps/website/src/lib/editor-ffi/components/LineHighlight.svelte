<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getAppContext } from '@typie/ui/context';
  import { getEditorContext } from '../editor.svelte';

  const { editor } = getEditorContext();
  const app = getAppContext();

  const PADDING = 4;

  const show = $derived(!!editor?.focused && !!editor?.cursor);

  const isPaginated = $derived(editor?.documentAttrs?.layout_mode.type === 'paginated');

  const container = $derived(
    editor?.cursor ? (isPaginated ? editor.pageEls[editor.cursor.page_idx] : editor.scrollContainerEl) : undefined,
  );

  const top = $derived.by(() => {
    if (!editor?.cursor) return 0;
    if (isPaginated) {
      return editor.cursor.rect.y - PADDING;
    }
    const offset = editor.localToOffset(editor.cursor.page_idx, 0, editor.cursor.rect.y);
    return (offset?.y ?? 0) - PADDING;
  });

  const height = $derived(editor?.cursor ? editor.cursor.rect.height + PADDING * 2 : 0);

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
