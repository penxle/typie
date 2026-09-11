<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { clamp } from '@typie/ui/utils';
  import ActionRow from '$lib/components/ActionRow.svelte';
  import { perSecond, STATS_LABELS } from '$lib/stats';
  import Headline from './Headline.svelte';
  import Reveal from './Reveal.svelte';
  import StatGrid from './StatGrid.svelte';
  import type { Stats } from '$lib/stats';
  import type { StatItem } from './StatGrid.svelte';

  type Props = { stats: Stats | null; scrollTop: number };

  let { stats, scrollTop }: Props = $props();

  const SIDE_FROM = 40;
  const SIDE_TO = 200;

  const items = $derived<readonly StatItem[]>(
    stats
      ? [
          { value: stats.usersTotal.current, unit: '명', label: STATS_LABELS.usersTotal, rate: perSecond(stats.usersTotal) },
          {
            value: stats.charactersInput.current,
            unit: '자',
            label: STATS_LABELS.charactersInput,
            rate: perSecond(stats.charactersInput),
          },
          {
            value: stats.documentsTotal.current,
            unit: '개',
            label: STATS_LABELS.documentsTotal,
            rate: perSecond(stats.documentsTotal),
          },
        ]
      : [],
  );

  const sideProgress = $derived(clamp((scrollTop - SIDE_FROM) / (SIDE_TO - SIDE_FROM), 0, 1));

  let done = $state(false);

  const sectionClass = css({
    display: 'grid',
    gridTemplateRows: '[1fr auto]',
    minHeight: '[100dvh]',
    paddingX: { base: '20px', lg: '64px' },
    paddingTop: '96px',
    paddingBottom: '48px',
  });
  const bodyClass = css({
    display: 'grid',
    alignContent: 'center',
    justifyItems: 'center',
    width: 'full',
    maxWidth: '[1400px]',
    marginX: 'auto',
  });
  const actionClass = css({ display: 'grid', justifyItems: 'center', marginTop: { base: '40px', md: '48px', xl: '64px' } });
  const gridHostClass = css({ width: 'full', maxWidth: '[1400px]', marginX: 'auto', paddingTop: '48px' });
</script>

<section class={sectionClass}>
  <div class={bodyClass}>
    <Headline bind:done />
    <div class={actionClass}>
      <Reveal shown={done}>
        <ActionRow badge="2주 무료" justify="center" marginTop="0px" />
      </Reveal>
    </div>
  </div>
  <div class={gridHostClass}>
    <StatGrid {items} shown={done} {sideProgress} />
  </div>
</section>
