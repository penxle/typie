<script lang="ts">
  import dayjs from 'dayjs';
  import { fade } from 'svelte/transition';
  import { fragment, graphql } from '$graphql';
  import { createFloatingActions } from '$lib/actions';
  import { comma } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { center, flex, grid } from '$styled-system/patterns';
  import type { HomePage_ActivityGrid_user } from '$graphql';

  type Level = 0 | 1 | 2 | 3 | 4 | 5;

  type Activity = {
    date: dayjs.Dayjs;
    additions: number;
    level: Level;
  };

  type Props = {
    $user: HomePage_ActivityGrid_user;
  };

  const { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment HomePage_ActivityGrid_user on User {
        id

        characterCountChanges {
          date
          additions
        }
      }
    `),
  );

  let hoverActivity = $state<Activity & { element: HTMLElement }>();

  const endDate = dayjs.kst().startOf('day');
  const startDate = endDate.subtract(364, 'days');

  const activities = $derived.by<Activity[]>(() => {
    const activities: Activity[] = [];

    const numbers = $user.characterCountChanges.map(({ additions }) => additions).filter((n) => n > 0);
    const min = Math.min(...numbers, 0);
    const max = Math.max(...numbers, 0);
    const range = max - min;

    let currentDate = startDate;
    while (!currentDate.isAfter(endDate)) {
      const change = $user.characterCountChanges.find(({ date }) => dayjs.utc(date).isSame(currentDate, 'day'));
      if (change) {
        if (change.additions === 0) {
          activities.push({ date: currentDate, additions: 0, level: 0 });
        } else if (change.additions === max) {
          activities.push({ date: currentDate, additions: change.additions, level: 5 });
        } else {
          const value = (change.additions - min) / range;
          const level = (Math.floor(value * 4) + 1) as Level;
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
    let i = 1;
    while (!currentDate.isAfter(endDate)) {
      const month = currentDate.month() + 1;

      const last = monthSpans.at(-1);
      if (last?.month === month) {
        last.end = i;
      } else {
        monthSpans.push({ month, start: i, end: i });
      }

      currentDate = currentDate.add(1, 'week');
      i++;
    }

    return monthSpans;
  });

  const weekdays = [null, '월', null, '수', null, '금', null];

  const cssByLevel = {
    0: css.raw({ backgroundColor: 'gray.100' }),
    1: css.raw({ backgroundColor: 'brand.100' }),
    2: css.raw({ backgroundColor: 'brand.200' }),
    3: css.raw({ backgroundColor: 'brand.300' }),
    4: css.raw({ backgroundColor: 'brand.400' }),
    5: css.raw({ backgroundColor: 'brand.500' }),
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
    <div class={css({ position: 'relative', gridRow: `${i + 2}`, gridColumn: '1' })}>
      {#if weekday}
        <div
          class={center({
            position: 'absolute',
            left: '0',
            insetY: '0',
            fontSize: '13px',
            fontWeight: 'medium',
            color: 'gray.500',
          })}
        >
          {weekday}
        </div>
      {:else}{/if}
    </div>
  {/each}

  {#each monthSpans as month, i (i)}
    <div
      style:grid-column={`${month.start + 1} / ${month.end + 2}`}
      class={center({
        gridRow: '1',
        paddingY: '1px',
        fontSize: '13px',
        fontWeight: 'medium',
        color: 'gray.500',
      })}
    >
      {month.month}월
    </div>
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
      color: 'white',
      backgroundColor: 'gray.600',
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
        쉬었어요
      {/if}
    </div>
  </div>
{/if}
