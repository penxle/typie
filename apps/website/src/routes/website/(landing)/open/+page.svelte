<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Helmet } from '@typie/ui/components';
  import { hydrateQuery } from '$lib/graphql';
  import Page from '$lib/landing/components/Page.svelte';
  import StartButton from '$lib/landing/components/StartButton.svelte';
  import MetricCards from './MetricCards.svelte';
  import { COPY, formatWithUnit, LIFETIMES, WHY } from './open';
  import type { PageData } from './$types';
  import type { OpenStats } from './open';

  type Props = { data: PageData };

  let { data }: Props = $props();

  const query = $derived(hydrateQuery(() => data.query));
  const stats = $derived<OpenStats>(query.data.stats);

  const coreClass = css({ marginTop: { base: '56px', lg: '80px' } });
  const sectionClass = css({ paddingTop: { base: '96px', lg: '128px' } });
  const headingClass = css({
    fontSize: { base: '[28px]', md: '[36px]' },
    fontWeight: 'bold',
    letterSpacing: '[-0.02em]',
    lineHeight: '[1.25]',
  });
  const lifetimeGridClass = css({
    display: 'grid',
    gridTemplateColumns: { base: '1fr', md: '[repeat(2, minmax(0, 1fr))]' },
    marginTop: { base: '32px', md: '48px' },
    borderTopWidth: '1px',
    borderTopColor: 'border.hairline',
  });
  const lifetimeCellClass = css({
    paddingY: { base: '28px', md: '40px' },
    borderBottomWidth: '1px',
    borderBottomColor: 'border.hairline',
    md: {
      '&:nth-child(odd)': { paddingRight: '48px', borderRightWidth: '1px', borderRightColor: 'border.hairline' },
      '&:nth-child(even)': { paddingLeft: '48px' },
    },
  });
  const lifetimeValueClass = css({
    fontSize: { base: '[44px]', md: '[56px]' },
    fontWeight: 'bold',
    lineHeight: '[1]',
    letterSpacing: '[-0.03em]',
    fontVariantNumeric: 'tabular-nums',
  });
  const lifetimeDescClass = css({ marginTop: '12px', fontSize: '14px', color: 'text.muted' });
  const whyGridClass = css({
    display: 'grid',
    gridTemplateColumns: { base: '1fr', lg: '[minmax(0, 1fr) minmax(0, 1fr)]' },
    gap: { base: '32px', lg: '80px' },
    alignItems: 'start',
  });
  const whySubClass = css({ marginTop: '20px', fontSize: { base: '15px', md: '17px' }, lineHeight: '[1.65]', color: 'text.muted' });
  const whyListClass = css({ display: 'grid', gap: '28px' });
  const whyTitleClass = css({ fontSize: { base: '17px', md: '18px' }, fontWeight: 'bold', letterSpacing: '[-0.01em]' });
  const whyBodyClass = css({ marginTop: '8px', fontSize: '15px', lineHeight: '[1.7]', color: 'text.muted' });
  const closingClass = css({ display: 'grid', justifyItems: 'center', paddingTop: { base: '112px', lg: '160px' }, textAlign: 'center' });
  const closingTitleClass = css({
    fontSize: { base: '[36px]', md: '[56px]' },
    fontWeight: 'bold',
    letterSpacing: '[-0.025em]',
    lineHeight: '[1.15]',
  });
  const closingSubClass = css({ marginTop: '16px', fontSize: { base: '15px', md: '17px' }, lineHeight: '[1.6]', color: 'text.muted' });
</script>

<Helmet description={COPY.description} title={COPY.pageTitle} />

<Page sub={COPY.sub}>
  {#snippet title()}
    {COPY.title[0]}
    <br />
    <span class={css({ color: 'text.muted' })}>{COPY.title[1]}</span>
  {/snippet}

  <section class={coreClass}>
    <h2 class={headingClass}>{COPY.coreTitle}</h2>
    <MetricCards {stats} />
  </section>

  <section class={sectionClass}>
    <h2 class={headingClass}>{COPY.lifetimeTitle}</h2>
    <div class={lifetimeGridClass}>
      {#each LIFETIMES as item (item.key)}
        <div class={lifetimeCellClass}>
          <p class={lifetimeValueClass}>{formatWithUnit(stats[item.key].current, item.unit)}</p>
          <p class={lifetimeDescClass}>{item.description}</p>
        </div>
      {/each}
    </div>
  </section>

  <section class={sectionClass}>
    <div class={whyGridClass}>
      <div>
        <h2 class={headingClass}>{COPY.whyTitle}</h2>
        <p class={whySubClass}>
          {COPY.whySub[0]}
          <br />
          {COPY.whySub[1]}
        </p>
      </div>
      <div class={whyListClass}>
        {#each WHY as item (item.title)}
          <div>
            <h3 class={whyTitleClass}>{item.title}</h3>
            <p class={whyBodyClass}>{item.body}</p>
          </div>
        {/each}
      </div>
    </div>
  </section>

  <section class={closingClass}>
    <h2 class={closingTitleClass}>{COPY.closingTitle}</h2>
    <p class={closingSubClass}>
      {COPY.closingSub[0]}
      <br />
      {COPY.closingSub[1]}
    </p>
    <div class={css({ marginTop: '36px' })}><StartButton badge="2주 무료" /></div>
  </section>
</Page>
