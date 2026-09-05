<script lang="ts">
  import { createMutation, getClient } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal, ProgressRing } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import { comma, downloadFromBase64 } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import CopyIcon from '~icons/lucide/copy';
  import DownloadIcon from '~icons/lucide/download';
  import { dailyGoalStatus, mergeTodayCharacterCountChanges, writingStreaks } from '$lib/user-stats';
  import { graphql } from '$mearie';
  import ActivityChart from './ActivityChart.svelte';
  import ActivityGrid from './ActivityGrid.svelte';
  import type { DataOf } from '@mearie/core';

  const app = getAppContext();
  const client = getClient();

  const statsQuery = graphql(`
    query DashboardLayout_StatsModal_Query {
      me @required {
        id
        name
        documentCount

        characterCountChanges {
          date
          additions
          deletions
        }

        todayCharacterCountChange {
          date
          additions
          deletions
        }

        usage {
          totalCharacterCount
        }

        goal {
          id
          targetCharacterCount
        }

        goalHistory {
          date
          targetCharacterCount
          additions
          achieved
        }
      }
    }
  `);

  const createSnapshot = (data: DataOf<typeof statsQuery>, today: dayjs.Dayjs) => ({
    today,
    name: data.me.name,
    documentCount: data.me.documentCount,
    characterCountChanges: data.me.characterCountChanges.map((change) => ({ ...change })),
    todayCharacterCountChange: { ...data.me.todayCharacterCountChange },
    totalCharacterCount: data.me.usage.totalCharacterCount,
    goal: data.me.goal ? { ...data.me.goal } : null,
    goalHistory: data.me.goalHistory.map((entry) => ({ ...entry })),
  });

  let snapshot = $state.raw<ReturnType<typeof createSnapshot>>();
  let requestId = 0;

  const loadSnapshot = async () => {
    const id = ++requestId;
    snapshot = undefined;

    try {
      const data = await client.query(statsQuery, {}, { fetchPolicy: 'network-only' });
      if (id !== requestId || !app.state.statsOpen) return;

      snapshot = createSnapshot(data, dayjs.kst());
    } catch {
      if (id !== requestId || !app.state.statsOpen) return;
      app.state.statsOpen = false;
      Toast.error('통계를 불러오지 못했어요. 잠시 후 다시 시도해 주세요.');
    }
  };

  $effect(() => {
    if (app.state.statsOpen) {
      void loadSnapshot();
    } else {
      requestId++;
      snapshot = undefined;
    }
  });

  type StreakData = {
    currentStreak: number;
    longestStreak: number;
    thisMonthDays: number;
    totalDays: number;
  };

  const calculateStreakData = (characterCountChanges: { date: string; additions: number }[], today: dayjs.Dayjs): StreakData => {
    const startOfToday = today.startOf('day');
    const activeDates = new Set(characterCountChanges.filter((c) => c.additions > 0).map((c) => dayjs(c.date).kst().format('YYYY-MM-DD')));
    const streak = writingStreaks(characterCountChanges, startOfToday);

    const monthStart = startOfToday.startOf('month');
    let thisMonthDays = 0;
    for (const dateStr of activeDates) {
      const date = dayjs.kst(dateStr);
      if (date.isSame(monthStart, 'month')) {
        thisMonthDays++;
      }
    }

    const totalDays = activeDates.size;

    return {
      currentStreak: streak.current,
      longestStreak: streak.best,
      thisMonthDays,
      totalDays,
    };
  };

  const characterCountChanges = $derived.by(() => {
    if (!snapshot) return [];
    return mergeTodayCharacterCountChanges(snapshot.characterCountChanges, snapshot.todayCharacterCountChange, snapshot.today);
  });

  const streakData = $derived.by(() => {
    if (!snapshot) return null;
    return calculateStreakData(characterCountChanges, snapshot.today);
  });

  type WeekdayData = {
    dayIndex: number;
    label: string;
    totalAdditions: number;
    avgAdditions: number;
    count: number;
  };

  const weekdayLabels = ['일', '월', '화', '수', '목', '금', '토'];

  const calculateWeekdayPattern = (characterCountChanges: { date: string; additions: number }[]): WeekdayData[] => {
    const weekdayStats = Array.from({ length: 7 }, (_, i) => ({
      dayIndex: i,
      label: weekdayLabels[i],
      totalAdditions: 0,
      count: 0,
    }));

    for (const change of characterCountChanges) {
      if (change.additions <= 0) {
        continue;
      }

      const dayOfWeek = dayjs(change.date).kst().day();
      weekdayStats[dayOfWeek].totalAdditions += change.additions;
      weekdayStats[dayOfWeek].count++;
    }

    return weekdayStats.map((stat) => ({
      ...stat,
      avgAdditions: stat.count > 0 ? Math.round(stat.totalAdditions / stat.count) : 0,
    }));
  };

  const weekdayData = $derived.by(() => {
    if (!snapshot) return null;
    return calculateWeekdayPattern(characterCountChanges);
  });

  const maxWeekdayAvg = $derived(weekdayData ? Math.max(...weekdayData.map((d) => d.avgAdditions)) : 0);
  const bestWeekdayIndex = $derived(weekdayData ? weekdayData.findIndex((d) => d.avgAdditions === maxWeekdayAvg) : -1);

  const [generateActivityImage] = createMutation(
    graphql(`
      mutation DashboardLayout_StatsModal_GenerateActivityImage {
        generateActivityImage
      }
    `),
  );

  const copyActivityImage = async () => {
    const resp = await generateActivityImage();
    const b64 = resp.generateActivityImage;
    const blob = new Blob([Uint8Array.fromBase64(b64)], { type: 'image/png' });
    await navigator.clipboard.write([new ClipboardItem({ 'image/png': blob })]);

    Toast.success('이미지가 클립보드에 복사되었어요.');
  };

  const downloadActivityImage = async () => {
    const resp = await generateActivityImage();
    const b64 = resp.generateActivityImage;
    downloadFromBase64(b64, `${snapshot?.name ?? '타이피'} - 나의 글쓰기 발자취.png`, 'image/png');

    Toast.success('이미지가 다운로드되었어요.');
  };

  const cardStyle = css.raw({
    padding: '20px',
    borderRadius: '16px',
    backgroundColor: 'surface.default',
    borderWidth: '1px',
    borderColor: 'border.hairline',
  });
