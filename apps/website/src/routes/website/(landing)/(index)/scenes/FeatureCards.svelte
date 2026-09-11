<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { CARD_SPAN, CARD_STEP, HIDDEN_BELOW, ramp, rise } from '../scrub';
  import type { Card } from '../features';

  type Props = { cards: readonly Card[]; progress: number };

  let { cards, progress }: Props = $props();

  const shownAt = (index: number) => ramp(progress, index * CARD_STEP, index * CARD_STEP + CARD_SPAN);

  const gridClass = css({
    display: 'grid',
    gridTemplateColumns: { base: '[minmax(0, 1fr)]', md: '[repeat(2, minmax(0, 1fr))]', xl: '[repeat(4, minmax(0, 1fr))]' },
    columnGap: '32px',
    rowGap: '28px',
    width: 'full',
    maxWidth: '[1400px]',
    marginX: 'auto',
  });
  const cardClass = css({ willChange: 'opacity, transform' });
  const titleClass = css({ fontFamily: 'landing', fontSize: '15px', fontWeight: 'bold', color: 'text.default' });
  const bodyClass = css({
    marginTop: '6px',
    fontFamily: 'landing',
    fontSize: { base: '13px', xl: '15px' },
    lineHeight: '[1.6]',
    color: 'text.muted',
  });
</script>

<div class={gridClass}>
  {#each cards as card, index (card.title)}
    {@const shown = shownAt(index)}
    <div style:opacity={shown} style:transform={rise(shown, 12)} class={cardClass} aria-hidden={shown < HIDDEN_BELOW}>
      <div class={titleClass}>{card.title}</div>
      <p class={bodyClass}>{card.body}</p>
    </div>
  {/each}
</div>
