<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { clamp } from '@typie/ui/utils';
  import ActionRow from '$lib/landing/components/ActionRow.svelte';
  import { STATS_LABELS } from '$lib/landing/stats';
  import { graphql } from '$mearie';
  import Headline from './Headline.svelte';
  import Reveal from './Reveal.svelte';
  import StatGrid from './StatGrid.svelte';
  import type { IndexPage_Hero_landingStats$key } from '$mearie';
  import type { StatItem } from './StatGrid.svelte';

  type Props = { landingStats$key: IndexPage_Hero_landingStats$key; scrollTop: number };

  let { landingStats$key, scrollTop }: Props = $props();

  const landingStats = createFragment(
    graphql(`
      fragment IndexPage_Hero_landingStats on LandingStats {
        usersTotal {
          current
          perSecond
        }

        documentsTotal {
          current
          perSecond
        }

        charactersInput {
          current
          perSecond
        }
      }
    `),
    () => landingStats$key,
  );

  const SIDE_FROM = 40;
  const SIDE_TO = 200;

  const items = $derived<readonly StatItem[]>([
    {
      value: Number(landingStats.data.usersTotal.current),
      unit: '명',
      label: STATS_LABELS.usersTotal,
      rate: landingStats.data.usersTotal.perSecond,
    },
    {
      value: Number(landingStats.data.charactersInput.current),
      unit: '자',
      label: STATS_LABELS.charactersInput,
      rate: landingStats.data.charactersInput.perSecond,
    },
    {
      value: Number(landingStats.data.documentsTotal.current),
      unit: '개',
      label: STATS_LABELS.documentsTotal,
      rate: landingStats.data.documentsTotal.perSecond,
    },
  ]);

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