</script>

<Modal
  style={css.raw({
    gap: '20px',
    maxWidth: '720px',
    padding: '24px',
    backgroundColor: 'surface.canvas',
  })}
  loading={!snapshot}
  onclose={() => {
    app.state.statsOpen = false;
  }}
  open={app.state.statsOpen}
>
  {#if snapshot && streakData}
    <div class={css({ fontSize: '17px', fontWeight: 'semibold', color: 'text.default' })}>나의 글쓰기 통계</div>

    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <div class={flex({ gap: '12px' })}>
        <div class={css(cardStyle, { flex: '1' })}>
          <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted', marginBottom: '8px' })}>총 글자</div>
          <div class={css({ fontSize: '28px', fontWeight: 'bold', color: 'text.default', fontVariantNumeric: 'tabular-nums' })}>
            {comma(snapshot.totalCharacterCount)}
          </div>
        </div>

        <div class={css(cardStyle, { flex: '1' })}>
          <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted', marginBottom: '8px' })}>총 문서</div>
          <div class={css({ fontSize: '28px', fontWeight: 'bold', color: 'text.default', fontVariantNumeric: 'tabular-nums' })}>
            {snapshot.documentCount}
          </div>
        </div>

        <div class={css(cardStyle, { flex: '1' })}>
          <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted', marginBottom: '8px' })}>활동일</div>
          <div class={css({ fontSize: '28px', fontWeight: 'bold', color: 'text.default', fontVariantNumeric: 'tabular-nums' })}>
            {streakData.totalDays}
          </div>
        </div>
      </div>

      <div class={flex({ gap: '12px' })}>
        <div class={css(cardStyle, { flex: '1' })}>
          <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted', marginBottom: '8px' })}>연속 기록</div>
          <div class={css({ fontSize: '28px', fontWeight: 'bold', color: 'text.default', fontVariantNumeric: 'tabular-nums' })}>
            {streakData.currentStreak}
            <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.muted' })}>일째</span>
          </div>
          <div class={flex({ gap: '12px', marginTop: '12px', paddingTop: '12px', borderTopWidth: '1px', borderColor: 'border.hairline' })}>
            <div class={css({ fontSize: '13px', color: 'text.hint' })}>
              최장 <span class={css({ fontWeight: 'semibold', color: 'text.muted' })}>{streakData.longestStreak}일</span>
            </div>
            <div class={css({ fontSize: '13px', color: 'text.hint' })}>
              이번 달 <span class={css({ fontWeight: 'semibold', color: 'text.muted' })}>{streakData.thisMonthDays}일</span>
            </div>
          </div>
        </div>

        {#if weekdayData && maxWeekdayAvg > 0}
          <div class={css(cardStyle, { flex: '1' })}>
            <div class={flex({ justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '16px' })}>
              <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted' })}>요일별</div>
              {#if bestWeekdayIndex >= 0}
                <div class={css({ fontSize: '11px', color: 'text.hint' })}>
                  {weekdayLabels[bestWeekdayIndex]}요일 최다
                </div>
              {/if}
            </div>
            <div class={flex({ alignItems: 'flex-end', gap: '6px', height: '52px' })}>
              {#each weekdayData as data (data.dayIndex)}
                {@const heightPercent = maxWeekdayAvg > 0 ? (data.avgAdditions / maxWeekdayAvg) * 100 : 0}
                {@const isBest = data.dayIndex === bestWeekdayIndex}
                <div class={flex({ flex: '1', flexDirection: 'column', alignItems: 'center', gap: '6px' })}>
                  <div class={flex({ width: 'full', height: '32px', alignItems: 'flex-end' })}>
                    <div
                      style:height="{Math.max(heightPercent, 6)}%"
                      class={css({
                        width: 'full',
                        minHeight: '2px',
                        borderRadius: '3px',
                        backgroundColor: isBest ? 'accent.default' : 'border.default',
                      })}
                    ></div>
                  </div>
                  <div class={css({ fontSize: '11px', fontWeight: 'medium', color: isBest ? 'text.default' : 'text.hint' })}>
                    {data.label}
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>

      <div class={css(cardStyle)}>
        {#if snapshot.goal}
          {@const goal = snapshot.goal}
          {@const progress = dailyGoalStatus(
            snapshot.goalHistory,
            goal.targetCharacterCount,
            snapshot.todayCharacterCountChange,
            snapshot.today,
          )}

          <div class={flex({ justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '12px' })}>
            <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted' })}>일일 목표</div>
            <button
              class={css({ fontSize: '12px', color: 'text.muted', cursor: 'pointer', _hover: { color: 'text.default' } })}
              onclick={() => {
                app.state.userGoalOpen = true;
                mixpanel.track('open_user_goal_modal', { via: 'stats_modal' });
              }}
              type="button"
            >
              자세히
            </button>
          </div>

          <div class={flex({ alignItems: 'center', gap: '10px' })}>
            <ProgressRing
              progress={progress.additions / goal.targetCharacterCount}
              size={28}
              state={progress.achieved ? 'achieved' : 'under'}
            />
            <div class={css({ fontSize: '28px', fontWeight: 'bold', color: 'text.default', fontVariantNumeric: 'tabular-nums' })}>
              {comma(progress.additions)}
              <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.muted' })}>
                / {comma(goal.targetCharacterCount)}자
              </span>
            </div>
          </div>
        {:else}
          <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted', marginBottom: '12px' })}>일일 목표</div>
          <button
            class={css({ fontSize: '14px', color: 'text.muted', cursor: 'pointer', _hover: { color: 'text.default' } })}
            onclick={() => {
              app.state.userGoalOpen = true;
              mixpanel.track('open_user_goal_modal', { via: 'stats_modal' });
            }}
            type="button"
          >
            일일 목표 정하기
          </button>
        {/if}
      </div>

      <div class={css(cardStyle)}>
        <div class={flex({ justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '12px' })}>
          <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted' })}>지난 1년간의 기록</div>
          <div class={flex({ gap: '6px' })}>
            <Button style={css.raw({ gap: '4px' })} onclick={copyActivityImage} size="sm" variant="secondary">
              <Icon icon={CopyIcon} />
              복사
            </Button>
            <Button style={css.raw({ gap: '4px' })} onclick={downloadActivityImage} size="sm" variant="secondary">
              <Icon icon={DownloadIcon} />
              다운로드
            </Button>
          </div>
        </div>
        <ActivityGrid
          characterCountChanges={snapshot.characterCountChanges}
          today={snapshot.today}
          todayCharacterCountChange={snapshot.todayCharacterCountChange}
        />
      </div>

      <div class={css(cardStyle)}>
        <ActivityChart
          characterCountChanges={snapshot.characterCountChanges}
          today={snapshot.today}
          todayCharacterCountChange={snapshot.todayCharacterCountChange}
        />
      </div>
    </div>
  {/if}
</Modal>
