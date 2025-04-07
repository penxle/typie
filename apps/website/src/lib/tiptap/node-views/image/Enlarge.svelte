<script lang="ts">
  import { onMount } from 'svelte';
  import { cubicOut } from 'svelte/easing';
  import { tweened } from 'svelte/motion';
  import { derived } from 'svelte/store';
  import XIcon from '~icons/lucide/x';
  import { portal, scrollLock } from '$lib/actions';
  import { Icon, Img } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
  import type { Readable } from 'svelte/store';
  import type { NodeViewProps } from '../../lib';

  type Rect = { top: number; left: number; width: number; height: number };

  type Props = {
    node: NodeViewProps['node'];
    referenceEl: HTMLDivElement;
    onclose: () => void;
  };

  let { node, referenceEl, onclose }: Props = $props();

  let containerEl = $state<HTMLDivElement>();
  let targetEl = $state<HTMLDivElement>();

  const progress = tweened(0, { duration: 300, easing: cubicOut });
  const opacity = derived(progress, ($progress) => $progress);
  let rect = $state<Readable<Rect>>();

  onMount(async () => {
    if (!referenceEl || !targetEl) {
      return;
    }

    const referenceRect = referenceEl.getBoundingClientRect();
    const targetRect = targetEl.getBoundingClientRect();

    rect = derived(progress, ($progress) => ({
      top: referenceRect.top + $progress * (targetRect.top - referenceRect.top),
      left: referenceRect.left + $progress * (targetRect.left - referenceRect.left),
      width: referenceRect.width + $progress * (targetRect.width - referenceRect.width),
      height: referenceRect.height + $progress * (targetRect.height - referenceRect.height),
    }));

    $progress = 1;
  });

  const handleClose = async () => {
    await progress.set(0);
    onclose();
  };
</script>

<svelte:window onclickcapture={handleClose} onkeydown={(e) => e.key === 'Escape' && handleClose()} />

<div class={css({ position: 'fixed', inset: '0', size: 'full', zIndex: '50' })} use:portal use:scrollLock>
  <div class={css({ position: 'fixed', inset: '0', size: 'full', paddingX: '[5vw]', paddingY: '[5vh]' })}>
    <div bind:this={targetEl} class={css({ size: 'full' })}></div>
  </div>

  <div style:opacity={$opacity} class={css({ position: 'fixed', inset: '0', size: 'full', backgroundColor: 'white' })}>
    <div class={css({ position: 'absolute', top: '20px', right: '20px' })}>
      <button
        class={center({
          borderWidth: '[1.5px]',
          borderColor: 'gray.300',
          borderRadius: 'full',
          marginBottom: '4px',
          color: 'gray.500',
          size: '40px',
          backgroundColor: 'white',
          boxShadow: 'xlarge',
          zIndex: '30',
          _hover: {
            borderColor: 'gray.500',
            color: 'gray.700',
          },
        })}
        aria-label="닫기"
        onclick={handleClose}
        type="button"
      >
        <Icon icon={XIcon} />
      </button>
      <span class={css({ display: 'block', fontSize: '13px', fontWeight: 'semibold', color: 'gray.400', textAlign: 'center' })}>ESC</span>
    </div>
  </div>

  {#if $rect}
    <div
      bind:this={containerEl}
      style:top={`${$rect.top}px`}
      style:left={`${$rect.left}px`}
      style:width={`${$rect.width}px`}
      style:height={`${$rect.height}px`}
      class={center({ position: 'fixed' })}
    >
      <Img
        style={css.raw({ size: 'full', borderRadius: '4px', objectFit: 'contain', cursor: 'zoom-out' })}
        $image={node.current.attrs}
        alt="본문 이미지"
        size="full"
      />
    </div>
  {/if}
</div>
