<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import Maximize2Icon from '~icons/lucide/maximize-2';
  import Minimize2Icon from '~icons/lucide/minimize-2';
  import { CONTEXT_BAR_TRANSIENT_VISIBLE_MS } from './editor-context-bar.svelte';
  import type { TransientVisibilityState } from './transient-visibility.svelte';

  type Props = {
    enabled: boolean;
    segment: TransientVisibilityState;
    visible: boolean;
    onToggle: () => unknown;
  };

  let { enabled, segment, visible, onToggle }: Props = $props();
  let observedEnabled: boolean | undefined;

  const label = $derived(enabled ? '집중 모드 끄기' : '집중 모드 켜기');
  const shortcut = $derived(enabled ? (['Esc'] as ['Esc']) : (['Mod', 'Shift', 'M'] as ['Mod', 'Shift', 'M']));

  $effect(() => {
    const current = enabled;
    if (observedEnabled !== undefined && observedEnabled !== current) {
      segment.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
    }
    observedEnabled = current;
  });
</script>

<div class={css({ display: 'flex', alignItems: 'center', paddingX: '8px', paddingY: '4px' })}>
  <button
    class={css({
      display: 'grid',
      placeItems: 'center',
      size: '24px',
      borderRadius: '8px',
      cursor: 'pointer',
      _supportHover: { backgroundColor: 'interactive.hover', color: 'text.default' },
      _focusVisible: { outline: '2px solid token(colors.border.focus)', outlineOffset: '-2px' },
    })}
    aria-label={label}
    aria-pressed={enabled}
    data-editor-focus-mode-control
    onclick={onToggle}
    onpointerdown={(event) => event.preventDefault()}
    tabindex={visible ? 0 : -1}
    type="button"
    use:tooltip={{ message: label, keys: shortcut, placement: 'bottom' }}
  >
    <Icon icon={enabled ? Minimize2Icon : Maximize2Icon} size={14} />
  </button>
</div>
