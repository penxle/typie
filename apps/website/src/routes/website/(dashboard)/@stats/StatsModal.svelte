<script lang="ts">
  import { createMutation, createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import { comma, downloadFromBase64 } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import CopyIcon from '~icons/lucide/copy';
  import DownloadIcon from '~icons/lucide/download';
  import { graphql } from '$mearie';
  import ActivityChart from './ActivityChart.svelte';
  import ActivityGrid from './ActivityGrid.svelte';

  const app = getAppContext();

  const query = createQuery(
    graphql(`
      query DashboardLayout_StatsModal_Query {
        me @required {
          id
          name
          documentCount
          totalCharacterCount

          characterCountChanges {
            date
            additions
          }

          ...DashboardLayout_Stats_ActivityChart_user
          ...DashboardLayout_Stats_ActivityGrid_user
        }
      }
    `),
    undefined,
    () => ({ skip: !app.state.statsOpen }),
  );

  type StreakData = {
    currentStreak: number;
    longestStreak: number;
    thisMonthDays: number;
    totalDays: number;
    avgCharactersPerDay: number;
  };

  const calculateStreakData = (characterCountChanges: { date: unknown; additions: number }[], totalCharacterCount: number): StreakData => {
    const today = dayjs.kst().startOf('day');
    const activeDates = new Set(
      characterCountChanges.filter((c) => c.additions > 0).map((c) => dayjs(c.date as string).format('YYYY-MM-DD')),
    );

    let currentStreak = 0;
    let checkDate = today;

    if (!activeDates.has(today.format('YYYY-MM-DD'))) {
      checkDate = today.subtract(1, 'day');
    }

    while (activeDates.has(checkDate.format('YYYY-MM-DD'))) {
      currentStreak++;
      checkDate = checkDate.subtract(1, 'day');
    }

    let longestStreak = 0;
    let tempStreak = 0;
    const sortedDates = [...activeDates].toSorted();

    for (let i = 0; i < sortedDates.length; i++) {
      if (i === 0) {
        tempStreak = 1;
      } else {
        const prevDate = dayjs(sortedDates[i - 1]);
        const currDate = dayjs(sortedDates[i]);
        if (currDate.diff(prevDate, 'day') === 1) {
          tempStreak++;
        } else {
          tempStreak = 1;
        }
      }
      longestStreak = Math.max(longestStreak, tempStreak);
    }

    const monthStart = today.startOf('month');
    let thisMonthDays = 0;
    for (const dateStr of activeDates) {
      const date = dayjs(dateStr);
      if (date.isSame(monthStart, 'month')) {
        thisMonthDays++;
      }
    }

    const totalDays = activeDates.size;
    const avgCharactersPerDay = totalDays > 0 ? Math.round(totalCharacterCount / totalDays) : 0;

    return {
      currentStreak,
      longestStreak,
      thisMonthDays,
      totalDays,
      avgCharactersPerDay,
    };
  };

  const streakData = $derived.by(() => {
    if (!query.data) return null;
    return calculateStreakData([...query.data.me.characterCountChanges], query.data.me.totalCharacterCount);
  });

  type WeekdayData = {
    dayIndex: number;
    label: string;
    totalAdditions: number;
    avgAdditions: number;
    count: number;
  };

  const weekdayLabels = ['일', '월', '화', '수', '목', '금', '토'];

  const calculateWeekdayPattern = (characterCountChanges: { date: unknown; additions: number }[]): WeekdayData[] => {
    const weekdayStats = Array.from({ length: 7 }, (_, i) => ({
      dayIndex: i,
      label: weekdayLabels[i],
      totalAdditions: 0,
      count: 0,
    }));

    for (const change of characterCountChanges) {
      if (change.additions > 0) {
        const dayOfWeek = dayjs(change.date as string).day();
        weekdayStats[dayOfWeek].totalAdditions += change.additions;
        weekdayStats[dayOfWeek].count++;
      }
    }

    return weekdayStats.map((stat) => ({
      ...stat,
      avgAdditions: stat.count > 0 ? Math.round(stat.totalAdditions / stat.count) : 0,
    }));
  };

  const weekdayData = $derived.by(() => {
    if (!query.data) return null;
    return calculateWeekdayPattern([...query.data.me.characterCountChanges]);
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

  const loaded = $derived(app.state.statsOpen && !!query.data && !query.loading);

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
    downloadFromBase64(b64, `${query.data?.me.name ?? '타이피'} - 나의 글쓰기 발자취.png`, 'image/png');

    Toast.success('이미지가 다운로드되었어요.');
  };

  const cardStyle = css.raw({
    padding: '20px',
    borderRadius: '16px',
    backgroundColor: 'surface.default',
    borderWidth: '1px',
    borderColor: 'border.subtle',
  });
</script>

<Modal
  style={css.raw({
    gap: '20px',
    maxWidth: '720px',
    padding: '24px',
    backgroundColor: 'surface.subtle',
  })}
  loading={!loaded || !query}
  onclose={() => {
    app.state.statsOpen = false;
  }}
  open={app.state.statsOpen}
>
  {#if loaded && query.data && streakData}
    <div class={css({ fontSize: '17px', fontWeight: 'semibold', color: 'text.default' })}>나의 글쓰기 통계</div>

    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <div class={flex({ gap: '12px' })}>
        <div class={css(cardStyle, { flex: '1' })}>
          <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint', marginBottom: '8px' })}>총 글자</div>
          <div class={css({ fontSize: '28px', fontWeight: 'bold', color: 'text.default', fontVariantNumeric: 'tabular-nums' })}>
            {comma(query.data.me.totalCharacterCount)}
          </div>
        </div>

        <div class={css(cardStyle, { flex: '1' })}>
          <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint', marginBottom: '8px' })}>총 문서</div>
          <div class={css({ fontSize: '28px', fontWeight: 'bold', color: 'text.default', fontVariantNumeric: 'tabular-nums' })}>
            {query.data.me.documentCount}
          </div>
        </div>

        <div class={css(cardStyle, { flex: '1' })}>
          <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint', marginBottom: '8px' })}>활동일</div>
          <div class={css({ fontSize: '28px', fontWeight: 'bold', color: 'text.default', fontVariantNumeric: 'tabular-nums' })}>
            {streakData.totalDays}
          </div>
        </div>
      </div>

      <div class={flex({ gap: '12px' })}>
        <div class={css(cardStyle, { flex: '1' })}>
          <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint', marginBottom: '8px' })}>연속 기록</div>
          <div class={css({ fontSize: '28px', fontWeight: 'bold', color: 'text.default', fontVariantNumeric: 'tabular-nums' })}>
            {streakData.currentStreak}
            <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.faint' })}>일째</span>
          </div>
          <div class={flex({ gap: '12px', marginTop: '12px', paddingTop: '12px', borderTopWidth: '1px', borderColor: 'border.subtle' })}>
            <div class={css({ fontSize: '13px', color: 'text.faint' })}>
              최장 <span class={css({ fontWeight: 'semibold', color: 'text.muted' })}>{streakData.longestStreak}일</span>
            </div>
            <div class={css({ fontSize: '13px', color: 'text.faint' })}>
              이번 달 <span class={css({ fontWeight: 'semibold', color: 'text.muted' })}>{streakData.thisMonthDays}일</span>
            </div>
          </div>
        </div>

        {#if weekdayData && maxWeekdayAvg > 0}
          <div class={css(cardStyle, { flex: '1' })}>
            <div class={flex({ justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '16px' })}>
              <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>요일별</div>
              {#if bestWeekdayIndex >= 0}
                <div class={css({ fontSize: '11px', color: 'text.faint' })}>
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
                        backgroundColor: isBest ? 'text.default' : 'border.default',
                      })}
                    ></div>
                  </div>
                  <div class={css({ fontSize: '11px', fontWeight: 'medium', color: isBest ? 'text.default' : 'text.faint' })}>
                    {data.label}
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>

      <div class={css(cardStyle)}>
        <div class={flex({ justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '12px' })}>
          <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>지난 1년간의 기록</div>
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
        <ActivityGrid user$key={query.data.me} />
      </div>

      <div class={css(cardStyle)}>
        <ActivityChart user$key={query.data.me} />
      </div>
    </div>
  {/if}
</Modal>
