<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import dayjs from 'dayjs';
  import { graphql } from '$mearie';
  import type { DashboardLayout_GoalTrendChart_entity$key } from '$mearie';

  type Props = {
    entity$key: DashboardLayout_GoalTrendChart_entity$key;
    current: number;
  };

  let { entity$key, current }: Props = $props();

  const entity = createFragment(
    graphql(`
      fragment DashboardLayout_GoalTrendChart_entity on Entity {
        id

        goal {
          id
          targetCharacterCount
          dueAt
          createdAt
        }

        characterCountHistory {
          date
          characterCount
        }
      }
    `),
    () => entity$key,
  );

  const history = $derived(entity.data.characterCountHistory);
  const goal = $derived(entity.data.goal);
  const target = $derived(goal?.targetCharacterCount ?? null);
  const dueAt = $derived(goal?.dueAt ?? null);

  let width = $state(0);
  const height = 200;
  const padding = { top: 8, right: 8, bottom: 20, left: 8 };

  const today = $derived(dayjs.kst().startOf('day'));
  const first = $derived(history.length > 0 ? dayjs(history[0].date).kst() : today);
  const last = $derived(dueAt ? dayjs.max(today, dayjs(dueAt).kst().startOf('day')) : today);
  const spanDays = $derived(Math.max(1, last.diff(first, 'day')));
  const yMax = $derived(Math.max(1, target ?? 0, current, ...history.map((p) => p.characterCount)) * 1.05);

  const x = $derived((date: dayjs.Dayjs) => padding.left + (date.diff(first, 'day') / spanDays) * (width - padding.left - padding.right));
  const y = $derived((value: number) => padding.top + (1 - value / yMax) * (height - padding.top - padding.bottom));

  const linePoints = $derived(history.map((p) => `${x(dayjs(p.date).kst())},${y(p.characterCount)}`).join(' '));

  const paceStart = $derived.by(() => {
    if (!goal?.dueAt) return null;
    const created = dayjs(goal.createdAt).kst().startOf('day');
    const anchor = history.findLast((p) => !dayjs(p.date).kst().isAfter(created)) ?? history[0];
    return anchor ? { date: dayjs.max(created, dayjs(anchor.date).kst()), value: anchor.characterCount } : null;
  });

  const actualStroke = css.raw({ stroke: { base: 'gray.600', _dark: 'dark.gray.400' } });
  const targetStroke = css.raw({ stroke: { base: 'gray.300', _dark: 'dark.gray.700' } });
  const paceStroke = css.raw({ stroke: { base: 'gray.400', _dark: 'dark.gray.600' } });
</script>

<div class={flex({ flexDirection: 'column', gap: '8px' })}>
  <div class={css({ width: 'full' })} bind:clientWidth={width}>
    {#if width > 0 && history.length > 0}
      <svg {height} {width}>
        {#if target !== null}
          <line
            class={css(targetStroke)}
            stroke-dasharray="4 3"
            x1={padding.left}
            x2={width - padding.right}
            y1={y(target)}
            y2={y(target)}
          />
        {/if}
        {#if target !== null && dueAt && paceStart}
          <line
            class={css(paceStroke)}
            stroke-dasharray="2 3"
            x1={x(paceStart.date)}
            x2={x(dayjs(dueAt).kst().startOf('day'))}
            y1={y(paceStart.value)}
            y2={y(target)}
          />
        {/if}
        <polyline class={css({ fill: '[none]' }, actualStroke)} points={linePoints} stroke-linejoin="round" stroke-width="2" />
        <circle class={css({ fill: { base: 'gray.700', _dark: 'dark.gray.300' } })} cx={x(today)} cy={y(current)} r="3" />
        <text class={css({ fill: 'text.faint', fontSize: '10px' })} x={padding.left} y={height - 6}>{first.format('M월 D일')}</text>
        <text class={css({ fill: 'text.faint', fontSize: '10px' })} text-anchor="end" x={width - padding.right} y={height - 6}>
          {last.format('M월 D일')}
        </text>
      </svg>
    {:else if history.length === 0}
      <div class={css({ paddingY: '24px', textAlign: 'center', fontSize: '13px', color: 'text.faint' })}>
        아직 기록이 없어요. 글을 쓰면 하루하루의 글자 수가 쌓여요.
      </div>
    {/if}
  </div>

  {#if history.length > 0}
    <div class={flex({ alignItems: 'center', gap: '12px', fontSize: '12px', color: 'text.faint' })}>
      <div class={flex({ alignItems: 'center', gap: '4px' })}>
        <svg height="8" width="16">
          <line class={css(actualStroke)} stroke-width="2" x1="0" x2="16" y1="4" y2="4" />
        </svg>
        <span>글자 수</span>
      </div>

      {#if target !== null}
        <div class={flex({ alignItems: 'center', gap: '4px' })}>
          <svg height="8" width="16">
            <line class={css(targetStroke)} stroke-dasharray="4 3" x1="0" x2="16" y1="4" y2="4" />
          </svg>
          <span>목표선</span>
        </div>
      {/if}

      {#if dueAt && paceStart}
        <div class={flex({ alignItems: 'center', gap: '4px' })}>
          <svg height="8" width="16">
            <line class={css(paceStroke)} stroke-dasharray="2 3" x1="0" x2="16" y1="4" y2="4" />
          </svg>
          <span>필요 페이스</span>
        </div>
      {/if}
    </div>
  {/if}
</div>
