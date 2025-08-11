<script lang="ts">
  import { Slice } from '@tiptap/pm/model';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { fade } from 'svelte/transition';
  import GripVerticalIcon from '~icons/lucide/grip-vertical';
  import TextSelectIcon from '~icons/lucide/text-select';
  import { tooltip } from '../../../actions';
  import { Icon } from '../../../components';
  import { getAncestorWrappingNodeIds, unwrapWrappingNodes } from '../../extensions/clipboard';
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
  const parentWithFullContentSelected = $derived.by(() => {
    const { from, to } = editor.state.selection;
    if (from === to) return null;

    const fromPos = editor.state.doc.resolve(from);
    const toPos = editor.state.doc.resolve(to);

    if (fromPos.depth < 2 || toPos.depth < 2) return null;

    const fromParentPos = fromPos.before(fromPos.depth);
    const toParentPos = toPos.before(toPos.depth);
    if (fromParentPos !== toParentPos) return null;

    const parent = fromPos.parent;

    if (!WRAPPING_NODE_TYPES.includes(parent.type.name) && !TEXT_NODE_TYPES.includes(parent.type.name)) {
      return null;
    }

    const parentPos = fromParentPos;
    const contentStart = parentPos + 1;
    const contentEnd = parentPos + parent.nodeSize - 1;

    if (from <= contentStart && to >= contentEnd) {
      return parent;
    }

    return null;
  });

  const showUnwrap = $derived(isWrappingNode || isTextNode || !!parentWithFullContentSelected);
  const unwrapTooltip = $derived.by(() => {
    const targetNode = parentWithFullContentSelected || node;
    if (targetNode) return unwrapLabels[targetNode.type.name] || '';
    return '';
  });

  const isSelectionOverlapping = $derived.by(() => {
    const currentNode = editor.state.doc.nodeAt(pos);
    if (!currentNode) return false;

    const { from, to } = editor.state.selection;
    const nodeEnd = pos + currentNode.nodeSize;

    return from < nodeEnd && to > pos && from !== to;
  });

  const handleUnwrapClick = () => {
    const targetNode = parentWithFullContentSelected || node;
    if (!targetNode) return;

    const isTargetWrapping = WRAPPING_NODE_TYPES.includes(targetNode.type.name);
    const isTargetText = TEXT_NODE_TYPES.includes(targetNode.type.name);

    if (isTargetWrapping) {
      const targetPos = parentWithFullContentSelected
        ? editor.state.selection.from + 1 // 선택 영역 내부로 이동
        : pos + 1; // 노드 내부로 이동

      editor.chain().focus().setTextSelection(targetPos).unwrapNode(targetNode.type.name).run();
    } else if (isTargetText) {
      const targetPos = parentWithFullContentSelected
        ? editor.state.doc.resolve(editor.state.selection.from).before(editor.state.doc.resolve(editor.state.selection.from).depth)
        : pos;

      editor.chain().focus().setNodeSelection(targetPos).setNode('paragraph').run();
    }
  };

  const updateSelectionIfNeeded = () => {
    const node = editor.state.doc.nodeAt(pos);
    if (!node) return false;

    const { from, to } = editor.state.selection;
    const nodeEnd = pos + node.nodeSize;

    const isNodeSelection = from === pos && to === nodeEnd;
    // NOTE: 이 노드가 현재 selection을 포함하거나 노드 자체 선택인 경우 selection 유지
    if (!isSelectionOverlapping && !isNodeSelection) {
      editor.chain().setNodeSelection(pos).focus().run();
    }

    return true;
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

    editor.view.dragging = {
      slice: editor.state.selection.content(),
      move: true,
    };

    if (!updateSelectionIfNeeded()) {
      editor.view.dragging = null;
      return;
    }

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

    // NOTE: 선택이 업데이트된 후 slice를 다시 가져옴
    let slice = editor.state.selection.content();

    // NOTE: 텍스트 노드 내부의 텍스트를 선택한 경우, 텍스트만 추출
    const { from } = editor.state.selection;
    const fromPos = editor.state.doc.resolve(from);
    const parent = fromPos.parent;

    // NOTE: cut, copy 할 때와 다르게 드래그 할 때는 재귀적으로 wrapping node 제거해 줘야 함
    const wrappingNodeIds = getAncestorWrappingNodeIds(editor.state.selection);

    if (TEXT_NODE_TYPES.includes(parent.type.name)) {
      const textContent = slice.content.textBetween(0, slice.content.size, '\n');
      const schema = editor.state.schema;
      const paragraph = schema.nodes.paragraph.create(null, schema.text(textContent));
      slice = new Slice(paragraph.content, 0, 0);
    } else if (wrappingNodeIds.size > 0) {
      const unwrappedFragment = unwrapWrappingNodes(slice.content, wrappingNodeIds);
      slice = new Slice(unwrappedFragment, 0, 0);
    }

    editor.view.dragging.slice = slice;
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
    use:tooltip={{ message: isSelectionOverlapping ? '드래그하여 선택 영역 이동' : '선택 또는 드래그하여 이동', placement: 'top' }}
  >
    <Icon icon={GripVerticalIcon} size={18} />
  </button>
</div>
