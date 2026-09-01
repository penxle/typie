import { createStableContext } from '@typie/ui/context/stable';
import dayjs from 'dayjs';
import { onMount } from 'svelte';
import { delayUntilNextKstDay } from './day-clock';
import type { Dayjs } from 'dayjs';

type DayClock = {
  readonly now: Dayjs;
};

const [getDayClock, setDayClock] = createStableContext<DayClock>('dashboard.DayClock');

export { getDayClock };

export const setupDayClock = (): DayClock => {
  let dayKey = $state(dayjs.kst().format('YYYY-MM-DD'));

  onMount(() => {
    let timer: number | undefined;

    const refresh = () => {
      if (timer !== undefined) window.clearTimeout(timer);

      const now = dayjs.kst();
      dayKey = now.format('YYYY-MM-DD');
      timer = window.setTimeout(refresh, delayUntilNextKstDay(now) + 1);
    };

    const handleVisibilityChange = () => {
      if (document.visibilityState === 'visible') refresh();
    };

    refresh();
    document.addEventListener('visibilitychange', handleVisibilityChange);

    return () => {
      if (timer !== undefined) window.clearTimeout(timer);
      document.removeEventListener('visibilitychange', handleVisibilityChange);
    };
  });

  return setDayClock({
    get now() {
      // Keep the current timestamp, but make consumers reactive to KST day changes.
      void dayKey;
      return dayjs.kst();
    },
  });
};
