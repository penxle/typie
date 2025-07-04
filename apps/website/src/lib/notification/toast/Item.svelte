<script lang="ts">
  import { backInOut, expoInOut, linear, sineInOut } from 'svelte/easing';
  import { Tween } from 'svelte/motion';
  import { fly, scale, slide } from 'svelte/transition';
  import CheckIcon from '~icons/lucide/check';
  import XIcon from '~icons/lucide/x';
  import ExclamationIcon from '~icons/typie/exclamation';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import { store } from './store';
  import type { Toast } from './store';

  type Props = {
    toast: Toast;
  };

  let { toast }: Props = $props();

  const dismiss = () => store.update((v) => v.filter((t) => t.id !== toast.id));
  const progress = new Tween(100, { duration: toast.duration, easing: linear });

  $effect(() => {
    if (progress.current === 0) {
      dismiss();
    }
  });
</script>

<div
  class={flex({
    align: 'center',
    borderRadius: '10px',
    paddingX: '4px',
    width: 'fit',
    minWidth: '44px',
    maxWidth: '402px',
    height: '38px',
    backgroundColor: 'surface.subtle',
    boxShadow: 'small',
    overflow: 'hidden',
    pointerEvents: 'auto',
  })}
  in:scale={{ duration: 400, easing: backInOut }}
  out:scale={{ duration: 400, delay: 600, easing: backInOut }}
>
  <div
    class={css({
      position: 'relative',
      display: 'flex',
      flex: 'none',
      paddingX: '10px',
      overflow: 'hidden',
    })}
  >
    <div
      class={css(
        { borderRadius: 'full', padding: '3px', size: '18px' },
        toast.type === 'success' && { backgroundColor: { base: '[#0CBB7A]', _dark: '[#19C283]' } },
        toast.type === 'error' && { backgroundColor: 'accent.danger.default' },
      )}
    >
      {#if toast.type === 'success'}
        <Icon style={css.raw({ color: 'text.bright' })} icon={CheckIcon} size={12} />
      {:else if toast.type === 'error'}
        <Icon style={css.raw({ color: 'text.bright' })} icon={ExclamationIcon} size={12} />
      {/if}
    </div>
  </div>

  <div
    in:slide={{ axis: 'x', duration: 400, delay: 400, easing: expoInOut }}
    out:slide={{ axis: 'x', duration: 400, delay: 200, easing: expoInOut }}
  >
    <div
      class={css({ display: 'flex' })}
      onintroend={() => (progress.target = 0)}
      in:fly={{ x: '-0.125rem', duration: 200, delay: 800, easing: sineInOut }}
      out:fly={{ x: '-0.125rem', duration: 200, easing: sineInOut }}
    >
      <div class={flex({ direction: 'column', paddingY: '9px', paddingRight: '40px' })}>
        <span class={css({ fontSize: '14px', fontWeight: 'medium' })}>{toast.title}</span>
        {#if toast.message}
          <span class={css({ fontSize: '12px', color: 'text.subtle' })}>
            {toast.message}
          </span>
        {/if}
      </div>
      <div
        class={center({
          borderLeftWidth: '1px',
          padding: '10px',
          paddingRight: '6px',
          color: 'text.subtle',
        })}
      >
        <button onclick={dismiss} type="button">
          <Icon icon={XIcon} size={18} />
        </button>
      </div>
    </div>
  </div>
</div>
