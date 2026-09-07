<script lang="ts">
  import '@typie/lib/dayjs';

  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import { nextSchedulableParts, scheduleDayPassed, scheduleHourPassed, scheduleMinutePassed } from '$lib/publication/publish-form';
  import type { Dayjs } from 'dayjs';
  import type { ScheduleParts } from '$lib/publication/publish-form';

  type Props = {
    parts: ScheduleParts;
  };

  let { parts = $bindable() }: Props = $props();

  const WEEKDAYS = ['일', '월', '화', '수', '목', '금', '토'] as const;

  const pad = (value: number) => String(value).padStart(2, '0');

  let now = $state(dayjs());
  const today = $derived(now.kst().startOf('day'));

  $effect(() => {
    const timer = setInterval(() => {
      now = dayjs();

      const next = nextSchedulableParts(parts, now);
      if (next !== parts) parts = next;
    }, 10_000);

    return () => clearInterval(timer);
  });

  const hour = $derived(Number(parts.time.slice(0, 2)));
  const minute = $derived(Number(parts.time.slice(3, 5)));

  let viewDate = $state<Dayjs>(dayjs(parts.date).startOf('month'));

  let hourEl = $state<HTMLElement>();
  let minuteEl = $state<HTMLElement>();

  const grid = $derived.by(() => {
    const start = viewDate.startOf('month').startOf('week');
    return Array.from({ length: 42 }, (_, index) => start.add(index, 'day'));
  });

  const showToday = $derived(!dayjs(parts.date).isSame(today, 'day') || !viewDate.isSame(today, 'month'));

  const apply = (next: ScheduleParts) => {
    parts = nextSchedulableParts(next, now);
  };

  const selectDate = (day: Dayjs) => {
    apply({ date: day.startOf('day').toDate(), time: parts.time });
    viewDate = day.startOf('month');
  };

  const selectTime = (nextHour: number, nextMinute: number) => {
    apply({ date: parts.date, time: `${pad(nextHour)}:${pad(nextMinute)}` });
  };

  const goToToday = () => {
    apply({ date: today.toDate(), time: parts.time });
    viewDate = today.startOf('month');
  };

  const scrollIntoColumn = (container: HTMLElement | undefined) => {
    const selected = container?.querySelector<HTMLElement>('[data-selected="true"]');
    if (!container || !selected) return;

    container.scrollTop = selected.offsetTop - container.clientHeight / 2 + selected.clientHeight / 2;
  };

  $effect(() => {
    const frame = requestAnimationFrame(() => {
      scrollIntoColumn(hourEl);
      scrollIntoColumn(minuteEl);
    });

    return () => cancelAnimationFrame(frame);
  });

  const navButtonStyle = css.raw({
    flexShrink: '0',
    size: '24px',
    borderRadius: '4px',
    color: 'text.muted',
    transition: 'common',
    _hover: { color: 'text.default', backgroundColor: 'surface.hover' },
  });

  const cellStyle = css.raw({
    position: 'relative',
    display: 'grid',
    placeItems: 'center',
    height: '34px',
    fontSize: '12px',
    color: 'text.default',
    fontVariantNumeric: 'tabular-nums',
    _before: {
      content: '""',
      position: 'absolute',
      size: '30px',
      borderRadius: 'full',
      transition: 'common',
    },
    _hover: { _before: { backgroundColor: 'surface.hover' } },
  });

  const slotStyle = css.raw({
    display: 'block',
    width: 'full',
    paddingY: '5px',
    borderRadius: '4px',
    fontSize: '12px',
    color: 'text.muted',
    textAlign: 'center',
    fontVariantNumeric: 'tabular-nums',
    transition: 'common',
    _hover: { color: 'text.default', backgroundColor: 'surface.hover' },
  });

  const slotSelectedStyle = css.raw({
    color: 'text.on.inverse',
    backgroundColor: 'surface.inverse',
    _hover: { color: 'text.on.inverse', backgroundColor: 'surface.inverse' },
  });

  const passedStyle = css.raw({
    color: 'text.hint',
    opacity: '40',
    cursor: 'default',
    _hover: { color: 'text.hint', backgroundColor: 'transparent', _before: { backgroundColor: 'transparent' } },
  });

  const columnStyle = css.raw({
    position: 'relative',
    flex: '1',
    minWidth: '0',
    overflowY: 'auto',
    paddingRight: '2px',
  });
</script>

