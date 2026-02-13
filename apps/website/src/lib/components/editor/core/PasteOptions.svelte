<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import ClipboardTypeIcon from '~icons/lucide/clipboard-type';
  import { getEditorContext } from '$lib/editor/context.svelte';

  const { editor } = getEditorContext();

  let element = $state<HTMLButtonElement>();
  const show = $derived(!!editor.pasteOptions);

  $effect(() => {
    if (!element) return;

    const { pageIdx, bounds } = editor.cursor;
    const containerEls = editor.pageContainerEls;

    if (show && bounds && containerEls[pageIdx]) {
      containerEls[pageIdx].append(element);

      element.style.display = 'flex';
      element.style.left = `${bounds.x}px`;
      element.style.top = `${bounds.y + bounds.height + 4}px`;
    } else {
      element.style.display = 'none';
    }
  });

  $effect(() => {
    if (show) {
      return pushEscapeHandler(() => {
        editor.pasteOptions = null;
        return true;
      });
    }
  });

  const buttonStyle = css({
    position: 'absolute',
    zIndex: '1',
    display: 'flex',
    alignItems: 'center',
    gap: '4px',
    height: '28px',
    paddingX: '8px',
    backgroundColor: 'surface.default',
    border: '1px solid',
    borderColor: 'border.subtle',
    borderRadius: '6px',
    boxShadow: 'small',
    fontSize: '13px',
    fontWeight: 'medium',
    color: 'text.subtle',
    cursor: 'pointer',
    transition: 'colors',
    userSelect: 'none',
    whiteSpace: 'nowrap',
    _hover: {
      backgroundColor: 'surface.subtle',
      color: 'text.default',
      borderColor: 'border.default',
    },
  });
</script>

<button
  bind:this={element}
  class={buttonStyle}
  data-external-element
  onclick={(e) => {
    e.stopPropagation();
    editor.handleRepasteAsText();
  }}
  onpointerdown={(e) => {
    e.stopPropagation();
  }}
  type="button"
>
  <ClipboardTypeIcon />
  <span>서식 없이 다시 붙여넣기</span>
</button>
