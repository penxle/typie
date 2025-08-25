<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import { fade } from 'svelte/transition';
  import { fragment, graphql } from '$graphql';
  import type { DashboardLayout_Stats_ActivityChart_user } from '$graphql';

  type DayData = {
    date: dayjs.Dayjs;
    additions: number;
    deletions: number;
    total: number;
  };

  type Props = {
    $user: DashboardLayout_Stats_ActivityChart_user;
  };

  const { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_Stats_ActivityChart_user on User {
        id

        characterCountChanges {
          date
          additions
          deletions
        }
      }
    `),
  );

  let hoverData = $state<DayData & { element: HTMLElement }>();
  let isHoveringCompressedBar = $state(false);
  let showAdditions = $state(true);
  let showDeletions = $state(true);

  const daysData = $derived.by<DayData[]>(() => {
    const data: DayData[] = [];
    const endDate = dayjs.kst().startOf('day');
    const startDate = endDate.subtract(89, 'days');

    const changesByDate: Record<string, { additions: number; deletions: number }> = {};

    for (const change of $user.characterCountChanges) {
      const date = dayjs(change.date).format('YYYY-MM-DD');
      changesByDate[date] = {
        additions: change.additions,
        deletions: change.deletions,
      };
    }

    let currentDate = startDate;
    while (!currentDate.isAfter(endDate)) {
      const dateKey = currentDate.format('YYYY-MM-DD');
      const dayChanges = changesByDate[dateKey] || { additions: 0, deletions: 0 };

      data.push({
        date: currentDate,
        additions: dayChanges.additions,
        deletions: Math.abs(dayChanges.deletions),
        total: dayChanges.additions + Math.abs(dayChanges.deletions),
      });

      currentDate = currentDate.add(1, 'day');
    }

    return data;
  });

  const compressionRange = $derived.by(() => {
    const allValues: number[] = [];
    daysData.forEach((d) => {
      const effectiveTotal = (showAdditions ? d.additions : 0) + (showDeletions ? d.deletions : 0);
      if (effectiveTotal > 0) allValues.push(effectiveTotal);
    });

    if (allValues.length === 0) return { start: 0, end: 0 };

    const sortedValues = [...allValues].sort((a, b) => a - b);
    const maxValue = sortedValues.at(-1);

    if (!maxValue) return { start: 0, end: 0 };

    const threshold = maxValue * 0.25;

    let rangeStart = 0;
    let rangeEnd = 0;
    let maxGapSize = 0;

    for (let i = 0; i < sortedValues.length - 1; i++) {
      const currentValue = sortedValues[i];
      const nextValue = sortedValues[i + 1];
      const gap = nextValue - currentValue;

      if (gap > threshold && gap > maxGapSize) {
        maxGapSize = gap;
        rangeStart = currentValue;
        rangeEnd = nextValue;
      }
    }

    if (maxGapSize <= threshold) {
      return { start: 0, end: 0 };
    }

    const buffer = maxValue * 0.05;

    return {
      start: rangeStart + buffer,
      end: rangeEnd - buffer,
    };
  });

  const { anchor, floating } = createFloatingActions({
    placement: 'top',
    offset: 8,
  });

  const additionColor = css.raw({ backgroundColor: { base: 'brand.400', _dark: 'dark.brand.600' } });
  const deletionColor = css.raw({ backgroundColor: { base: 'gray.400', _dark: 'dark.gray.500' } });

  const maxVal = $derived(Math.max(...daysData.map((d) => (showAdditions ? d.additions : 0) + (showDeletions ? d.deletions : 0)), 1));

  const hasCompression = $derived(!isHoveringCompressedBar && compressionRange.end > compressionRange.start);

  const calculateBarHeights = (data: DayData) => {
    const getCompressedHeight = (value: number): number => {
      if (!hasCompression || value === 0) return value;

      let height = 0;

      if (value <= compressionRange.start) {
        height = value;
      } else if (value <= compressionRange.end) {
        height = compressionRange.start;
        const compressedPortion = (value - compressionRange.start) / (compressionRange.end - compressionRange.start);
        height += compressedPortion * 20;
      } else {
        height = compressionRange.start + 20 + (value - compressionRange.end);
      }

      return height;
    };

    const maxCompressedHeight = hasCompression
      ? Math.max(
          ...daysData.map((d) => {
            const totalValue = (showAdditions ? d.additions : 0) + (showDeletions ? d.deletions : 0);
            return getCompressedHeight(totalValue);
          }),
        )
      : maxVal;

    const scaleFactor = hasCompression && maxCompressedHeight > 0 ? 140 / maxCompressedHeight : 1;

    const calculateAdditionHeight = () => {
      if (data.additions === 0 || !showAdditions) {
        return 0;
      }

      if (!hasCompression) {
        return (data.additions / maxVal) * 140;
      }

      const value = data.additions;
      let height = 0;

      if (value <= compressionRange.start) {
        height = value;
      } else if (value <= compressionRange.end) {
        height = compressionRange.start;
        const compressedPortion = value - compressionRange.start;
        const compressRatio = compressedPortion / (compressionRange.end - compressionRange.start);
        height += compressRatio * 20;
      } else {
        height = compressionRange.start;
        height += 20;
        const abovePortion = value - compressionRange.end;
        height += abovePortion;
      }

      return height * scaleFactor;
    };

    const calculateDeletionHeight = () => {
      if (data.deletions === 0 || !showDeletions) {
        return 0;
      }

      if (!hasCompression) {
        return (data.deletions / maxVal) * 140;
      }

      let height = 0;
      const startValue = showAdditions ? data.additions : 0;
      const endValue = startValue + data.deletions;

      if (startValue >= compressionRange.end) {
        height = data.deletions;
      } else if (startValue < compressionRange.end && endValue > compressionRange.start) {
        if (startValue < compressionRange.start) {
          const beforeCompress = Math.min(compressionRange.start - startValue, data.deletions);
          height += beforeCompress;
        }

        const compressStart = Math.max(startValue, compressionRange.start);
        const compressEnd = Math.min(endValue, compressionRange.end);
        if (compressEnd > compressStart) {
          const compressedPortion = compressEnd - compressStart;
          const compressRatio = compressedPortion / (compressionRange.end - compressionRange.start);
          height += compressRatio * 20;
        }

        if (endValue > compressionRange.end) {
          const afterCompress = endValue - compressionRange.end;
          height += afterCompress;
        }
      } else {
        height = data.deletions;
      }

      return height * scaleFactor;
    };

    const additionHeight = calculateAdditionHeight();
    const deletionHeight = calculateDeletionHeight();

    const effectiveAdditions = showAdditions ? data.additions : 0;
    const effectiveDeletions = showDeletions ? data.deletions : 0;
    const totalValue = effectiveAdditions + effectiveDeletions;
    const hasCompressionInBar = hasCompression && totalValue > compressionRange.start;

    let wavePosition = 0;
    if (hasCompressionInBar) {
      if (compressionRange.start <= effectiveAdditions) {
        wavePosition = compressionRange.start * scaleFactor;
      } else {
        const additionFullHeight = effectiveAdditions * scaleFactor;
        const deletionStartOfCompression = compressionRange.start - effectiveAdditions;
        wavePosition = additionFullHeight + deletionStartOfCompression * scaleFactor;
      }
    }

    return {
      additionHeight,
      deletionHeight,
      wavePosition,
      effectiveTotal: totalValue,
      showWave: hasCompression && totalValue > compressionRange.start,
      isCompressedBar: compressionRange.end > compressionRange.start && totalValue > compressionRange.start,
    };
  };

  $effect(() => {
    if (hoverData) {
      anchor(hoverData.element);
    }
  });
</script>

<div class={flex({ flexDirection: 'column', gap: '16px', width: 'full' })}>
  <div class={flex({ justifyContent: 'space-between', alignItems: 'center' })}>
    <div class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'text.faint' })}>지난 3개월의 그래프</div>
    <div class={flex({ alignItems: 'center', gap: '12px' })}>
      {#if compressionRange.end > compressionRange.start && !isHoveringCompressedBar}
        <span class={css({ fontSize: '11px', color: 'text.faint' })}>
          생략: {comma(Math.round(compressionRange.start))}자 – {comma(Math.round(compressionRange.end))}자 구간
        </span>
      {/if}
    </div>
  </div>

  <div class={flex({ flexDirection: 'column', gap: '4px' })}>
    <div class={flex({ alignItems: 'flex-end', gap: '2px', height: '141px', overflow: 'hidden', position: 'relative' })}>
      <!-- 배경 격자 -->
      <div class={css({ position: 'absolute', inset: '0', pointerEvents: 'none' })}>
        {#each [1, 2, 3, 4, 5] as i (i)}
          <div
            style:bottom="{i * 28}px"
            class={css({
              position: 'absolute',
              left: '0',
              right: '0',
              height: '1px',
              backgroundColor: { base: 'gray.200', _dark: 'dark.gray.700' },
              opacity: '50',
            })}
          ></div>
        {/each}
      </div>

      {#each daysData as data (data.date.format('YYYY-MM-DD'))}
        {@const barHeights = calculateBarHeights(data)}

        <div class={flex({ flex: '1', flexDirection: 'column', justifyContent: 'flex-end', position: 'relative' })}>
          <div
            class={flex({
              flexDirection: 'column',
              width: 'full',
              minHeight: '140px',
              justifyContent: 'flex-end',
              cursor: 'pointer',
            })}
            onpointerenter={(e) => {
              hoverData = { ...data, element: e.currentTarget };
              if (barHeights.isCompressedBar) {
                isHoveringCompressedBar = true;
              }
            }}
            onpointerleave={() => {
              hoverData = undefined;
              isHoveringCompressedBar = false;
            }}
          >
            {#if barHeights.showWave}
              <div
                style:bottom="{barHeights.wavePosition - 4}px"
                class={css({
                  position: 'absolute',
                  left: '-2px',
                  right: '-2px',
                  height: '8px',
                  pointerEvents: 'none',
                  zIndex: '30',
                })}
              >
                <svg height="8" preserveAspectRatio="none" width="100%">
                  <!-- Top black wave -->
                  <path
                    d="M 0,2 L 3,0 L 6,2 L 9,0 L 12,2 L 15,0 L 18,2 L 21,0 L 24,2 L 27,0 L 30,2 L 33,0 L 36,2 L 39,0 L 42,2 L 45,0 L 48,2 L 51,0 L 54,2 L 57,0 L 60,2 L 63,0 L 66,2 L 69,0 L 72,2 L 75,0 L 78,2 L 81,0 L 84,2 L 87,0 L 90,2 L 93,0 L 96,2 L 99,0"
                    fill="none"
                    opacity="1"
                    stroke="black"
                    stroke-width="1"
                  />
                  <!-- Middle white wave -->
                  <path
                    d="M 0,4 L 3,2 L 6,4 L 9,2 L 12,4 L 15,2 L 18,4 L 21,2 L 24,4 L 27,2 L 30,4 L 33,2 L 36,4 L 39,2 L 42,4 L 45,2 L 48,4 L 51,2 L 54,4 L 57,2 L 60,4 L 63,2 L 66,4 L 69,2 L 72,4 L 75,2 L 78,4 L 81,2 L 84,4 L 87,2 L 90,4 L 93,2 L 96,4 L 99,2"
                    fill="none"
                    stroke="white"
                    stroke-width="2"
                  />
                  <!-- Bottom black wave -->
                  <path
                    d="M 0,6 L 3,4 L 6,6 L 9,4 L 12,6 L 15,4 L 18,6 L 21,4 L 24,6 L 27,4 L 30,6 L 33,4 L 36,6 L 39,4 L 42,6 L 45,4 L 48,6 L 51,4 L 54,6 L 57,4 L 60,6 L 63,4 L 66,6 L 69,4 L 72,6 L 75,4 L 78,6 L 81,4 L 84,6 L 87,4 L 90,6 L 93,4 L 96,6 L 99,4"
                    fill="none"
                    opacity="1"
                    stroke="black"
                    stroke-width="1"
                  />
                </svg>
              </div>
            {/if}

            {#if data.deletions > 0 && showDeletions}
              <div
                style:height="{Math.max(barHeights.deletionHeight, 1)}px"
                class={css({
                  width: 'full',
                  backgroundColor: deletionColor.backgroundColor,
                  borderRadius: '1px',
                  transition: 'all',
                  position: 'relative',
                })}
              ></div>
            {/if}

            {#if data.additions > 0 && showAdditions}
              <div
                style:height="{Math.max(barHeights.additionHeight, 1)}px"
                class={css({
                  width: 'full',
                  backgroundColor: additionColor.backgroundColor,
                  borderRadius: '1px',
                  transition: 'all',
                  marginTop: data.deletions > 0 && showDeletions ? '1px' : '0',
                  position: 'relative',
                })}
              ></div>
            {/if}

            {#if barHeights.effectiveTotal === 0}
              <div
                class={css({
                  width: 'full',
                  height: '1px',
                  backgroundColor: { base: 'gray.200', _dark: 'dark.gray.700' },
                  borderRadius: '1px',
                })}
              ></div>
            {/if}
          </div>
        </div>
      {/each}
    </div>

    <!-- X축 레이블 -->
    <div class={flex({ position: 'relative', height: '20px' })}>
      {#each daysData as data, index (data.date.format('YYYY-MM-DD'))}
        {@const isFirstDay = index === 0}
        {@const isLastDay = index === daysData.length - 1}
        {@const isFirstOfMonth = data.date.date() === 1}
        {@const shouldShowLabel = isFirstDay || isLastDay || isFirstOfMonth}

        {#if shouldShowLabel}
          {@const prevLabelIndex = (() => {
            for (let i = index - 1; i >= 0; i--) {
              if (i === 0 || daysData[i].date.date() === 1) return i;
            }
            return -1;
          })()}
          {@const nextLabelIndex = (() => {
            for (let i = index + 1; i < daysData.length; i++) {
              if (i === daysData.length - 1 || daysData[i].date.date() === 1) return i;
            }
            return daysData.length;
          })()}

          {@const shouldCheckDistance = !isFirstDay && !isLastDay}
          {@const tooCloseToPrev = shouldCheckDistance && prevLabelIndex >= 0 && index - prevLabelIndex < 5}
          {@const tooCloseToNext = shouldCheckDistance && nextLabelIndex < daysData.length && nextLabelIndex - index < 5}

          {#if !tooCloseToPrev && !tooCloseToNext}
            <div
              style:left="{(index / daysData.length) * 100}%"
              class={css({
                position: 'absolute',
                fontSize: '12px',
                color: 'text.faint',
                transform: isFirstDay ? 'translateX(0)' : 'translateX(-50%)',
                whiteSpace: 'nowrap',
              })}
            >
              {data.date.format('M/D')}
            </div>
          {/if}
        {/if}
      {/each}
    </div>
  </div>
</div>
<div class={flex({ gap: '16px', fontSize: '12px', alignSelf: 'flex-end' })}>
  <button
    class={flex({ alignItems: 'center', gap: '6px', userSelect: 'none' })}
    onclick={() => (showAdditions = !showAdditions)}
    type="button"
  >
    <div
      class={css({
        width: '12px',
        height: '12px',
        backgroundColor: additionColor.backgroundColor,
        borderRadius: '2px',
        opacity: showAdditions ? '100' : '30',
      })}
    ></div>
    <span class={css({ color: showAdditions ? 'text.muted' : 'text.faint' })}>입력한 글자</span>
  </button>
  <button
    class={flex({ alignItems: 'center', gap: '6px', userSelect: 'none' })}
    onclick={() => (showDeletions = !showDeletions)}
    type="button"
  >
    <div
      class={css({
        width: '12px',
        height: '12px',
        backgroundColor: deletionColor.backgroundColor,
        borderRadius: '2px',
        opacity: showDeletions ? '100' : '30',
      })}
    ></div>
    <span class={css({ color: showDeletions ? 'text.muted' : 'text.faint' })}>지운 글자</span>
  </button>
</div>

{#if hoverData}
  <div
    class={flex({
      flexDirection: 'column',
      borderRadius: '6px',
      paddingX: '12px',
      paddingY: '8px',
      color: 'text.bright',
      backgroundColor: 'surface.dark',
      zIndex: 'modal',
    })}
    use:floating
    in:fade={{ duration: 100 }}
  >
    <div class={css({ fontSize: '13px', fontWeight: 'semibold' })}>
      {hoverData.date.format('YYYY년 M월 D일')}
    </div>

    {#if hoverData.additions > 0}
      <div class={css({ fontSize: '12px' })}>입력: {comma(hoverData.additions)}자</div>
    {/if}

    {#if hoverData.deletions > 0}
      <div class={css({ fontSize: '12px' })}>지움: {comma(hoverData.deletions)}자</div>
    {/if}

    {#if hoverData.total === 0}
      <div class={css({ fontSize: '12px' })}>기록이 없어요</div>
    {/if}
  </div>
{/if}
