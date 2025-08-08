<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import MinusIcon from '~icons/lucide/minus';
  import PlusIcon from '~icons/lucide/plus';
  import { Icon } from '$lib/components';
  import type { Canvas } from '$lib/canvas';

  type Props = {
    canvas: Canvas;
  };

  let { canvas }: Props = $props();

  const zoomLevels = [10, 25, 50, 75, 100, 125, 150, 200, 250, 500];

  const currentZoomIndex = $derived.by(() => {
    const currentZoom = Math.round(canvas.state.scale * 100);
    const index = zoomLevels.findIndex((level) => level >= currentZoom);
    return index === -1 ? zoomLevels.length - 1 : index;
  });

  const zoomIn = () => {
    const nextIndex = Math.min(currentZoomIndex + 1, zoomLevels.length - 1);
    canvas.scaleTo(zoomLevels[nextIndex] / 100);
  };

  const zoomOut = () => {
    const prevIndex = Math.max(currentZoomIndex - 1, 0);
    canvas.scaleTo(zoomLevels[prevIndex] / 100);
  };
</script>

<div
  class={flex({
    position: 'absolute',
    bottom: '20px',
    right: '20px',
    alignItems: 'center',
    gap: '6px',
    zIndex: '10',
  })}
>
  <button
    class={center({
      borderWidth: '1px',
      borderRadius: '6px',
      size: '28px',
      color: 'text.subtle',
      backgroundColor: 'surface.default',
      transition: 'common',
      _hover: {
        color: 'text.default',
        backgroundColor: 'surface.subtle',
      },
    })}
    onclick={zoomOut}
    type="button"
  >
    <Icon icon={MinusIcon} size={14} />
  </button>

  <span
    class={css({
      width: '60px',
      fontSize: '12px',
      fontWeight: 'medium',
      color: 'text.subtle',
      textAlign: 'center',
    })}
  >
    {Math.round(canvas.state.scale * 100)}%
  </span>

  <button
    class={center({
      borderWidth: '1px',
      borderRadius: '6px',
      size: '28px',
      color: 'text.subtle',
      backgroundColor: 'surface.default',
      transition: 'common',
      _hover: {
        color: 'text.default',
        backgroundColor: 'surface.subtle',
      },
    })}
    onclick={zoomIn}
    type="button"
  >
    <Icon icon={PlusIcon} size={14} />
  </button>
</div>
