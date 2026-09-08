<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Helmet } from '@typie/ui/components';
  import Page from '$lib/landing/components/Page.svelte';
  import StartButton from '$lib/landing/components/StartButton.svelte';
  import Faq from './Faq.svelte';
  import IntervalToggle from './IntervalToggle.svelte';
  import PlanFeatures from './PlanFeatures.svelte';
  import Price from './Price.svelte';
  import { COPY, PLAN, priceFor } from './pricing';
  import Trusts from './Trusts.svelte';
  import type { Interval } from './pricing';

  let interval = $state<Interval>('monthly');

  const price = $derived(priceFor(interval));

  const cardClass = css({
    width: 'full',
    maxWidth: '[440px]',
    marginX: 'auto',
    marginTop: { base: '40px', md: '56px' },
    padding: { base: '28px', md: '36px' },
    borderRadius: '[20px]',
    borderWidth: '1px',
    borderColor: 'border.hairline',
    backgroundColor: 'surface.canvas',
    boxShadow: '[inset 0 1px 0 color-mix(in srgb, token(colors.text.default) 6%, transparent)]',
  });
  const planClass = css({
    fontFamily: 'mono',
    fontSize: '[12px]',
    fontWeight: 'medium',
    letterSpacing: '[0.1em]',
    textTransform: 'uppercase',
    color: 'text.muted',
  });
  const priceRowClass = css({ display: 'flex', alignItems: 'baseline', marginTop: '20px' });
  const noteClass = css({ marginTop: '10px', minHeight: '[20px]', fontSize: '14px', color: 'text.muted' });
  const taglineClass = css({ marginTop: '20px', fontSize: '15px', lineHeight: '[1.6]', color: 'text.default' });
  const ctaClass = css({
    display: 'grid',
    marginTop: '24px',
    '& a': { justifyContent: 'center', width: 'full' },
    '& > span': { display: 'block' },
  });
  const assuranceClass = css({ marginTop: '12px', textAlign: 'center', fontSize: '13px', color: 'text.hint' });
  const includesClass = css({ marginTop: '28px', paddingTop: '24px', borderTopWidth: '1px', borderTopColor: 'border.hairline' });
  const includesLabelClass = css({
    marginBottom: '14px',
    fontFamily: 'mono',
    fontSize: '[11px]',
    letterSpacing: '[0.1em]',
    textTransform: 'uppercase',
    color: 'text.muted',
  });
  const trustsClass = css({ maxWidth: '[960px]', marginX: 'auto', paddingTop: { base: '80px', lg: '112px' } });
  const faqClass = css({ maxWidth: '[760px]', marginX: 'auto', paddingTop: { base: '96px', lg: '128px' } });
  const faqTitleClass = css({
    fontSize: { base: '[26px]', md: '[32px]' },
    fontWeight: 'bold',
    letterSpacing: '[-0.02em]',
    lineHeight: '[1.3]',
  });
  const closingClass = css({ display: 'grid', justifyItems: 'center', paddingTop: { base: '112px', lg: '160px' }, textAlign: 'center' });
  const closingTitleClass = css({
    fontSize: { base: '[36px]', md: '[56px]' },
    fontWeight: 'bold',
    letterSpacing: '[-0.025em]',
    lineHeight: '[1.15]',
  });
  const closingSubClass = css({ marginTop: '16px', fontSize: { base: '15px', md: '17px' }, color: 'text.muted' });
</script>

<Helmet description={COPY.description} title={COPY.pageTitle} />

<Page sub={COPY.sub}>
  {#snippet title()}
    {COPY.title[0]}
    <br />
    <span class={css({ color: 'text.muted' })}>{COPY.title[1]}</span>
  {/snippet}
  {#snippet hero()}
    <div class={css({ marginTop: '36px' })}><IntervalToggle bind:interval /></div>
  {/snippet}

  <div class={cardClass}>
    <span class={planClass}>{PLAN.name}</span>
    <div class={priceRowClass}><Price unit={COPY.perMonth} value={price} /></div>
    <p class={noteClass}>{interval === 'yearly' ? COPY.yearlyNote : COPY.monthlyNote}</p>
    <p class={taglineClass}>{COPY.tagline}</p>
    <div class={ctaClass}><StartButton badge="2주 무료" /></div>
    <p class={assuranceClass}>카드 등록 없이 시작할 수 있어요</p>
    <div class={includesClass}>
      <p class={includesLabelClass}>{COPY.includes}</p>
      <PlanFeatures />
    </div>
  </div>

  <div class={trustsClass}><Trusts /></div>

  <section class={faqClass}>
    <h2 class={faqTitleClass}>{COPY.faqTitle}</h2>
    <div class={css({ marginTop: '24px' })}><Faq /></div>
  </section>

  <section class={closingClass}>
    <h2 class={closingTitleClass}>{COPY.closingTitle}</h2>
    <p class={closingSubClass}>{COPY.closingSub}</p>
    <div class={css({ marginTop: '36px' })}><StartButton badge="2주 무료" /></div>
    <p class={assuranceClass}>카드 등록 없이 시작할 수 있어요</p>
  </section>
</Page>
