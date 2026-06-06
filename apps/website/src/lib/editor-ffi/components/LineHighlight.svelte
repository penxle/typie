<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getAppContext } from '@typie/ui/context';
  import { getEditorContext } from '../editor.svelte';

  const { editor } = getEditorContext();
  const app = getAppContext();

  const show = $derived(!!editor?.focused && !!editor?.cursor);
  // app is absent in the public viewer (no AppContext provider); fall back to off.
  const lineHighlightEnabled = $derived(app?.preference.current.lineHighlightEnabled ?? false);

  const isPaginated = $derived(editor?.rootAttrs?.layout_mode.type === 'paginated');

  const container = $derived(
    editor?.cursor ? (isPaginated ? editor.pageEls[editor.cursor.page_idx] : editor.scrollContainerEl) : undefined,
  );

  const top = $derived.by(() => {
    if (!editor?.cursor) return 0;
    if (isPaginated) {
      return editor.cursor.line.y;
    }
    const offset = editor.localToOffset(editor.cursor.page_idx, 0, editor.cursor.line.y);
    return offset?.y ?? 0;
  });

  const height = $derived(editor?.cursor ? editor.cursor.line.height : 0);

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
      backgroundColor: 'surface.muted',
      insetX: '0',
      zIndex: '[-1]',
      pointerEvents: 'none',
    })}
  ></div>
{/if}
