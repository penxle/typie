<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import { mergeTodayCharacterCountChanges, mergeTodayGoalHistory } from '$lib/user-stats';
  import { graphql } from '$mearie';
  import { getDayClock } from '../day-clock.svelte';
  import type { DashboardLayout_UserGoalHistoryTable_user$key } from '$mearie';

  type Props = { user$key: DashboardLayout_UserGoalHistoryTable_user$key };

  let { user$key }: Props = $props();
  const dayClock = getDayClock();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_UserGoalHistoryTable_user on User {
        id

        goal {
          id
          targetCharacterCount
        }

        characterCountChanges {
          date
          additions
        }

        todayCharacterCountChange {
          date
          additions
        }

        goalHistory {
          date
          additions
          achieved
        }
      }
    `),
    () => user$key,
  );

  const characterCountChanges = $derived(
    mergeTodayCharacterCountChanges(user.data.characterCountChanges, user.data.todayCharacterCountChange, dayClock.now),
  );
  const goalHistory = $derived(
    user.data.goal
      ? mergeTodayGoalHistory(user.data.goalHistory, user.data.goal.targetCharacterCount, user.data.todayCharacterCountChange, dayClock.now)
      : user.data.goalHistory,
  );
  const additionsByDate = $derived(new Map(characterCountChanges.map((h) => [dayjs(h.date).kst().format('YYYY-MM-DD'), h.additions])));
  const judgmentByDate = $derived(new Map(goalHistory.map((h) => [dayjs(h.date).kst().format('YYYY-MM-DD'), h.achieved])));

  const rows = $derived.by(() => {
    const today = dayClock.now;

    return Array.from({ length: 30 }, (_, i) => {
      const date = today.subtract(i, 'day');
      const key = date.format('YYYY-MM-DD');
      return { key, label: date.format('M월 D일'), additions: additionsByDate.get(key) ?? 0, achieved: judgmentByDate.get(key) ?? null };
    });
  });
</script>

<div class={flex({ flexDirection: 'column' })}>
  <div class={flex({ paddingY: '4px', fontSize: '11px', color: 'text.muted', borderBottomWidth: '1px', borderColor: 'border.hairline' })}>
    <span class={css({ flex: '1' })}>날짜</span>
    <span class={css({ flex: '1', textAlign: 'center' })}>쓴 글자</span>
    <span class={css({ flex: '1', textAlign: 'right' })}>달성</span>
  </div>

  {#each rows as row (row.key)}
    <div
      class={flex({
        paddingY: '4px',
        fontSize: '12px',
        fontVariantNumeric: 'tabular-nums',
        borderBottomWidth: '1px',
        borderColor: 'border.hairline',
      })}
    >
      <span class={css({ flex: '1', color: 'text.hint' })}>{row.label}</span>
      <span class={css({ flex: '1', textAlign: 'center', color: 'text.muted' })}>{comma(row.additions)}자</span>
      <span class={css({ flex: '1', textAlign: 'right' }, row.achieved ? { color: 'success.default' } : { color: 'text.hint' })}>
        {row.achieved ? '달성' : row.achieved === false ? '미달성' : '—'}
      </span>
    </div>
  {/each}
</div>
