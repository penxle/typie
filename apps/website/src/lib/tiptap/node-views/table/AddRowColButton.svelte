<script lang="ts">
  import { TableMap } from '@tiptap/pm/tables';
  import PlusIcon from '~icons/lucide/plus';
  import { Icon } from '$lib/components';
  import { css, cx } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
  import type { Editor } from '@tiptap/core';
  import type { Node } from '@tiptap/pm/model';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor?: Ref<Editor>;
    tableNode: Node;
    getPos: () => number | undefined;
    isLastRowHovered: boolean;
    isLastColumnHovered: boolean;
  };

  let { editor, tableNode, getPos, isLastRowHovered, isLastColumnHovered }: Props = $props();

  function addRowAtEnd(tableNode: Node) {
    if (!editor) {
      return;
    }

    const { state } = editor.current;
    const { tr } = state;

    const map = TableMap.get(tableNode);
    const lastRowIndex = map.height - 1;
    const tablePos = getPos();
    if (tablePos === undefined) {
      return;
    }

    const tableStart = tablePos + 1;

    const cellPos = map.positionAt(lastRowIndex, 0, tableNode);
    const cellResolvedPos = tr.doc.resolve(tableStart + cellPos);

    editor.current.commands.setTextSelection(cellResolvedPos.pos);

    const result = editor.current.commands.addRowAfter();

    return result;
  }

  function addColumnAtEnd(tableNode: Node) {
    if (!editor) {
      return;
    }

    const { state } = editor.current;
    const { tr } = state;

    const map = TableMap.get(tableNode);
    const lastColumnIndex = map.width - 1;
    const tablePos = getPos();
    if (tablePos === undefined) {
      return;
    }

    const tableStart = tablePos + 1;

    const cellPos = map.positionAt(0, lastColumnIndex, tableNode);
    const cellResolvedPos = tr.doc.resolve(tableStart + cellPos);

    editor.current.commands.setTextSelection(cellResolvedPos.pos);

    const result = editor.current.commands.addColumnAfter();

    return result;
  }
</script>

<div
  class={cx(
    'group',
    css({
      position: 'absolute',
      zIndex: '10',
      left: '0',
      bottom: '0',
      right: '0',
      width: 'full',
      height: '23px',
      translate: 'auto',
      translateY: 'full',
      paddingTop: '5px',
      '.block-selection-decoration &': {
        display: 'none',
      },
    }),
  )}
  contenteditable={false}
>
  <button
    class={center({
      size: 'full',
      borderRadius: '4px',
      fontSize: '14px',
      fontWeight: 'medium',
      color: 'gray.400',
      backgroundColor: 'gray.100',
      display: isLastRowHovered ? 'flex' : 'none',
      opacity: '90',
      _groupHover: {
        display: 'flex',
      },
      _hover: {
        backgroundColor: 'gray.200',
      },
      _active: {
        color: 'white',
        backgroundColor: '[var(--prosemirror-color-selection)]',
      },
    })}
    onclick={() => addRowAtEnd(tableNode)}
    type="button"
  >
    <Icon icon={PlusIcon} size={14} />
  </button>
</div>

<div
  class={cx(
    'group',
    css({
      position: 'absolute',
      zIndex: '10',
      top: '0',
      right: '0',
      bottom: '0',
      width: '23px',
      height: 'full',
      translate: 'auto',
      translateX: 'full',
      paddingLeft: '5px',
      '.block-selection-decoration &': {
        display: 'none',
      },
    }),
  )}
  contenteditable={false}
>
  <button
    class={center({
      size: 'full',
      borderRadius: '4px',
      fontSize: '14px',
      fontWeight: 'medium',
      color: 'gray.400',
      backgroundColor: 'gray.100',
      display: isLastColumnHovered ? 'flex' : 'none',
      opacity: '90',
      _groupHover: {
        display: 'flex',
      },
      _hover: {
        backgroundColor: 'gray.200',
      },
      _active: {
        color: 'white',
        backgroundColor: '[var(--prosemirror-color-selection)]',
      },
    })}
    onclick={() => {
      addColumnAtEnd(tableNode);
    }}
    type="button"
  >
    <Icon icon={PlusIcon} size={14} />
  </button>
</div>

<div
  class={cx(
    'group',
    css({
      position: 'absolute',
      zIndex: '10',
      right: '0',
      bottom: '0',
      size: '23px',
      translate: 'auto',
      translateX: 'full',
      translateY: 'full',
      paddingLeft: '5px',
      paddingTop: '5px',
      '.block-selection-decoration &': {
        display: 'none',
      },
    }),
  )}
  contenteditable={false}
>
  <button
    class={center({
      size: 'full',
      borderRadius: 'full',
      fontSize: '14px',
      fontWeight: 'medium',
      color: 'gray.400',
      backgroundColor: 'gray.100',
      display: isLastRowHovered && isLastColumnHovered ? 'flex' : 'none',
      opacity: '90',
      _groupHover: {
        display: 'flex',
      },
      _hover: {
        backgroundColor: 'gray.200',
      },
      _active: {
        color: 'white',
        backgroundColor: '[var(--prosemirror-color-selection)]',
      },
    })}
    onclick={() => {
      addRowAtEnd(tableNode);
      addColumnAtEnd(tableNode);
    }}
    type="button"
  >
    <Icon icon={PlusIcon} size={14} />
  </button>
</div>