<div class={flex({ alignItems: 'flex-start', gap: '10px' })}>
  <div class={css({ flexShrink: '0' })}>
    <div class={flex({ position: 'relative', alignItems: 'center', gap: '4px', paddingX: '2px', paddingBottom: '6px' })}>
      <button class={center(navButtonStyle)} aria-label="이전 달" onclick={() => (viewDate = viewDate.subtract(1, 'month'))} type="button">
        <Icon icon={ChevronLeftIcon} size={14} />
      </button>

      <span
        class={css({
          position: 'absolute',
          left: '0',
          right: '0',
          fontSize: '13px',
          fontWeight: 'semibold',
          textAlign: 'center',
          fontVariantNumeric: 'tabular-nums',
          pointerEvents: 'none',
        })}
      >
        {viewDate.format('YYYY년 M월')}
      </span>

      <span class={css({ flex: '1' })}></span>

      <button
        class={css({
          flexShrink: '0',
          paddingX: '6px',
          paddingY: '3px',
          borderRadius: '4px',
          fontSize: '11px',
          color: 'text.muted',
          transition: 'common',
          visibility: showToday ? 'visible' : 'hidden',
          _hover: { color: 'text.default', backgroundColor: 'surface.hover' },
        })}
        aria-hidden={!showToday}
        onclick={goToToday}
        tabindex={showToday ? 0 : -1}
        type="button"
      >
        오늘
      </button>

      <button class={center(navButtonStyle)} aria-label="다음 달" onclick={() => (viewDate = viewDate.add(1, 'month'))} type="button">
        <Icon icon={ChevronRightIcon} size={14} />
      </button>
    </div>

    <div class={css({ display: 'grid', gridTemplateColumns: '[repeat(7, 34px)]' })}>
      {#each WEEKDAYS as weekday (weekday)}
        <span class={center({ height: '24px', fontSize: '11px', color: 'text.hint' })}>{weekday}</span>
      {/each}

      {#each grid as day (day.format('YYYY-MM-DD'))}
        {@const passed = scheduleDayPassed(day.toDate(), now)}
        {@const outside = day.month() !== viewDate.month()}
        {@const selected = day.isSame(dayjs(parts.date), 'day')}
        <button
          class={css(
            cellStyle,
            (day.day() === 0 || day.day() === 6) && { color: 'text.muted' },
            outside && { color: 'text.hint' },
            day.isSame(today, 'day') && !selected && { _before: { borderWidth: '1px', borderColor: 'accent.default' } },
            selected && {
              color: 'text.on.inverse',
              _before: { backgroundColor: 'surface.inverse' },
              _hover: { _before: { backgroundColor: 'surface.inverse' } },
            },
            passed && passedStyle,
          )}
          aria-pressed={selected}
          disabled={passed}
          onclick={() => selectDate(day)}
          type="button"
        >
          <span class={css({ position: 'relative' })}>{day.date()}</span>
        </button>
      {/each}
    </div>
  </div>

  <div
    class={flex({
      flexDirection: 'column',
      flexShrink: '0',
      width: '136px',
      height: '264px',
      paddingLeft: '10px',
      borderLeftWidth: '1px',
      borderColor: 'border.hairline',
    })}
  >
    <div class={flex({ gap: '6px', paddingBottom: '4px' })}>
      <span class={css({ flex: '1', fontSize: '11px', color: 'text.hint', textAlign: 'center' })}>시</span>
      <span class={css({ flex: '1', fontSize: '11px', color: 'text.hint', textAlign: 'center' })}>분</span>
    </div>

    <div class={flex({ gap: '6px', flex: '1', minHeight: '0' })}>
      <div bind:this={hourEl} class={css(columnStyle)}>
        {#each Array.from({ length: 24 }, (_, value) => value) as value (value)}
          {@const passed = scheduleHourPassed(parts.date, value, now)}
          <button
            class={css(slotStyle, hour === value && slotSelectedStyle, passed && passedStyle)}
            data-selected={hour === value}
            disabled={passed}
            onclick={() => selectTime(value, minute)}
            type="button"
          >
            {pad(value)}
          </button>
        {/each}
      </div>

      <div bind:this={minuteEl} class={css(columnStyle)}>
        {#each Array.from({ length: 60 }, (_, value) => value) as value (value)}
          {@const passed = scheduleMinutePassed(parts.date, hour, value, now)}
          <button
            class={css(slotStyle, minute === value && slotSelectedStyle, passed && passedStyle)}
            data-selected={minute === value}
            disabled={passed}
            onclick={() => selectTime(hour, value)}
            type="button"
          >
            {pad(value)}
          </button>
        {/each}
      </div>
    </div>
  </div>
</div>
