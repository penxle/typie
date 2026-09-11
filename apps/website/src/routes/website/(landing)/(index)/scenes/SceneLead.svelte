<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { CARD_STEP, HIDDEN_BELOW, ramp, rise } from '../scrub';
  import type { Card, Lead } from '../features';

  type Props = { lead: Lead; shown: number; cards?: readonly Card[]; cardsProgress?: number; split?: boolean };

  let { lead, shown, cards, cardsProgress = 0, split = false }: Props = $props();

  // 카드 한 걸음 50dvh 중 전환 20dvh, 머무름 30dvh
  const FOLD = CARD_STEP * 0.4;
  const EDGE = 40;
  const BLUR = 6;

  // 레일 높이는 접힘과 무관하게 고정한다 — 닫힌 상태 높이에 가장 긴 본문 하나를 더한 만큼을 늘 확보한다.
  // 한 번에 한 항목만 열리고, 전환 중에는 두 항목의 열림 합이 1이므로 이보다 높아지지 않는다.
  let closedHeight = $state(0);
  let bodyHeights = $state<number[]>([]);
  const railHeight = $derived(closedHeight + Math.max(0, ...bodyHeights));

  const reachedAt = (index: number) => ramp(cardsProgress, index * CARD_STEP, index * CARD_STEP + FOLD);
  const openAt = (index: number) => {
    const reached = reachedAt(index);
    if (!cards || index === cards.length - 1) return reached;
    return reached - ramp(cardsProgress, (index + 1) * CARD_STEP, (index + 1) * CARD_STEP + FOLD);
  };

  const hostClass = css({ willChange: 'opacity, transform' });
  const splitClass = css({
    display: 'grid',
    gridTemplateColumns: { base: '[minmax(0, 1fr)]', lg: '[minmax(0, 1fr) minmax(0, 1fr)]' },
    columnGap: '64px',
    alignItems: 'start',
  });
  const headlineClass = css({
    fontFamily: 'landing',
    fontSize: { base: '22px', md: '26px', xl: '[32px]' },
    fontWeight: 'bold',
    lineHeight: '[1.35]',
    letterSpacing: '[-0.02em]',
    color: 'text.default',
    wordBreak: 'keep-all',
  });
  const bodyClass = css({
    marginTop: '20px',
    fontFamily: 'landing',
    fontSize: { base: '14px', xl: '16px' },
    lineHeight: '[1.75]',
    color: 'text.muted',
    wordBreak: 'keep-all',
  });

  const railHostClass = css({ position: 'relative', marginTop: '32px', '&[data-split="true"]': { marginTop: '0' } });
  const railClass = css({ display: 'grid', alignContent: 'start', rowGap: '6px' });
  const ghostClass = css({ position: 'absolute', left: '0', right: '0', top: '0', visibility: 'hidden', pointerEvents: 'none' });
  const itemClass = css({ position: 'relative', paddingX: '16px', paddingY: '12px' });
  const panelClass = css({
    position: 'absolute',
    inset: '0',
    borderRadius: '12px',
    backgroundColor: 'surface.inset',
    border: '1px solid',
    borderColor: 'border.hairline',
  });
  const contentClass = css({ position: 'relative' });
  const headClass = css({ display: 'grid', gridTemplateColumns: '[26px minmax(0, 1fr)]', alignItems: 'center', willChange: 'opacity' });
  const iconClass = css({ size: '16px', color: 'text.hint' });
  const titleClass = css({ fontFamily: 'landing', fontSize: '15px', fontWeight: 'bold', color: 'text.default' });
  const foldClass = css({ display: 'grid', overflow: 'hidden' });
  // 접히는 가장자리를 직선으로 자르지 않는다 — 다 열리면 페이드 폭이 0이 되어 마지막 줄은 온전하다
  const foldInnerClass = css({
    minHeight: '0',
    overflow: 'hidden',
    maskImage: '[linear-gradient(to bottom, #000 calc(100% - var(--edge)), transparent 100%)]',
    WebkitMaskImage: '[linear-gradient(to bottom, #000 calc(100% - var(--edge)), transparent 100%)]',
  });
  const cardBodyClass = css({
    paddingTop: '8px',
    paddingLeft: '26px',
    fontFamily: 'landing',
    fontSize: '16px',
    lineHeight: '[1.6]',
    color: 'text.default',
    willChange: 'filter, opacity',
  });
</script>

{#snippet prose()}
  <div>
    <h2 class={headlineClass}>
      {lead.headline[0]}
      <br />
      {lead.headline[1]}
    </h2>
    <p class={bodyClass}>{lead.body.join(' ')}</p>
  </div>
{/snippet}

{#snippet rail()}
  {#if cards}
    <div style:min-height="{railHeight}px" class={railHostClass} data-split={split}>
      <div class="{railClass} {ghostClass}" aria-hidden="true" bind:clientHeight={closedHeight}>
        {#each cards as card, index (card.title)}
          {@const Icon = card.icon}
          <div class={itemClass}>
            <div class={headClass}>
              <Icon class={iconClass} />
              <span class={titleClass}>{card.title}</span>
            </div>
            <p class="{cardBodyClass} {ghostClass}" bind:clientHeight={bodyHeights[index]}>{card.body}</p>
          </div>
        {/each}
      </div>

      <div class={railClass}>
        {#each cards as card, index (card.title)}
          {@const reached = reachedAt(index)}
          {@const open = openAt(index)}
          {@const text = ramp(open, 0.3, 1)}
          {@const Icon = card.icon}
          <div style:transform={rise(reached, 10)} class={itemClass}>
            <span style:opacity={ramp(open, 0, 0.5)} class={panelClass}></span>
            <div class={contentClass}>
              <div style:opacity={0.45 + 0.55 * Math.max(reached, open)} class={headClass}>
                <Icon class={iconClass} />
                <span class={titleClass}>{card.title}</span>
              </div>

              <div style:grid-template-rows="{open}fr" class={foldClass}>
                <div style:--edge="{(1 - open) * EDGE}px" class={foldInnerClass}>
                  <p
                    style:opacity={text}
                    style:filter="blur({(1 - text) * BLUR}px)"
                    class={cardBodyClass}
                    aria-hidden={open < HIDDEN_BELOW}
                  >
                    {card.body}
                  </p>
                </div>
              </div>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}
{/snippet}

<div style:opacity={shown} style:transform={rise(shown, 12)} class={hostClass} aria-hidden={shown < HIDDEN_BELOW}>
  {#if split}
    <div class={splitClass}>
      {@render prose()}
      {@render rail()}
    </div>
  {:else}
    {@render prose()}
    {@render rail()}
  {/if}
</div>
