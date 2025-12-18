<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { getEditor } from '$lib/editor/context';
  import type { ExternalElement } from '$lib/editor/types';

  type Props = {
    el: ExternalElement;
  };

  let { el }: Props = $props();

  const editor = getEditor();

  const handleDelete = (e: MouseEvent) => {
    e.stopPropagation();
    editor.dispatch({ type: 'deleteNode', nodeId: el.nodeId });
    editor.focus();
  };
</script>

<div
  style:left="{el.bounds.x}px"
  style:top="{el.bounds.y}px"
  style:width="{el.bounds.width}px"
  style:height="{el.bounds.height}px"
  class={cx('group', css({ position: 'absolute', userSelect: 'none' }))}
  data-node-id={el.nodeId}
>
  <img class={css({ height: 'full', width: 'full', objectFit: 'cover' })} alt="" src={el.data.src} />

  {#if el.isSelected}
    <div class={css({ position: 'absolute', inset: '0', backgroundColor: 'selection' })}></div>
  {/if}

  <button
    class={center({
      position: 'absolute',
      top: '10px',
      right: '10px',
      borderRadius: '4px',
      size: '28px',
      color: 'text.bright',
      backgroundColor: '[#363839/70]',
      opacity: '0',
      transition: 'opacity',
      cursor: 'pointer!',
      _hover: { backgroundColor: '[#363839/40]' },
      _groupHover: { opacity: '100' },
    })}
    aria-label="이미지 삭제"
    onclick={handleDelete}
    type="button"
  >
    <Icon icon={Trash2Icon} size={16} />
  </button>
</div>
