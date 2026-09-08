<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import ActionRow from '$lib/components/ActionRow.svelte';
  import Footer from '$lib/components/Footer.svelte';
  import { ramp, rise } from '../scrub';
  import {
    ACTION_RANGE,
    ASSURANCE_RANGE,
    CLOSING_HEADLINE,
    CLOSING_LINE_OFFSETS,
    FOOTER_RANGE,
    headlineRamp,
    SUMMARY_RANGE,
  } from './closing';

  type Props = { progress: number };

  let { progress }: Props = $props();

  const lines = CLOSING_HEADLINE;
  const SUMMARY = ['2주 동안 전부 무료로 써보세요. 끝나도 자동으로 결제되지 않고,', '계속 쓰고 싶을 때 월 2,900원으로 구독하면 돼요.'];
  const reveal = (index: number) => ramp(progress, ...headlineRamp(index));

  const summary = $derived(ramp(progress, ...SUMMARY_RANGE));
  const action = $derived(ramp(progress, ...ACTION_RANGE));
  const assurance = $derived(ramp(progress, ...ASSURANCE_RANGE));
  const footer = $derived(ramp(progress, ...FOOTER_RANGE));

  const sectionClass = css({ position: 'relative', height: { base: '[auto]', lg: '[300dvh]' } });
  const stageClass = css({
    position: { base: 'static', lg: 'sticky' },
    top: '0',
    display: 'grid',
    gridTemplateRows: '1fr auto',
    height: { base: '[auto]', lg: '[100dvh]' },
    minHeight: { base: '[100dvh]', lg: '[auto]' },
  });
  const bodyClass = css({
    display: 'grid',
    alignContent: '[safe center]',
    justifyItems: 'center',
    textAlign: 'center',
    paddingX: { base: '20px', lg: '64px' },
    paddingTop: '96px',
    paddingBottom: '48px',
  });
  const headlineClass = css({
    marginTop: '[-0.131em]',
    marginBottom: '[-0.1275em]',
    fontSize: { base: '[36px]', md: '[56px]', xl: '[72px]' },
    fontWeight: 'bold',
    lineHeight: '[1.15]',
    letterSpacing: '[-0.025em]',
  });
  const wordClass = css({ display: 'inline-block', marginRight: '[0.22em]', willChange: 'opacity, transform' });
  const actionClass = css({ display: 'flex', justifyContent: 'center', marginTop: { base: '40px', md: '48px', xl: '64px' } });
  const summaryClass = css({
    marginTop: '24px',
    fontFamily: 'ui',
    fontSize: { base: '15px', md: '16px', xl: '[18px]' },
    lineHeight: '[1.6]',
    color: 'text.muted',
  });
  const softBreakClass = css({ display: { base: 'none', md: 'inline' } });
  const assuranceClass = css({ marginTop: '16px', fontFamily: 'ui', fontSize: '13px', color: 'text.hint' });
</script>

<section class={sectionClass}>
  <div class={stageClass}>
    <div class={bodyClass}>
      <h2 class={headlineClass}>
        {#each lines as line, lineIndex (lineIndex)}
          {#if lineIndex > 0}<br />{/if}
          {#each line as word, wordIndex (wordIndex)}
            {@const shown = reveal(CLOSING_LINE_OFFSETS[lineIndex] + wordIndex)}
            <span style:opacity={shown} style:transform={rise(shown, 0.4, 'em')} class={wordClass}>{word}</span>
          {/each}
        {/each}
      </h2>
      <p style:opacity={summary} style:transform={rise(summary, 8)} class={summaryClass}>
        {#each SUMMARY as line, index (index)}
          {#if index > 0}<br class={softBreakClass} />
            &nbsp;{/if}{line}
        {/each}
      </p>
      <div style:opacity={action} style:transform={rise(action, 12)} class={actionClass}>
        <ActionRow badge="2주 무료" marginTop="0px" platforms={false} />
      </div>
      <p style:opacity={assurance} style:transform={rise(assurance, 8)} class={assuranceClass}>카드 등록 없이 시작할 수 있어요</p>
    </div>
    <div style:opacity={footer} style:transform={rise(footer, 24)}><Footer /></div>
  </div>
</section>
