<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import { graphql } from '$mearie';
  import type { DashboardLayout_UserGoalTrendChart_user$key } from '$mearie';

  type Props = { user$key: DashboardLayout_UserGoalTrendChart_user$key };

  let { user$key }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_UserGoalTrendChart_user on User {
        id

        goal {
          id
          targetCharacterCount
        }

        characterCountChanges {
          date
          additions
        }
      }
    `),
    () => user$key,
  );

  const target = $derived(user.data.goal?.targetCharacterCount ?? null);
  const byDate = $derived(new Map(user.data.characterCountChanges.map((h) => [dayjs(h.date).kst().format('YYYY-MM-DD'), h.additions])));

  const days = $derived.by(() => {
    const today = dayjs.kst().startOf('day');

    return Array.from({ length: 28 }, (_, i) => {
      const date = today.subtract(27 - i, 'day');
      const key = date.format('YYYY-MM-DD');
      return { key, label: date.format('M월 D일 ddd'), additions: byDate.get(key) ?? 0 };
    });
  });

  let width = $state(0);
  const height = 80;
  const gap = 3;

  const yMax = $derived(Math.max(1, target ?? 0, ...days.map((d) => d.additions)) * 1.05);
  const barWidth = $derived(width > 0 ? (width - (days.length - 1) * gap) / days.length : 0);
  const barHeight = (additions: number) => (additions / yMax) * height;

  const barFill = css.raw({ fill: { base: 'gray.400', _dark: 'dark.gray.600' } });
  const targetStroke = css.raw({ stroke: { base: 'gray.300', _dark: 'dark.gray.700' } });
</script>

<div class={flex({ flexDirection: 'column', gap: '4px' })}>
  <div class={css({ width: 'full' })} bind:clientWidth={width}>
    {#if width > 0}
      <svg {height} {width}>
        {#if target !== null}
          <line
            class={css(targetStroke)}
            stroke-dasharray="4 3"
            x1="0"
            x2={width}
            y1={height - (target / yMax) * height}
            y2={height - (target / yMax) * height}
          />
        {/if}
        {#each days as day, i (day.key)}
          {#if day.additions > 0}
            <rect
              class={css(barFill)}
              height={barHeight(day.additions)}
              rx="1"
              width={barWidth}
              x={i * (barWidth + gap)}
              y={height - barHeight(day.additions)}
            >
              <title>{day.label} · {comma(day.additions)}자</title>
            </rect>
          {/if}
        {/each}
      </svg>
    {/if}
  </div>

  <div class={flex({ justifyContent: 'space-between', fontSize: '10px', color: 'text.faint' })}>
    <span>{dayjs.kst().subtract(27, 'day').format('M월 D일')}</span>
    <span>{dayjs.kst().format('M월 D일')}{target === null ? '' : ' · ┄ 목표선'}</span>
  </div>
</div>
