<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import PinIcon from '~icons/lucide/pin';

  type Props = {
    pinned: boolean;
    onToggle: () => unknown;
  };

  let { pinned, onToggle }: Props = $props();
  const tooltipLabel = $derived(pinned ? '고정 해제하기' : '고정하기');
  const accessibleLabel = $derived(pinned ? '문서 경로 및 보기 도구 고정 해제' : '문서 경로 및 보기 도구 고정');
</script>

<button
  class={css({
    display: 'grid',
    placeItems: 'center',
    flex: 'none',
    size: '24px',
    borderRadius: '8px',
    cursor: 'pointer',
    _supportHover: { backgroundColor: 'interactive.hover', color: 'text.default' },
  })}
  aria-label={accessibleLabel}
  aria-pressed={pinned}
  data-context-bar-pin
  onclick={onToggle}
  onpointerdown={(event) => event.preventDefault()}
  type="button"
  use:tooltip={{ message: tooltipLabel, placement: 'bottom' }}
>
  <span style:transform={pinned ? 'rotate(0deg)' : 'rotate(45deg)'} class={css({ display: 'grid', placeItems: 'center' })}>
    <Icon icon={PinIcon} size={14} />
  </span>
</button>
