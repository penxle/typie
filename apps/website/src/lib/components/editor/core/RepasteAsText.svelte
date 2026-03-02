<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import ClipboardTypeIcon from '~icons/lucide/clipboard-type';
  import { getEditorContext } from '$lib/editor/context.svelte';

  const { editor } = getEditorContext();

  let element = $state<HTMLButtonElement>();
  let show = $derived(editor.repasteAsTextEnabled);

  $effect(() => {
    if (!element) return;

    const { pageIdx, bounds } = editor.cursor;
    const containerEls = editor.pageContainerEls;
    const extensionAreaEl = editor.extensionArea.containerEl;
    const pageEl = containerEls[pageIdx];

    if (show && bounds && pageEl && extensionAreaEl) {
      extensionAreaEl.append(element);

      const pageRect = pageEl.getBoundingClientRect();
      const extensionRect = extensionAreaEl.getBoundingClientRect();
      const zoom = editor.layout?.layoutMode.type === 'paginated' ? editor.displayZoom : 1;

      element.style.display = 'flex';
      element.style.left = `${pageRect.left - extensionRect.left + bounds.x * zoom}px`;
      element.style.top = `${pageRect.top - extensionRect.top + (bounds.y + bounds.height) * zoom + 4}px`;
    } else {
      element.style.display = 'none';
    }
  });

  $effect(() => {
    if (show) {
      return pushEscapeHandler(() => {
        show = false;
        return true;
      });
    }
  });

  const buttonStyle = css({
    position: 'absolute',
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
