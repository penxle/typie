<script lang="ts">
  import { fade } from 'svelte/transition';
  import GripVerticalIcon from '~icons/lucide/grip-vertical';
  import PlusIcon from '~icons/lucide/plus';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { Editor } from '@tiptap/core';

  type Props = {
    editor: Editor;
    pos: number;
  };

  let { editor, pos }: Props = $props();

  const handlePlusClick = () => {
    const node = editor.state.doc.nodeAt(pos);
    if (!node) {
      return;
    }

    if (node.type.name === 'paragraph' && node.childCount === 0) {
      editor
        .chain()
        .focus(pos + 1)
        .run();
    } else {
      editor
        .chain()
        .insertContentAt(pos + node.nodeSize, { type: 'paragraph' })
        .focus(pos + node.nodeSize + 1)
        .run();
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
  <button
    class={css({ borderRadius: '6px', padding: '2px', color: 'text.faint', _hover: { backgroundColor: 'interactive.hover' } })}
    onclick={handlePlusClick}
    type="button"
  >
    <Icon icon={PlusIcon} size={18} />
  </button>

  <button
    class={css({ borderRadius: '6px', padding: '2px', color: 'text.faint', _hover: { backgroundColor: 'interactive.hover' } })}
    draggable="true"
    onclick={handleGripClick}
    ondragstart={handleDragStart}
    type="button"
  >
    <Icon icon={GripVerticalIcon} size={18} />
  </button>
</div>
