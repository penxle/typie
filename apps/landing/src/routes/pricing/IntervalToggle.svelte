<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { COPY, PLAN } from './pricing';
  import type { Interval } from './pricing';

  type Props = { interval: Interval };

  let { interval = $bindable() }: Props = $props();

  const OPTIONS: readonly { id: Interval; label: string; badge?: string }[] = [
    { id: 'monthly', label: COPY.monthlyLabel },
    { id: 'yearly', label: COPY.yearlyLabel, badge: PLAN.yearlyDiscount },
  ];

  let buttons = $state<HTMLButtonElement[]>([]);
  let metrics = $state<{ width: number; offset: number }[]>([]);

  const active = $derived(metrics[OPTIONS.findIndex((option) => option.id === interval)] ?? { width: 0, offset: 0 });

  $effect(() => {
    const observer = new ResizeObserver(() => {
      metrics = buttons.map((button) => ({ width: button.offsetWidth, offset: button.offsetLeft }));
    });
    for (const button of buttons) if (button) observer.observe(button);
    return () => observer.disconnect();
  });

  const groupClass = css({
    position: 'relative',
    display: 'inline-flex',
    padding: '4px',
    borderRadius: 'full',
    borderWidth: '1px',
    borderColor: 'border.hairline',
    fontFamily: 'ui',
  });
  const indicatorClass = css({
    position: 'absolute',
    top: '4px',
    left: '0',
    height: '32px',
    borderRadius: 'full',
    backgroundColor: 'text.default/10',
    transition: '[transform 200ms cubic-bezier(0.23, 1, 0.32, 1), width 200ms cubic-bezier(0.23, 1, 0.32, 1)]',
    _motionReduce: { transition: '[none]' },
    pointerEvents: 'none',
  });
  const buttonClass = css({
    position: 'relative',
    display: 'inline-flex',
    alignItems: 'center',
    gap: '8px',
    height: '32px',
    paddingX: '14px',
    borderRadius: 'full',
    fontSize: '14px',
    fontWeight: 'medium',
    color: 'text.muted',
    transition: '[color 150ms ease-out]',
    _hover: { color: 'text.default' },
    _pressed: { color: 'text.default' },
    _active: { transform: 'scale(0.97)' },
  });
  const badgeClass = css({
    fontSize: '11px',
    fontFamily: 'mono',
    fontWeight: 'medium',
    letterSpacing: '[0.02em]',
    color: 'success.default',
  });
</script>

<div class={groupClass} aria-label="결제 주기" role="group">
  <span style:width="{active.width}px" style:transform="translateX({active.offset}px)" class={indicatorClass} aria-hidden="true"></span>
  {#each OPTIONS as option, index (option.id)}
    <button
      bind:this={buttons[index]}
      class={buttonClass}
      aria-pressed={interval === option.id}
      onclick={() => (interval = option.id)}
      type="button"
    >
      {option.label}
      {#if option.badge}<span class={badgeClass}>{option.badge}</span>{/if}
    </button>
  {/each}
</div>
