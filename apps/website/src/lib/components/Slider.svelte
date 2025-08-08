<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { tooltip } from '$lib/actions';

  type Props = {
    min?: number;
    max?: number;
    step?: number;
    value?: number;
    disabled?: boolean;
    tooltipFormatter?: (value: number) => string;
    onchange?: () => void;
  };

  let {
    min = 0,
    max = 100,
    step = 1,
    value = $bindable(0),
    disabled = false,
    tooltipFormatter = (v: number) => v.toString(),
    onchange,
  }: Props = $props();

  let trackEl = $state<HTMLDivElement>();
  let isDragging = $state(false);

  const percentage = $derived(((value - min) / (max - min)) * 100);
  const tooltipMessage = $derived(tooltipFormatter(value));

  const handlePointerDown = (event: PointerEvent) => {
    if (disabled) return;

    const target = event.currentTarget as HTMLElement;
    target.setPointerCapture(event.pointerId);

    isDragging = true;
    updateValue(event);
  };

  const handlePointerMove = (event: PointerEvent) => {
    if (!isDragging) return;
    updateValue(event);
  };

  const handlePointerUp = (event: PointerEvent) => {
    const target = event.currentTarget as HTMLElement;
    target.releasePointerCapture(event.pointerId);
    isDragging = false;
    onchange?.();
  };

  const updateValue = (event: PointerEvent) => {
    if (!trackEl || disabled) return;

    const rect = trackEl.getBoundingClientRect();
    const x = Math.max(0, Math.min(event.clientX - rect.left, rect.width));
    const percentage = x / rect.width;
    const rawValue = min + percentage * (max - min);

    const steppedValue = Math.round(rawValue / step) * step;
    value = Math.max(min, Math.min(max, steppedValue));
  };
</script>

<div
  bind:this={trackEl}
  class={css({
    position: 'relative',
    width: 'full',
    height: '8px',
    borderRadius: '4px',
    backgroundColor: 'surface.muted',
    cursor: disabled ? 'not-allowed' : 'pointer',
    opacity: disabled ? '50' : '100',
  })}
  aria-disabled={disabled}
  aria-valuemax={max}
  aria-valuemin={min}
  aria-valuenow={value}
  ondragstart={(e) => e.preventDefault()}
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  role="slider"
  tabindex={disabled ? -1 : 0}
>
  <div
    style:width="{percentage}%"
    class={css({
      position: 'absolute',
      top: '0',
      left: '0',
      height: 'full',
      borderRadius: '4px',
      backgroundColor: 'accent.brand.default',
      transition: isDragging ? undefined : 'common',
    })}
  ></div>

  <div
    style:left="{percentage}%"
    style:cursor={isDragging ? 'grabbing' : undefined}
    class={css({
      position: 'absolute',
      top: '[50%]',
      transform: '[translate(-50%, -50%)]',
      width: '20px',
      height: '20px',
      borderRadius: 'full',
      backgroundColor: 'surface.default',
      borderWidth: '2px',
      borderColor: 'accent.brand.default',
      boxShadow: '[0 2px 4px rgba(0,0,0,0.1)]',
      cursor: disabled ? 'not-allowed' : 'grab',
      transition: isDragging ? undefined : 'common',
    })}
    ondragstart={(e) => e.preventDefault()}
    role="presentation"
    use:tooltip={{ message: tooltipMessage, placement: 'top', offset: 8, delay: 0, force: isDragging }}
  ></div>
</div>
