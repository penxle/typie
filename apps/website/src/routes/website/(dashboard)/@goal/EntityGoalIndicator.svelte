<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { ProgressRing, Tooltip } from '@typie/ui/components';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import { dueStatus, goalColorState, timeFraction } from '$lib/goal';
  import { getDayClock } from '../day-clock.svelte';

  type Props = {
    current: number;
    targetCharacterCount: number;
    dueAt?: string | null;
    goalCreatedAt: string;
    onclick?: () => void;
  };

  let { current, targetCharacterCount, dueAt, goalCreatedAt, onclick }: Props = $props();
  const dayClock = getDayClock();

  const today = $derived(dayClock.now.startOf('day'));
  const state = $derived(goalColorState(current, targetCharacterCount));
  const duePassed = $derived(!!dueAt && dayjs(dueAt).kst().startOf('day').isBefore(today));
  const overdue = $derived(duePassed && state === 'under');
  const pie = $derived(state === 'under' && dueAt ? timeFraction(dayjs(goalCreatedAt).kst(), dayjs(dueAt).kst(), dayClock.now) : null);

  const percent = $derived(Math.floor((current / targetCharacterCount) * 100));
  const tooltip = $derived.by(() => {
    const base = `${comma(current)} / ${comma(targetCharacterCount)}자 (${percent}%)`;
    if (!dueAt) return base;
    const status = dueStatus(current, targetCharacterCount, dayjs(dueAt).kst(), today, 'full');
    return status ? `${base} · ${status.label}` : base;
  });
</script>

<Tooltip style={css.raw({ display: 'flex', flexShrink: '0' })} message={tooltip} placement="bottom">
  {#if onclick}
    <button
      class={css({ display: 'flex', flexShrink: '0', cursor: 'pointer' })}
      aria-label="목표"
      onclick={(e) => {
        e.preventDefault();
        e.stopPropagation();
        onclick();
      }}
      type="button"
    >
      <ProgressRing {pie} pieWarning={overdue} progress={current / targetCharacterCount} size={16} {state} />
    </button>
  {:else}
    <ProgressRing {pie} pieWarning={overdue} progress={current / targetCharacterCount} size={16} {state} />
  {/if}
</Tooltip>
