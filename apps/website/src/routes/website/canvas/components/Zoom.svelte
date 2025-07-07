<script lang="ts">
  import Maximize2Icon from '~icons/lucide/maximize-2';
  import MinusIcon from '~icons/lucide/minus';
  import PlusIcon from '~icons/lucide/plus';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import type { Canvas } from '../lib/canvas.svelte';

  type Props = {
    canvas: Canvas;
  };

  let { canvas }: Props = $props();

  const buttonStyle = center({
    borderRadius: '8px',
    size: '36px',
    color: 'text.muted',
    backgroundColor: 'surface.default',
    transition: 'common',
    _hover: {
      backgroundColor: 'interactive.hover',
      color: 'text.default',
    },
  });
</script>

<div
  class={flex({
    flexDirection: 'column',
    gap: '2px',
    borderRadius: '12px',
    backgroundColor: 'surface.default',
    padding: '8px',
    boxShadow: 'medium',
    borderWidth: '1px',
    borderColor: 'border.default',
  })}
>
  <button class={buttonStyle} onclick={() => canvas.scaleBy(1.1)} title="확대 (Cmd/Ctrl + Scroll)" type="button">
    <Icon icon={PlusIcon} size={18} />
  </button>

  <button class={buttonStyle} onclick={() => canvas.scaleBy(0.9)} title="축소 (Cmd/Ctrl + Scroll)" type="button">
    <Icon icon={MinusIcon} size={18} />
  </button>

  <div
    class={css({
      height: '1px',
      backgroundColor: 'border.default',
      marginY: '4px',
    })}
  ></div>

  <button class={buttonStyle} onclick={() => canvas.scaleTo(1)} title="화면에 맞추기" type="button">
    <Icon icon={Maximize2Icon} size={18} />
  </button>

  <div
    class={center({
      marginTop: '4px',
      color: 'text.muted',
      fontSize: '11px',
      fontWeight: 'medium',
      height: '24px',
    })}
  >
    {Math.round(canvas.state.scale * 100)}%
  </div>
</div>
