<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex, grid } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import { fade } from 'svelte/transition';

  type Level = 0 | 1 | 2 | 3 | 4 | 5;

  type Activity = {
    date: dayjs.Dayjs;
    additions: number;
    level: Level;
  };

  type Props = {
    characterCountChanges: {
      date: string;
      additions: number;
    }[];
  };

  const { characterCountChanges }: Props = $props();

  let hoverActivity = $state<Activity & { element: HTMLElement }>();

  const endDate = dayjs.kst().startOf('day');
  const startDate = endDate.subtract(364, 'days');

  const activities = $derived.by<Activity[]>(() => {
    const activities: Activity[] = [];

    const numbers = characterCountChanges.map(({ additions }) => additions).filter((n) => n > 0);
    const min = Math.min(...numbers);
    const max = Math.max(...numbers);
    const range = max - min;

    const changes = Object.fromEntries(characterCountChanges.map((change) => [dayjs(change.date).unix(), change]));

    let currentDate = startDate;
    while (!currentDate.isAfter(endDate)) {
      const change = changes[currentDate.unix()];
      if (change) {
        if (change.additions === 0) {
          activities.push({ date: currentDate, additions: 0, level: 0 });
        } else if (range === 0) {
          activities.push({ date: currentDate, additions: change.additions, level: 3 });
        } else {
          const value = (change.additions - min) / range;
          const level = Math.min(Math.floor(value * 5) + 1, 5) as Level;
          activities.push({ date: currentDate, additions: change.additions, level });
        }
      } else {
        activities.push({ date: currentDate, additions: 0, level: 0 });
      }

      currentDate = currentDate.add(1, 'day');
    }

    return activities;
  });

  const monthSpans = $derived.by(() => {
    const monthSpans: { month: number; start: number; end: number }[] = [];

    let currentDate = startDate.startOf('week');
    const endWeek = endDate.endOf('week');

    let weekIndex = 0;
    let prevMonth = -1;
    let monthStartWeek = -1;

    while (!currentDate.isAfter(endWeek)) {
      let weekMonth = -1;
      let hasFirstOfMonth = false;

      for (let day = 0; day < 7; day++) {
        const checkDate = currentDate.add(day, 'days');

        if (checkDate.isBefore(startDate) || checkDate.isAfter(endDate)) {
          continue;
        }

        if (weekMonth === -1) {
          weekMonth = checkDate.month() + 1;
        }

        if (checkDate.date() === 1) {
          hasFirstOfMonth = true;
          weekMonth = checkDate.month() + 1;
          break;
        }
      }

      if (weekMonth !== -1 && (monthStartWeek === -1 || (hasFirstOfMonth && weekMonth !== prevMonth))) {
        if (monthStartWeek >= 0 && prevMonth !== -1) {
          monthSpans.push({ month: prevMonth, start: monthStartWeek, end: weekIndex - 1 });
        }

        monthStartWeek = weekIndex;
        prevMonth = weekMonth;
      }

      currentDate = currentDate.add(1, 'week');
      weekIndex++;
    }

    if (monthStartWeek >= 0 && prevMonth !== -1) {
      monthSpans.push({ month: prevMonth, start: monthStartWeek, end: weekIndex - 1 });
    }

    return monthSpans;
  });

  const weekdays = [null, '월', null, '수', null, '금', null];

  const cssByLevel = {
    0: css.raw({ backgroundColor: { base: 'gray.100', _dark: 'dark.gray.800' } }),
    1: css.raw({ backgroundColor: { base: 'brand.100', _dark: 'dark.brand.700' } }),
    2: css.raw({ backgroundColor: { base: 'brand.300', _dark: 'dark.brand.600' } }),
    3: css.raw({ backgroundColor: { base: 'brand.500', _dark: 'dark.brand.500' } }),
    4: css.raw({ backgroundColor: { base: 'brand.700', _dark: 'dark.brand.300' } }),
    5: css.raw({ backgroundColor: { base: 'brand.900', _dark: 'dark.brand.100' } }),
  };

  const { anchor, floating } = createFloatingActions({
    placement: 'left-start',
    offset: 4,
  });

  $effect(() => {
    if (hoverActivity) {
      anchor(hoverActivity.element);
    }
  });
</script>

<div
  class={grid({
    gridTemplateRows: 'auto repeat(7, minmax(0, 1fr))',
    gridTemplateColumns: 'repeat(auto-fit, minmax(0px, 1fr))',
    gridAutoFlow: 'column',
    gap: '4px',
    width: 'full',
  })}
>
  {#each weekdays as weekday, i (i)}
    <div style:grid-row={`${i + 2}`} class={css({ position: 'relative', gridColumn: '1' })}>
      {#if weekday}
        <div
          class={center({
            position: 'absolute',
            left: '0',
            insetY: '0',
            fontSize: '13px',
            fontWeight: 'medium',
            color: 'text.faint',
          })}
        >
          {weekday}
        </div>
      {/if}
    </div>
  {/each}

  {#each monthSpans as month, i (i)}
    {#if month.end - month.start >= 1 || i === monthSpans.length - 1}
      <div
        style:grid-column={`${month.start + 2} / ${month.end + 3}`}
        class={css({
          gridRow: '1',
          paddingY: '1px',
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.faint',
          whiteSpace: 'nowrap',
          overflow: 'visible',
        })}
      >
        {month.month}월
      </div>
    {/if}
  {/each}

  {#each activities as activity (activity.date)}
    <div
      style:grid-row={activity.date.day() + 2}
      class={css({ borderRadius: '2px', aspectRatio: '1/1' }, cssByLevel[activity.level])}
      onpointerenter={(e) => {
        hoverActivity = { ...activity, element: e.currentTarget };
      }}
      onpointerleave={() => {
        hoverActivity = undefined;
      }}
    ></div>
  {/each}
</div>

{#if hoverActivity}
  <div
    class={flex({
      flexDirection: 'column',
      borderRadius: '6px',
      paddingX: '10px',
      paddingY: '6px',
      color: 'text.bright',
      backgroundColor: 'surface.dark',
      zIndex: 'modal',
    })}
    use:floating
    in:fade={{ duration: 100, delay: 100 }}
  >
    <div class={css({ fontSize: '12px', fontWeight: 'medium' })}>
      {hoverActivity.date.format('YYYY년 M월 D일')}
    </div>

    <div class={css({ fontSize: '12px', fontWeight: 'bold' })}>
      {#if hoverActivity.additions > 0}
        {comma(hoverActivity.additions)}자 작성했어요
      {:else}
        기록이 없어요
      {/if}
    </div>
  </div>
{/if}
