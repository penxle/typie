<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { getMarginContext } from './context.svelte.ts';
  import { GUTTER } from './margin-view.ts';
  import { layoutRails, RAIL_CHIP_SIZE, RAIL_TEXT_GAP, RAIL_WIDTH, railLeft } from './rail-layout.ts';
  import type { RailSpan } from './rail-layout.ts';

  // 거터는 실제로 비어 있는 폭, gap은 본문과의 이격이다 — 둘 다 좁아진 만큼만 받는다.
  // 공간이 모자랄 때 양보하는 것은 이격이지 막대가 아니다
  type Props = { spans: RailSpan[]; gutter?: number; gap?: number };
  let { spans, gutter = GUTTER, gap = RAIL_TEXT_GAP }: Props = $props();

  const margin = getMarginContext();
  const rails = $derived(layoutRails(spans, gutter, gap));

  const barRecipe = cva({
    base: {
      position: 'absolute',
      borderRadius: 'full',
      cursor: 'pointer',
      transition: '[background-color 0.25s cubic-bezier(0.2, 0, 0, 1), left 0.25s cubic-bezier(0.2, 0, 0, 1)]',
    },
    variants: {
      tone: {
        open: { backgroundColor: 'review.issue.subtle' },
        closed: { backgroundColor: 'surface.muted' },
        strength: { backgroundColor: 'review.strength.subtle' },
      },
      active: { true: {}, false: {} },
    },
    // 막대·레일 칩·카드 칩은 같은 지적의 세 표지다 — 상태마다 셋이 정확히 같은 색이어야 한다
    compoundVariants: [
      { tone: 'open', active: true, css: { backgroundColor: 'review.issue.default' } },
      { tone: 'closed', active: true, css: { backgroundColor: 'border.strong' } },
      { tone: 'strength', active: true, css: { backgroundColor: 'review.strength.default' } },
    ],
  });

  const chipRecipe = cva({
    base: {
      position: 'absolute',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: '5px',
      fontSize: '10px',
      fontWeight: 'bold',
      cursor: 'pointer',
      transition: '[background-color 0.25s cubic-bezier(0.2, 0, 0, 1), color 0.25s cubic-bezier(0.2, 0, 0, 1)]',
    },
    variants: {
      tone: {
        open: { backgroundColor: 'review.issue.subtle', color: 'review.issue.default' },
        closed: { backgroundColor: 'surface.muted', color: 'text.faint' },
        strength: {
          backgroundColor: 'review.strength.subtle',
          color: 'review.strength.default',
        },
      },
      active: { true: {}, false: {} },
    },
    // 활성 반전도 계열을 따른다 — 표기 전체가 회색조인 닫힌 지적에서 칩만 브랜드로 뒤집히면 어긋난다
    compoundVariants: [
      { tone: 'open', active: true, css: { backgroundColor: 'review.issue.default', color: 'surface.default' } },
      { tone: 'closed', active: true, css: { backgroundColor: 'border.strong', color: 'text.bright' } },
      { tone: 'strength', active: true, css: { backgroundColor: 'review.strength.default', color: 'surface.default' } },
    ],
  });

  const labelOf = (rail: (typeof rails)[number]) => {
    const item = margin.items.find((candidate) => candidate.id === rail.itemId);
    if (item?.kind === 'strength') return `잘 닿은 대목 ${rail.number}`;
    return `피드백 ${rail.number}: ${item?.thread?.trait ?? '피드백'}`;
  };
</script>

<!-- 레일은 켜기 전용이다 — 재클릭은 해제가 아니라 그 자리로 다시 데려간다. -->
{#each rails as rail (rail.id)}
  {@const active = margin.activeId === rail.itemId}
  <button
    style:top={`${rail.top}px`}
    style:height={`${rail.height}px`}
    style:left={`${railLeft(rail.lane, gutter, gap)}px`}
    style:width={`${RAIL_WIDTH}px`}
    class={css(barRecipe.raw({ tone: rail.tone, active }))}
    aria-label={labelOf(rail)}
    data-prism-margin-activator
    onclick={() => margin.activate(rail.itemId)}
    type="button"
  ></button>
  <button
    style:top={`${rail.chipTop}px`}
    style:left={`${rail.chipLeft}px`}
    style:width={`${RAIL_CHIP_SIZE}px`}
    style:height={`${RAIL_CHIP_SIZE}px`}
    class={css(chipRecipe.raw({ tone: rail.tone, active }))}
    aria-label={labelOf(rail)}
    data-prism-margin-activator
    onclick={() => margin.activate(rail.itemId)}
    type="button"
  >
    {rail.number}
  </button>
{/each}
