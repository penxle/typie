<script lang="ts">
  import { onMount } from 'svelte';
  import BoldIcon from '~icons/lucide/bold';
  import ItalicIcon from '~icons/lucide/italic';
  import StrikethroughIcon from '~icons/lucide/strikethrough';
  import UnderlineIcon from '~icons/lucide/underline';
  import { Icon } from '$lib/components';
  import { flex } from '$styled-system/patterns';
  import type { Editor } from '@tiptap/core';
  import type { Node } from '@tiptap/pm/model';

  type Props = {
    editor: Editor;
  };

  let { editor }: Props = $props();

  const marks = [
    { name: 'bold', icon: BoldIcon },
    { name: 'italic', icon: ItalicIcon },
    { name: 'underline', icon: UnderlineIcon },
    { name: 'strike', icon: StrikethroughIcon },
  ];

  let activeMarks = $state<string[]>([]);
  let activeNode = $state<Node | null>(null);
  let selectedBlocks = $state<Node[]>([]);
  let isInlineContentSelected = $state(false);

  const showMarksMenu = $derived(isInlineContentSelected);
  const isInCodeblock = $derived(activeNode?.type.name === 'code_block');
  const showBubbleMenu = $derived(!isInCodeblock && showMarksMenu);

  const bubbleMenuButtonStyle = flex({
    alignItems: 'center',
    justifyContent: 'center',
    borderRadius: '4px',
    width: '30px',
    height: '30px',
    _hover: {
      backgroundColor: 'gray.200',
    },
    _pressed: {
      color: 'brand.400',
      '& *': { strokeWidth: '[2.5]' },
    },
    _active: {
      backgroundColor: 'gray.300',
    },
  });

  const updateSelectedNodeAndMarks = () => {
    activeMarks = marks.map(({ name }) => name).filter((name) => editor.isActive(name));
    activeNode = editor.state.selection.$head.parent;

    selectedBlocks = [];
    isInlineContentSelected = false;

    const { from, to } = editor.state.selection;
    if (from !== null && to !== null) {
      editor.state.doc.nodesBetween(from, to, (node) => {
        if (node.isBlock) {
          selectedBlocks.push(node);
        }

        if (node.inlineContent && node.content.size > 0) {
          isInlineContentSelected = true;
        }
      });
    }
  };

  onMount(() => {
    editor.on('update', updateSelectedNodeAndMarks);
    editor.on('selectionUpdate', updateSelectedNodeAndMarks);

    return () => {
      editor?.off('update', updateSelectedNodeAndMarks);
      editor?.off('selectionUpdate', updateSelectedNodeAndMarks);
    };
  });
</script>

<div
  class={flex({
    alignItems: 'center',
    gap: '2px',
    borderWidth: '1px',
    borderColor: 'gray.200',
    borderRadius: '10px',
    padding: '4px',
    backgroundColor: 'gray.100',
    height: '42px',
    boxShadow: 'xlarge',
    zIndex: '20',
  })}
  hidden={!showBubbleMenu}
>
  {#if showMarksMenu}
    {#each marks as { name, icon } (name)}
      <button
        class={bubbleMenuButtonStyle}
        aria-pressed={activeMarks.includes(name)}
        onclick={() => {
          editor.chain().focus().toggleMark(name).run();
        }}
        type="button"
      >
        <Icon {icon} size={16} />
      </button>
    {/each}
  {/if}
</div>
