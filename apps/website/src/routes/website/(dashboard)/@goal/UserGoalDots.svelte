<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Tooltip } from '@typie/ui/components';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import { mergeTodayGoalHistory } from '$lib/user-stats';
  import { graphql } from '$mearie';
  import { getDayClock } from '../day-clock.svelte';
  import type { DashboardLayout_UserGoalDots_user$key } from '$mearie';

  type Props = { user$key: DashboardLayout_UserGoalDots_user$key };

  let { user$key }: Props = $props();
  const dayClock = getDayClock();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_UserGoalDots_user on User {
        id

        goal {
          id
          targetCharacterCount
        }

        goalHistory {
          date
          additions
          achieved
        }

        todayCharacterCountChange {
          date
          additions
        }
      }
    `),
    () => user$key,
  );

  const goalHistory = $derived(
    user.data.goal
      ? mergeTodayGoalHistory(user.data.goalHistory, user.data.goal.targetCharacterCount, user.data.todayCharacterCountChange, dayClock.now)
      : user.data.goalHistory,
  );
  const byDate = $derived(new Map(goalHistory.map((h) => [dayjs(h.date).kst().format('YYYY-MM-DD'), h])));

  const days = $derived.by(() => {
    const today = dayClock.now;

    return Array.from({ length: 112 }, (_, i) => {
      const date = today.subtract(111 - i, 'day');
      const key = date.format('YYYY-MM-DD');
      const row = byDate.get(key);
      return { key, label: date.format('M월 D일 ddd'), row };
    });
  });

  const message = (day: (typeof days)[number]) => {
    if (!day.row) {
      return `${day.label} · 목표 없음`;
    }

    const state = day.row.achieved ? '달성' : day.row.additions > 0 ? '일부 달성' : '미달성';
    return `${day.label} · ${comma(day.row.additions)}자 · ${state}`;
  };
</script>

<div class={flex({ flexDirection: 'column', gap: '8px' })}>
  <div class={flex({ flexWrap: 'wrap', gap: '3px' })}>
    {#each days as day (day.key)}
      <Tooltip message={message(day)} placement="top">
        <div
          class={css(
            { size: '10px', borderRadius: 'full' },
            day.row?.achieved
              ? { backgroundColor: 'accent.success.default' }
              : day.row && day.row.additions > 0
                ? { backgroundColor: 'accent.success.default', opacity: '40' }
                : day.row
                  ? { backgroundColor: { base: 'gray.300', _dark: 'dark.gray.700' } }
                  : { borderWidth: '1px', borderColor: { base: 'gray.300', _dark: 'dark.gray.700' } },
          )}
        ></div>
      </Tooltip>
    {/each}
  </div>

  <div class={flex({ alignItems: 'center', flexWrap: 'wrap', gap: '10px', fontSize: '11px', color: 'text.faint' })}>
    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <div class={css({ size: '8px', borderRadius: 'full', backgroundColor: 'accent.success.default' })}></div>
      <span>달성</span>
    </div>

    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <div class={css({ size: '8px', borderRadius: 'full', backgroundColor: 'accent.success.default', opacity: '40' })}></div>
      <span>일부 달성</span>
    </div>

    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <div class={css({ size: '8px', borderRadius: 'full', backgroundColor: { base: 'gray.300', _dark: 'dark.gray.700' } })}></div>
      <span>미달성</span>
    </div>

    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <div
        class={css({ size: '8px', borderRadius: 'full', borderWidth: '1px', borderColor: { base: 'gray.300', _dark: 'dark.gray.700' } })}
      ></div>
      <span>목표 없음</span>
    </div>
  </div>
</div>
