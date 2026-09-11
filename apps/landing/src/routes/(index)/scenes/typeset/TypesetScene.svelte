<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { ramp, SCENE_LEAD_RANGE, SCENE_SLIDE_RANGE } from '../../scrub';
  import SceneLead from '../SceneLead.svelte';
  import { PAGE, PAGE_TEXT_SIZE, pagedSequence, settingLineOf, SWEEP_RANGE, sweepAt, TYPESET_SEQUENCE, zoomFor } from './typeset';
  import type { SceneProps } from '../../scrub';
  import type { TypesetState } from './typeset';

  let { progress, cardsProgress = 0, lead, cards }: SceneProps = $props();

  const LINE_WIDTH = 2;
  // 재단선은 여백 안쪽 네 모서리에 L자로 — apps/website/src/lib/editor-ffi/components/Page.svelte
  const CROP_SIZE = 32;
  const CROP_PATH = (() => {
    const { width, height, margin } = PAGE;
    const [left, right, top, bottom] = [margin, width - margin, margin, height - margin];
    return [
      `M ${left} ${top - CROP_SIZE} L ${left} ${top} L ${left - CROP_SIZE} ${top}`,
      `M ${right} ${top - CROP_SIZE} L ${right} ${top} L ${right + CROP_SIZE} ${top}`,
      `M ${left} ${bottom + CROP_SIZE} L ${left} ${bottom} L ${left - CROP_SIZE} ${bottom}`,
      `M ${right} ${bottom + CROP_SIZE} L ${right} ${bottom} L ${right + CROP_SIZE} ${bottom}`,
    ].join(' ');
  })();
  const PARAGRAPHS = [
    '돌고 돌던 순환로를 벗어나 나는 첫 발을 떼었어. 그토록 멀고도 긴 한 발자국이었지. 지구가 얼마나 아름다웠는지를 추억하기에 우리는 이미 너무 멀리 온 것 같아. 하지만 아직 너무 늦지는 않았기를 믿고 있어.',
    '그러니까 부디 그때까지 기다려줬으면 해. 우리에게 남은 시간은 아주 많으니까.',
    '이 우주에게는 찰나의 순간이더라도, 우리에게는 아주 긴 시간일 테니까.',
  ];

  const LEAD_WIDTH = 557;
  const LEAD_GAP = 56;
  const SIDE_PADDING = 40;

  let hostWidth = $state(0);
  let hostHeight = $state(0);

  const zoom = $derived(zoomFor(hostWidth - SIDE_PADDING, hostHeight));
  const slot = $derived({ width: Math.round(PAGE.width * zoom), height: Math.round(PAGE.height * zoom) });
  const pageFontSize = $derived(Math.round(PAGE_TEXT_SIZE / zoom));
  let columnHeight = $state(0);

  const leadTop = $derived(Math.max(0, (hostHeight - columnHeight) / 2));

  const appear = $derived(0.2 + ramp(progress, 0, 0.1) * 0.8);
  const sequence = $derived(pagedSequence(hostWidth));
  const sweep = $derived(sweepAt(ramp(progress, ...SWEEP_RANGE), sequence));
  const slide = $derived(ramp(progress, ...SCENE_SLIDE_RANGE));
  const leadIn = $derived(ramp(progress, ...SCENE_LEAD_RANGE));
  const edge = $derived(Math.min(1, sweep.x * 40, (1 - sweep.x) * 40));
  const boundary = $derived(sweep.x * 100);

  const rootClass = css({
    display: 'grid',
    gridTemplateRows: '[minmax(0, 1fr)]',
    width: 'full',
    height: 'full',
  });
  const leadHostClass = css({
    display: { base: 'none', lg: 'block' },
    gridColumn: { lg: '1' },
    gridRow: { lg: '1' },
    alignSelf: 'start',
    paddingTop: '[var(--lead-top)]',
    width: 'full',
    marginX: 'auto',
    position: 'relative',
    zIndex: '1',
    pointerEvents: 'none',
  });
  const hostClass = css({
    gridColumn: { lg: '1' },
    gridRow: { lg: '1' },
    position: 'relative',
    height: 'full',
    marginX: { base: '-20px', lg: '-64px' },
  });
  const layerClass = css({ position: 'absolute', inset: '0', display: 'grid', alignContent: 'center', pointerEvents: 'none' });
  const blockClass = css({
    marginX: 'auto',
    width: 'full',
    paddingX: { base: '20px', lg: '0' },
  });
  const columnClass = css({
    color: 'text.default',
    fontSize: { base: '18px', md: '[clamp(18px, 2.6vh, 24px)]' },
    lineHeight: '[1.85]',
    letterSpacing: '[-0.01em]',
    wordBreak: 'break-all',
    '&[data-font="pretendard"]': { fontFamily: 'Pretendard' },
    '&[data-font="batang"]': { fontFamily: 'RIDIBatang' },
    '&[data-paged="true"]': {
      position: 'relative',
      backgroundColor: 'surface.default',
      boxShadow: 'md',
      ringWidth: '1px',
      ringColor: 'border.hairline',
    },
  });
  const slotClass = css({ position: 'relative', flexShrink: '0' });
  const innerClass = css({
    marginX: 'auto',
    '--shift': { base: '0px', lg: '[calc(var(--slide-progress) * var(--slide-offset))]' },
    transform: '[translateX(var(--shift))]',
    willChange: 'transform',
  });
  const cropClass = css({
    position: 'absolute',
    inset: '0',
    width: 'full',
    height: 'full',
    overflow: 'visible',
    pointerEvents: 'none',
    color: 'text.default',
    opacity: '15',
  });
  const settingsClass = css({
    marginBottom: '14px',
    fontFamily: 'ui',
    fontSize: '12px',
    lineHeight: '[1.4]',
    textAlign: 'right',
    color: 'text.hint',
    whiteSpace: 'nowrap',
  });
  const lineClass = css({
    position: 'absolute',
    top: '[50%]',
    height: '[120dvh]',
    transform: '[translateY(-50%)]',
    backgroundColor: 'text.default',
    pointerEvents: 'none',
  });
