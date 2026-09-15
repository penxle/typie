<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import XIcon from '~icons/lucide/x';

  type Props = {
    title: string;
    subtitle: string;
    back?: (() => void) | null;
    bordered?: boolean;
    onclose: () => void;
  };

  let { title, subtitle, back = null, bordered = false, onclose }: Props = $props();
</script>

<div
  class={flex({
    position: 'relative',
    alignItems: 'center',
    gap: '8px',
    minWidth: '0',
    paddingTop: '22px',
    paddingRight: '60px',
    paddingBottom: '16px',
    paddingLeft: '24px',
    borderBottomWidth: '1px',
    borderColor: bordered ? 'border.hairline' : 'transparent',
    transition: 'common',
  })}
>
  {#if back}
    <button
      class={center({
        flexShrink: '0',
        size: '28px',
        marginLeft: '-8px',
        borderRadius: '6px',
        color: 'text.muted',
        _hover: { color: 'text.default', backgroundColor: 'surface.hover' },
      })}
      aria-label="뒤로"
      onclick={() => back?.()}
      type="button"
      use:tooltip={{ message: '뒤로', placement: 'bottom' }}
    >
      <Icon icon={ChevronLeftIcon} size={18} />
    </button>
  {/if}

  <h2 class={css({ flexShrink: '0', fontSize: '17px', fontWeight: 'semibold', whiteSpace: 'nowrap' })}>{title}</h2>
  <span
    class={css({
      minWidth: '0',
      overflow: 'hidden',
      fontSize: '13px',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap',
      color: 'text.muted',
    })}
  >
    {subtitle}
  </span>

  <button
    class={center({
      position: 'absolute',
      top: '14px',
      right: '14px',
      size: '28px',
      borderRadius: '6px',
      color: 'text.muted',
      transition: 'common',
      _hover: { color: 'text.default', backgroundColor: 'surface.hover' },
    })}
    aria-label="닫기"
    onclick={onclose}
    type="button"
  >
    <Icon icon={XIcon} size={18} />
  </button>
</div>
