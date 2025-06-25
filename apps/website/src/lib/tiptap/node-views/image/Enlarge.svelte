<script lang="ts">
  import { cubicOut } from 'svelte/easing';
  import { Tween } from 'svelte/motion';
  import XIcon from '~icons/lucide/x';
  import { portal, scrollLock } from '$lib/actions';
  import { ContentProtect, Icon, Img } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
  import type { NodeViewProps } from '../../lib';

  type Props = {
    node: NodeViewProps['node'];
    referenceEl: HTMLDivElement;
    onclose: () => void;
  };

  let { node, referenceEl, onclose }: Props = $props();

  let attrs = $state(node.attrs);
  $effect(() => {
    attrs = node.attrs;
  });

  let targetEl = $state<HTMLDivElement>();

  let referenceRect = $state<DOMRect>();
  let targetRect = $state<DOMRect>();

  const progress = new Tween(0, { duration: 300, easing: cubicOut });
  const opacity = $derived(progress.current);

  let transform = $derived.by(() => {
    if (referenceRect && targetRect) {
      const { width, height, top, left } = referenceRect;

      const scaleX = width + progress.current * (targetRect.width - width);
      const scaleY = height + progress.current * (targetRect.height - height);
      const scale = Math.min(scaleX / width, scaleY / height);

      const centerX = left + width / 2;
      const centerY = top + height / 2;

      const targetCenterX = targetRect.left + targetRect.width / 2;
      const targetCenterY = targetRect.top + targetRect.height / 2;

      const translateX = centerX + progress.current * (targetCenterX - centerX) - centerX;
      const translateY = centerY + progress.current * (targetCenterY - centerY) - centerY;

      return {
        scale,
        translateX,
        translateY,
        width,
        height,
        top,
        left,
      };
    }
  });

  $effect(() => {
    if (referenceEl && targetEl) {
      referenceRect = referenceEl.getBoundingClientRect();
      targetRect = targetEl.getBoundingClientRect();

      progress.target = 1;
    }
  });

  const handleClose = async () => {
    await progress.set(0);
    onclose();
  };
</script>

<svelte:window onclickcapture={handleClose} onkeydown={(e) => e.key === 'Escape' && handleClose()} />

<div class={css({ position: 'fixed', inset: '0', size: 'full', zIndex: '50' })} use:portal use:scrollLock>
  <ContentProtect>
    <div class={css({ position: 'fixed', inset: '0', size: 'full', paddingX: '[5vw]', paddingY: '[5vh]' })}>
      <div bind:this={targetEl} class={css({ size: 'full' })}></div>
    </div>

    <div style:opacity class={css({ position: 'fixed', inset: '0', size: 'full', backgroundColor: 'surface.default' })}>
      <div class={css({ position: 'absolute', top: '20px', right: '20px' })}>
        <button
          class={center({
            borderWidth: '[1.5px]',
            borderColor: 'border.strong',
            borderRadius: 'full',
            marginBottom: '4px',
            color: 'text.faint',
            size: '40px',
            backgroundColor: 'surface.default',
            boxShadow: 'small',
            zIndex: '30',
            _hover: {
              borderColor: 'gray.500',
              color: 'text.subtle',
            },
          })}
          aria-label="닫기"
          onclick={handleClose}
          type="button"
        >
          <Icon icon={XIcon} />
        </button>
        <span class={css({ display: 'block', fontSize: '13px', fontWeight: 'semibold', color: 'text.disabled', textAlign: 'center' })}>
          ESC
        </span>
      </div>
    </div>

    {#if transform}
      <div
        style:top={`${transform.top}px`}
        style:left={`${transform.left}px`}
        style:width={`${transform.width}px`}
        style:height={`${transform.height}px`}
        style:transform={`translate(${transform.translateX}px, ${transform.translateY}px) scale(${transform.scale})`}
        class={center({ position: 'fixed', willChange: 'transform' })}
      >
        <Img
          style={css.raw({ size: 'full', borderRadius: '4px', objectFit: 'contain', cursor: 'zoom-out' })}
          $image={attrs}
          alt="본문 이미지"
          size="full"
        />
      </div>
    {/if}
  </ContentProtect>
</div>
