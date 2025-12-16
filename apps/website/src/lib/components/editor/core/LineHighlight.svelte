<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getAppContext } from '@typie/ui/context';
  import { getEditor } from '$lib/editor/context';

  const editor = getEditor();
  const app = getAppContext();

  const PADDING = 4;

  const show = $derived(editor.isFocused && editor.cursor.show && !!editor.cursor.bounds);
  const pageIdx = $derived(editor.cursor.pageIdx);
  const bounds = $derived(editor.cursor.bounds);

  const top = $derived(bounds ? bounds.y - PADDING : 0);
  const height = $derived(bounds ? bounds.height + PADDING * 2 : 0);
  const container = $derived(editor.pageContainerEls[pageIdx]);

  const inset = $derived(editor.layout.layoutMode.type === 'paginated' ? '0' : '-9999px');

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
    style:left={inset}
    style:right={inset}
    class={css({
      position: 'absolute',
      backgroundColor: 'surface.muted',
      zIndex: '[-1]',
      pointerEvents: 'none',
    })}
  ></div>
{/if}
