<script lang="ts">
  import { css, cva, cx } from '@typie/styled-system/css';
  import { getMarginContext } from './context.svelte.ts';
  import { GUTTER } from './margin-view.ts';
  import { layoutRailHitTargets, RAIL_CHIP_SIZE, RAIL_TEXT_GAP, RAIL_WIDTH } from './rail-layout.ts';
  import type { RailSpan } from './rail-layout.ts';

  // 거터는 rail이 사용할 수 있는 전체 폭이다. gap은 lane과 번호 칩의 상대 위치를 정할 때 오른쪽에 미리 남겨 두는 공간이다.
  // layoutRails는 그 상대 위치를 유지한 채, 세로로 겹치는 묶음 전체를 본문에서 최소 간격만큼 떨어진 자리로 옮긴다.
  type Props = { spans: RailSpan[]; gutter?: number; gap?: number };
  let { spans, gutter = GUTTER, gap = RAIL_TEXT_GAP }: Props = $props();

  const margin = getMarginContext();
  const rails = $derived(layoutRailHitTargets(spans, gutter, gap));

  const barRecipe = cva({
    base: {
      position: 'absolute',
      borderRadius: 'full',
      pointerEvents: 'auto',
      cursor: 'pointer',
      transition: '[background-color 0.2s cubic-bezier(0.2, 0, 0, 1), opacity 0.2s cubic-bezier(0.2, 0, 0, 1)]',
    },
    variants: {
      tone: {
        open: { backgroundColor: '[color-mix(in oklch, token(colors.review.issue) 16%, transparent)]' },
        closed: { backgroundColor: 'surface.inset' },
        strength: { backgroundColor: '[color-mix(in oklch, token(colors.review.strength) 16%, transparent)]' },
      },
      active: { true: {}, false: {} },
    },
    compoundVariants: [
      { tone: 'open', active: false, css: { _groupHover: { backgroundColor: 'review.issue', opacity: '80' } } },
      { tone: 'closed', active: false, css: { _groupHover: { backgroundColor: 'border.emphasis', opacity: '80' } } },
      { tone: 'strength', active: false, css: { _groupHover: { backgroundColor: 'review.strength', opacity: '80' } } },
      { tone: 'open', active: true, css: { backgroundColor: 'review.issue', opacity: '100' } },
      { tone: 'closed', active: true, css: { backgroundColor: 'border.emphasis', opacity: '100' } },
      { tone: 'strength', active: true, css: { backgroundColor: 'review.strength', opacity: '100' } },
    ],
  });

  const chipRecipe = cva({
    base: {
      position: 'absolute',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: '5px',
      borderWidth: '1px',
      borderStyle: 'solid',
      borderColor: 'transparent',
      fontSize: '10px',
      fontWeight: 'bold',
      pointerEvents: 'auto',
      cursor: 'pointer',
      transition:
        '[background-color 0.2s cubic-bezier(0.2, 0, 0, 1), border-color 0.2s cubic-bezier(0.2, 0, 0, 1), color 0.2s cubic-bezier(0.2, 0, 0, 1)]',
    },
    variants: {
      tone: {
        open: { backgroundColor: '[color-mix(in oklch, token(colors.review.issue) 16%, transparent)]', color: 'review.issue' },
        closed: { backgroundColor: 'surface.inset', color: 'text.hint' },
        strength: {
          backgroundColor: '[color-mix(in oklch, token(colors.review.strength) 16%, transparent)]',
          color: 'review.strength',
        },
      },
      active: { true: {}, false: {} },
    },
    compoundVariants: [
      { tone: 'open', active: false, css: { _groupHover: { borderColor: 'review.issue' } } },
      { tone: 'closed', active: false, css: { _groupHover: { borderColor: 'border.emphasis' } } },
      { tone: 'strength', active: false, css: { _groupHover: { borderColor: 'review.strength' } } },
      {
        tone: 'open',
        active: true,
        css: { backgroundColor: 'review.issue', borderColor: 'review.issue', color: 'surface.default' },
      },
      {
        tone: 'closed',
        active: true,
        css: { backgroundColor: 'surface.inverse', borderColor: 'surface.inverse', color: 'text.on.inverse' },
      },
      {
        tone: 'strength',
        active: true,
        css: { backgroundColor: 'review.strength', borderColor: 'review.strength', color: 'surface.default' },
      },
    ],
  });

  const labelOf = (rail: (typeof rails)[number]) => {
    const item = margin.items.find((candidate) => candidate.id === rail.itemId);
    if (item?.kind === 'strength') return `잘 닿은 대목 ${rail.number}`;
    return `피드백 ${rail.number}: ${item?.thread?.trait ?? '피드백'}`;
  };
</script>

<!-- 숫자와 막대의 최소 사각형이 한 버튼이다. 시각 요소는 모든 투명 클릭 면보다 위에 서서 직접 클릭을 먼저 받는다. -->
{#each rails as rail (rail.id)}
  {@const active = margin.activeId === rail.itemId}
  {@const visualPriority = rails.length + rail.hitPriority + 1}
  <button
    style:top={`${rail.hitBox.top}px`}
    style:height={`${rail.hitBox.bottom - rail.hitBox.top}px`}
    style:left={`${rail.hitBox.left}px`}
    style:width={`${rail.hitBox.right - rail.hitBox.left}px`}
    class={cx(
      'group',
      css({
        position: 'absolute',
        padding: '0',
        borderWidth: '0',
        backgroundColor: 'transparent',
        pointerEvents: 'none',
        cursor: 'pointer',
      }),
    )}
    aria-label={labelOf(rail)}
    data-selected={active ? '' : undefined}
    onclick={() => margin.activate(rail.itemId)}
    type="button"
  >
    <span
      style:z-index={rail.hitPriority}
      class={css({ position: 'absolute', inset: '0', pointerEvents: 'auto', cursor: 'pointer' })}
      aria-hidden="true"
    ></span>
    <span
      style:top={`${rail.top - rail.hitBox.top}px`}
      style:height={`${rail.height}px`}
      style:left={`${rail.left - rail.hitBox.left}px`}
      style:width={`${RAIL_WIDTH}px`}
      style:z-index={visualPriority}
      class={css(barRecipe.raw({ tone: rail.tone, active }))}
      aria-hidden="true"
    ></span>
    <span
      style:top={`${rail.chipTop - rail.hitBox.top}px`}
      style:left={`${rail.chipLeft - rail.hitBox.left}px`}
      style:width={`${RAIL_CHIP_SIZE}px`}
      style:height={`${RAIL_CHIP_SIZE}px`}
      style:z-index={visualPriority}
      class={css(chipRecipe.raw({ tone: rail.tone, active }))}
      aria-hidden="true"
    >
      {rail.number}
    </span>
  </button>
{/each}
