<script lang="ts">
  import { fade } from 'svelte/transition';
  import GripVerticalIcon from '~icons/lucide/grip-vertical';
  import TextSelectIcon from '~icons/lucide/text-select';
  import { tooltip } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import { TEXT_NODE_TYPES, WRAPPING_NODE_NAMES } from '../../extensions/node-commands';
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
  const isWrappingNode = $derived(node && WRAPPING_NODE_NAMES.includes(node.type.name));
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

  const handleGripClick = () => {
    const node = editor.state.doc.nodeAt(pos);
    if (!node) {
      return;
    }

    editor.chain().setNodeSelection(pos).focus().run();
  };

  const handleDragStart = (event: DragEvent) => {
    if (!event.dataTransfer) {
      return;
    }

    event.dataTransfer.clearData();
    event.dataTransfer.effectAllowed = 'move';

    const domNode = editor.view.nodeDOM(pos) as HTMLElement;
    if (domNode) {
      event.dataTransfer.setDragImage(domNode, 0, 0);
    }

    editor.chain().setNodeSelection(pos).focus().run();

    editor.view.dragging = {
      slice: editor.state.selection.content(),
      move: true,
    };
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
    ondragstart={handleDragStart}
    type="button"
    use:tooltip={{ message: '선택 또는 드래그하여 이동', placement: 'top' }}
  >
    <Icon icon={GripVerticalIcon} size={18} />
  </button>
</div>
