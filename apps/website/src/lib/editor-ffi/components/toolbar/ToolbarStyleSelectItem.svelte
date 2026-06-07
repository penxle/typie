<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import PencilIcon from '~icons/lucide/pencil';
  import type { StyleInfo } from '@typie/editor-ffi/browser';

  type Props = {
    entry: StyleInfo;
    isActive: boolean;
    onapply: () => void;
    onedit: () => void;
  };

  let { entry, isActive, onapply, onedit }: Props = $props();
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
  type="button"
>
  <span class={css({ flexGrow: '1', truncate: true })}>{entry.name}</span>
  <span
    class={center({ flexShrink: '0', color: 'text.faint', width: '20px', height: '20px', _hover: { color: 'text.default' } })}
    onclick={(e) => {
      e.stopPropagation();
      onedit();
    }}
    role="presentation"
  >
    <Icon icon={PencilIcon} size={14} />
  </span>
</button>
