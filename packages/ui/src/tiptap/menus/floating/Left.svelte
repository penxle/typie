<script lang="ts">
  import { Slice } from '@tiptap/pm/model';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { fade } from 'svelte/transition';
  import GripVerticalIcon from '~icons/lucide/grip-vertical';
  import TextSelectIcon from '~icons/lucide/text-select';
  import { tooltip } from '../../../actions';
  import { Icon } from '../../../components';
  import { getWrappingNodeId, unwrapNodeById } from '../../extensions/clipboard';
  import { TEXT_NODE_TYPES, WRAPPING_NODE_TYPES } from '../../extensions/node-commands';
  import { Blockquote, Callout, CodeBlock, Fold, HtmlBlock } from '../../node-views';
  import type { Editor } from '@tiptap/core';

  type Props = {
    editor: Editor;
    pos: number;
  };

  let { editor, pos }: Props = $props();

  const unwrapLabels = {
    [Blockquote.name]: '인용구 해제',
    [Callout.name]: '콜아웃 해제',
    [Fold.name]: '폴드 해제',
    [CodeBlock.name]: '코드 해제',
    [HtmlBlock.name]: 'HTML 해제',
  };

  const node = $derived(editor.state.doc.nodeAt(pos));
  const isWrappingNode = $derived(node && WRAPPING_NODE_TYPES.includes(node.type.name));
  const isTextNode = $derived(node && TEXT_NODE_TYPES.includes(node.type.name));
  const showUnwrap = $derived(isWrappingNode || isTextNode);
  const unwrapTooltip = $derived(node ? unwrapLabels[node.type.name] || '' : '');

  const handleUnwrapClick = () => {
    if (!node) return;

    if (isWrappingNode) {
      editor
        .chain()
        .focus()
        .setNodeSelection(pos + 1) // NOTE: 현재 노드 내부에서 unwrap 실행되도록 +1
        .unwrapNode(node.type.name)
        .run();
    } else if (isTextNode) {
      editor.chain().focus().setNodeSelection(pos).setNode('paragraph').run();
    }
  };

  const updateSelectionIfNeeded = () => {
    const node = editor.state.doc.nodeAt(pos);
    if (!node) return false;

    const { from, to } = editor.state.selection;
    const nodeEnd = pos + node.nodeSize;

    const isSelectionOverlapping = from < nodeEnd && to > pos && from !== to;
    // NOTE: 이 노드가 현재 selection을 포함하는 경우 selection 유지
    if (!isSelectionOverlapping) {
      editor.chain().setNodeSelection(pos).focus().run();
    }

    return { node, isSelectionOverlapping };
  };

  const handleGripClick = () => {
    updateSelectionIfNeeded();
  };

  const handleDragStart = (event: DragEvent) => {
    if (!event.dataTransfer) {
      return;
    }

    event.dataTransfer.clearData();
    event.dataTransfer.effectAllowed = 'move';

    const result = updateSelectionIfNeeded();
    if (!result) {
      return;
    }

    const { isSelectionOverlapping } = result;

    if (isSelectionOverlapping) {
      // NOTE: 텍스트 선택이 있는 경우, 선택 영역의 DOM 복사본을 드래그 이미지로 사용
      const selection = window.getSelection();
      if (!selection || selection.rangeCount === 0) return;

      const range = selection.getRangeAt(0);
      const contents = range.cloneContents();

      const dragImage = document.createElement('div');
      dragImage.style.position = 'absolute';
      dragImage.style.top = '0';
      dragImage.style.left = '-9999px';
      dragImage.append(contents);
      document.body.append(dragImage);

      event.dataTransfer.setDragImage(dragImage, 20, 20);

      const cleanup = () => dragImage.remove();
      setTimeout(cleanup, 0);
      editor.view.dom.addEventListener('dragend', cleanup, { once: true });
    } else {
      const domNode = editor.view.nodeDOM(pos);
      if (!(domNode instanceof HTMLElement)) return;
      event.dataTransfer.setDragImage(domNode, 0, 0);
    }

    let slice = editor.state.selection.content();
    const wrappingNodeId = getWrappingNodeId(editor.state.selection);

    if (wrappingNodeId) {
      const unwrappedFragment = unwrapNodeById(slice.content, wrappingNodeId);
      slice = new Slice(unwrappedFragment, slice.openStart, slice.openEnd);
    }

    editor.view.dragging = {
      slice,
      move: true,
    };
  };

  const handleDragEnd = () => {
    if (editor.view.dragging) {
      editor.view.dragging = null;
    }
  };
</script>

<div class={flex({ align: 'center' })} transition:fade={{ duration: 100 }}>
  {#if showUnwrap}
    <button
      class={css({ borderRadius: '6px', padding: '2px', color: 'text.faint', _hover: { backgroundColor: 'interactive.hover' } })}
      onclick={handleUnwrapClick}
      type="button"
      use:tooltip={{ message: unwrapTooltip, placement: 'top' }}
    >
      <Icon icon={TextSelectIcon} size={18} />
    </button>
  {/if}

  <button
    class={css({ borderRadius: '6px', padding: '2px', color: 'text.faint', _hover: { backgroundColor: 'interactive.hover' } })}
    draggable="true"
    onclick={handleGripClick}
    ondragend={handleDragEnd}
    ondragstart={handleDragStart}
    type="button"
    use:tooltip={{ message: '선택 또는 드래그하여 이동', placement: 'top' }}
  >
    <Icon icon={GripVerticalIcon} size={18} />
  </button>
</div>