</script>

{#snippet paragraphs(state: TypesetState)}
  {#each PARAGRAPHS as paragraph, index (index)}
    <p style:margin-top={index === 0 ? '0' : `${state.gap}em`} style:text-indent="{state.indent}em">{paragraph}</p>
  {/each}
{/snippet}

{#snippet column(state: TypesetState)}
  <div style:max-width="{state.maxWidth}px" class={blockClass} bind:clientHeight={columnHeight}>
    <div style:width={state.paged ? `${slot.width}px` : undefined} class={innerClass}>
      <div class={settingsClass}>{settingLineOf(state)}</div>

      {#if state.paged}
        <div style:width="{slot.width}px" style:height="{slot.height}px" class={slotClass}>
          <div
            style:width="{PAGE.width}px"
            style:height="{PAGE.height}px"
            style:padding="{PAGE.margin}px"
            style:font-size="{pageFontSize}px"
            style:transform="scale({zoom})"
            style:transform-origin="top left"
            class={columnClass}
            data-font={state.font}
            data-paged="true"
          >
            <svg class={cropClass} xmlns="http://www.w3.org/2000/svg">
              <path d={CROP_PATH} fill="none" stroke="currentColor" />
            </svg>

            {@render paragraphs(state)}
          </div>
        </div>
      {:else}
        <div class={columnClass} data-font={state.font} data-paged="false">
          {@render paragraphs(state)}
        </div>
      {/if}
    </div>
  </div>
{/snippet}

<div style:--slide-offset="{(LEAD_WIDTH + LEAD_GAP) / 2}px" style:--slide-progress={slide} class={rootClass}>
  <div style:--lead-top="{leadTop}px" style:max-width="{LEAD_WIDTH + LEAD_GAP + TYPESET_SEQUENCE[0].maxWidth}px" class={leadHostClass}>
    <div style:max-width="{LEAD_WIDTH}px">
      <SceneLead {cards} {cardsProgress} {lead} shown={leadIn} />
    </div>
  </div>

  <div class={hostClass} bind:clientWidth={hostWidth} bind:clientHeight={hostHeight}>
    <div style:clip-path="inset(-50vh 0 -50vh {boundary}%)" style:opacity={appear} class={layerClass}>{@render column(sweep.prev)}</div>
    <div style:clip-path="inset(-50vh {100 - boundary}% -50vh 0)" style:opacity={appear} class={layerClass}>
      {@render column(sweep.next)}
    </div>

    <div
      style:left="clamp(0px, calc({boundary}% - {LINE_WIDTH / 2}px), calc(100% - {LINE_WIDTH}px))"
      style:opacity={edge * appear}
      style:width="{LINE_WIDTH}px"
      class={lineClass}
    ></div>
  </div>
</div>
