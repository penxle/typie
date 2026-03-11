<script lang="ts">
  import '@typie/lib/dayjs';

  import { css } from '@typie/styled-system/css';
  import dayjs from 'dayjs';
  import { untrack } from 'svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Dayjs } from 'dayjs';

  type Props = {
    value?: Date;
    onchange?: (date: Date) => void;
    style?: SystemStyleObject;
  };

  let { value = $bindable(), onchange, style }: Props = $props();

  const WEEKDAYS = ['일', '월', '화', '수', '목', '금', '토'] as const;

  let viewDate: Dayjs = $state(value ? dayjs(value) : dayjs());

  $effect(() => {
    if (value) {
      const v = dayjs(value);
      const current = untrack(() => viewDate);
      if (v.month() !== current.month() || v.year() !== current.year()) {
        viewDate = v;
      }
    }
  });

  const grid: Dayjs[] = $derived.by(() => {
    const start = viewDate.startOf('month').startOf('week');
    return Array.from({ length: 42 }, (_, i) => start.add(i, 'day'));
  });

  function prevMonth() {
    viewDate = viewDate.subtract(1, 'month');
  }

  function nextMonth() {
    viewDate = viewDate.add(1, 'month');
  }

  function selectDate(day: Dayjs) {
    if (isOutside(day)) {
      viewDate = day;
      return;
    }
    const date = day.toDate();
    value = date;
    onchange?.(date);
  }

  function isSelected(day: Dayjs): boolean {
    return value != null && day.isSame(value, 'day');
  }

  function isToday(day: Dayjs): boolean {
    return day.isSame(dayjs(), 'day');
  }

  function isOutside(day: Dayjs): boolean {
    return day.month() !== viewDate.month();
  }

  function isWeekend(day: Dayjs): boolean {
    const d = day.day();
    return d === 0 || d === 6;
  }

  function cellColor(day: Dayjs) {
    if (isOutside(day)) return 'text.disabled' as const;
    if (isWeekend(day)) return 'text.faint' as const;
    return 'text.default' as const;
  }

  const navButtonStyle = css.raw({
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    width: '24px',
    height: '24px',
    borderRadius: '4px',
    color: 'text.subtle',
    cursor: 'pointer',
    _hover: { color: 'text.default', backgroundColor: 'surface.subtle' },
  });

  const baseCellStyle = css.raw({
    position: 'relative',
    width: '34px',
    height: '34px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    fontSize: '12px',
    cursor: 'pointer',
    _before: {
      content: '""',
      position: 'absolute',
      width: '30px',
      height: '30px',
      borderRadius: 'full',
    },
  });

  const activeStyle = css.raw({
    color: 'text.bright',
    _before: { backgroundColor: 'text.default' },
  });

  const todayCellStyle = css.raw({
    _before: {
      borderWidth: '1px',
      borderColor: 'border.default',
    },
  });
</script>

<div
  class={css(
    {
      width: '[fit-content]',
      padding: '10px',
      userSelect: 'none',
    },
    style,
  )}
>
  <!-- Header -->
  <div
    class={css({
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'flex-start',
      gap: '4px',
      marginBottom: '8px',
    })}
  >
    <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.faint', paddingLeft: '6px', marginRight: 'auto' })}>
      {viewDate.format('YYYY년 M월')}
    </span>

    <button
      class={css(navButtonStyle, { width: 'auto', paddingX: '6px', fontSize: '11px' })}
      onclick={() => (viewDate = dayjs())}
      type="button"
    >
      오늘
    </button>

    <button class={css(navButtonStyle)} aria-label="이전 월" onclick={prevMonth} type="button">
      <svg fill="none" height="14" viewBox="0 0 20 20" width="14">
        <path d="M12 15l-5-5 5-5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" />
      </svg>
    </button>

    <button class={css(navButtonStyle)} aria-label="다음 월" onclick={nextMonth} type="button">
      <svg fill="none" height="14" viewBox="0 0 20 20" width="14">
        <path d="M8 5l5 5-5 5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" />
      </svg>
    </button>
  </div>

  <!-- Weekday headers -->
  <div class={css({ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', textAlign: 'center', marginBottom: '2px' })}>
    {#each WEEKDAYS as day (day)}
      <span class={css({ fontSize: '11px', color: 'text.faint', width: '34px', textAlign: 'center', lineHeight: '[24px]' })}>{day}</span>
    {/each}
  </div>

  <!-- Date grid -->
  <div class={css({ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', textAlign: 'center' })} aria-label="캘린더" role="grid">
    {#each grid as day (day.format('YYYY-MM-DD'))}
      <button
        class={css(
          baseCellStyle,
          { color: cellColor(day) },
          isOutside(day) && { opacity: '30', cursor: 'default' },
          !isOutside(day) && !isSelected(day) && { _hover: activeStyle },
          !isOutside(day) && isSelected(day) && activeStyle,
          !isOutside(day) && isToday(day) && !isSelected(day) && todayCellStyle,
        )}
        aria-selected={isSelected(day)}
        onclick={() => selectDate(day)}
        role="gridcell"
        type="button"
      >
        <span class={css({ position: 'relative' })}>{day.date()}</span>
      </button>
    {/each}
  </div>
</div>
