<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import EllipsisVerticalIcon from '~icons/lucide/ellipsis-vertical';
  import Trash2Icon from '~icons/lucide/trash-2';
  import type { StyleInfo } from '@typie/editor-ffi/browser';

  type Props = {
    entry: StyleInfo;
    isActive: boolean;
    preview: string;
    showDelete: boolean;
    onapply: () => void;
    ondelete: () => void;
    onrowhover: () => void;
    oniconhover: () => void;
  };

  let { entry, isActive, preview, showDelete, onapply, ondelete, onrowhover, oniconhover }: Props = $props();

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom-end',
    offset: 4,
  });
</script>

<button
  class={css({
    display: 'flex',
    alignItems: 'center',
    gap: '12px',
    paddingLeft: '16px',
    paddingRight: '8px',
    paddingY: '8px',
    textAlign: 'left',
    fontSize: '13px',
    color: isActive ? 'text.brand' : 'text.default',
    backgroundColor: isActive ? 'surface.subtle' : 'transparent',
    cursor: 'pointer',
    _hover: { color: 'text.brand', backgroundColor: 'surface.subtle' },
    _focus: { color: 'text.brand', backgroundColor: 'surface.subtle' },
  })}
  data-active={isActive}
  onclick={onapply}
  onmouseenter={onrowhover}
  type="button"
>
  <span style={preview} class={css({ flexGrow: '1', truncate: true })}>{entry.name}</span>
  <span
    class={center({ flexShrink: '0', color: 'text.faint', width: '20px', height: '20px' })}
    onclick={(e) => e.stopPropagation()}
    onmouseenter={oniconhover}
    role="presentation"
    use:anchor
  >
    <Icon icon={EllipsisVerticalIcon} size={14} />
  </span>
</button>

{#if showDelete}
  <div
    class={css({
      borderWidth: '1px',
      borderColor: 'border.subtle',
      borderRadius: '6px',
      backgroundColor: 'surface.default',
      boxShadow: 'small',
      padding: '2px',
      zIndex: 'menu',
    })}
    data-floating-keep-open
    use:floating
  >
    <button
      class={css({
        display: 'flex',
        alignItems: 'center',
        gap: '6px',
        paddingX: '8px',
        height: '28px',
        borderRadius: '4px',
        fontSize: '13px',
        color: 'text.faint',
        cursor: 'pointer',
        whiteSpace: 'nowrap',
        _hover: { backgroundColor: 'surface.muted', color: 'text.danger' },
      })}
      onclick={ondelete}
      type="button"
    >
      <Icon icon={Trash2Icon} size={14} />
      스타일 삭제
    </button>
  </div>
{/if}
