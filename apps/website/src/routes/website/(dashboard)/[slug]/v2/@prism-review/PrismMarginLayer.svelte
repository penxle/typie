<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Button, SegmentButtons } from '@typie/ui/components';
  import { untrack } from 'svelte';
  import { CONTINUOUS_VIEW_PADDING } from '$lib/editor-ffi/constants';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import PrismReviewDetail from '../../../@prism/review/PrismReviewDetail.svelte';
  import { getMarginContext } from './context.svelte.ts';
  import { COLUMN_GAP, COLUMN_WIDTH, GUTTER } from './margin-view.ts';
  import PrismCardColumn from './PrismCardColumn.svelte';
  import PrismCardPopover from './PrismCardPopover.svelte';
  import PrismOverviewRuler from './PrismOverviewRuler.svelte';
  import PrismRail from './PrismRail.svelte';
  import { RAIL_TEXT_GAP, RAIL_WIDTH } from './rail-layout.ts';
  import type { MarginItem } from './context.svelte.ts';
  import type { RailSpan, RailTone } from './rail-layout.ts';

  type Mark = { itemId: string; ratio: number; tone: RailTone };

  // 컨트롤러가 실제로 낸 오른쪽 자리. 회차를 갈아타는 동안에도 래치되어 유지되므로
  // 컬럼의 렌더 조건을 여기서 읽으면 자리가 없는 프레임에 그리지도, 전환 중에 깜빡이지도 않는다
  type Props = { insetRight: number };
  let { insetRight }: Props = $props();

  const ctx = getEditorContext();
  const margin = getMarginContext();

  let spans = $state.raw<RailSpan[]>([]);
  let desiredTops = $state.raw<Record<string, number>>({});
  let marks = $state.raw<Mark[]>([]);
  let bodyLeft = $state(0);
  let pageRight = $state(0);
  // 확장 영역 위에 얹힌 헤더(제목·부제목) 블록의 높이 — 세그먼트를 그 높이에 세운다
  let headerHeight = $state(0);

  const toneOf = (item: MarginItem): RailTone =>
    item.kind === 'strength' ? 'strength' : item.thread?.state === 'CLOSED' ? 'closed' : 'open';

  // items는 적용 스냅숏마다 새로 지어지고 published도 프레임 교체마다 새 번들이다 — 신원으로 재측정을 걸면
  // 타이핑 한 번마다 rect를 읽어 강제 리플로우가 난다. 측정은 판 번호와 항목 집합이 실제로 바뀔 때만 돈다.
  // anchored는 일부러 뺐다 — measure가 읽지 않으면서 적용 스냅숏마다 뒤집히는 값이다
  const publishedRevision = $derived(ctx.editor?.publishedRevision);
  const itemsKey = $derived(margin.items.map((item) => `${item.id}:${item.number}:${toneOf(item)}:${item.rangeIds.join(',')}`).join(' '));

  const measure = () => {
    const editor = ctx.editor;
    const area = editor?.extensionAreaEl;
    const snapshot = editor?.published?.snapshot;
    if (!editor || !area || !snapshot) return;

    const areaRect = area.getBoundingClientRect();
    const zoom = editor.safeDisplayZoom();

    // 타이핑 한 번에 한 번씩 도는 자리다 — 조회는 색인으로, 페이지 rect는 페이지당 한 번만 읽는다.
    // 스냅숏의 사본은 tracked_ranges 필드가 설 때만 갈려 리플로우를 못 따라간다 — 코어에서 지금 것을 받는다
    const ranges = new Map(editor.freshTrackedRanges().map((range) => [range.id, range]));
    const pageTops: (number | null | undefined)[] = [];
    const pageTopOf = (page: number): number | null => {
      const cached = pageTops[page];
      if (cached !== undefined) return cached;
      const el = editor.pageEls[page];
      const top = el ? el.getBoundingClientRect().top - areaRect.top : null;
      pageTops[page] = top;
      return top;
    };

    // 룰러 트랙은 스크롤 콘텐츠 전체를 대표한다 — 본문 영역은 그 안에서 헤더 높이만큼 내려앉아 있다
    const scroller = editor.scrollContainerEl;
    const contentHeight = scroller?.scrollHeight || area.scrollHeight;
    const contentOffset = scroller ? areaRect.top - scroller.getBoundingClientRect().top + scroller.scrollTop : 0;
    headerHeight = contentOffset;

    const nextSpans: RailSpan[] = [];
    const nextTops: Record<string, number> = {};
    const nextMarks: Mark[] = [];

    for (const item of margin.items) {
      const tone = toneOf(item);
      let first = Infinity;

      for (const rangeId of item.rangeIds) {
        const range = ranges.get(rangeId);
        if (!range || range.rects.length === 0) continue;

        let top = Infinity;
        let bottom = -Infinity;
        for (const { page_idx, rect } of range.rects) {
          const pageTop = pageTopOf(page_idx);
          if (pageTop === null) continue;
          top = Math.min(top, pageTop + rect.y * zoom);
          bottom = Math.max(bottom, pageTop + (rect.y + rect.height) * zoom);
        }
        if (!Number.isFinite(top)) continue;

        nextSpans.push({ id: rangeId, itemId: item.id, number: item.number, tone, top, height: bottom - top });
        first = Math.min(first, top);
      }

      // 음수 desired는 활성 카드를 컬럼 위로 탈출시킨다 — 클램프는 측정 지점의 몫이다
      if (item.kind === 'issue') nextTops[item.id] = Number.isFinite(first) ? Math.max(0, first) : first;
      if (Number.isFinite(first) && contentHeight > 0)
        nextMarks.push({ itemId: item.id, ratio: (contentOffset + first) / contentHeight, tone });
    }

    spans = nextSpans;
    desiredTops = nextTops;
    marks = nextMarks;

    // 첫 페이지의 화면 좌표에서 본문 왼쪽을 얻는다 — 페이지 모드는 page_margin_left, 연속 모드는 CONTINUOUS_VIEW_PADDING만큼 안쪽이다
    const pageEl = editor.pageEls[0];
    const layout = editor.rootAttrs?.layout_mode;
    if (pageEl && layout) {
      const inset = layout.type === 'paginated' ? layout.page_margin_left * zoom : CONTINUOUS_VIEW_PADDING;
      const pageRect = pageEl.getBoundingClientRect();
      bodyLeft = pageRect.left - areaRect.left + inset;
      // 확장 영역은 인셋을 제 padding으로 받고 페이지는 그 안에서 가운데 정렬된다 — 오른쪽 끝에 붙이면
      // 창이 넓을수록 카드가 원고에서 멀어진다. 레일이 그렇듯 컬럼도 페이지에 붙어야 한다.
      pageRight = pageRect.right - areaRect.left;
    }
  };

  // 좌표는 판이 바뀔 때만 다시 잰다 — 스크롤은 브라우저가 콘텐츠와 함께 옮긴다
  $effect(() => {
    void publishedRevision;
    void itemsKey;
    untrack(measure);
  });

  // 인셋이 없는 팝오버 모드에선 본문 왼쪽에 GUTTER가 통째로 없을 수 있다 — 있는 만큼만 받아
  // layoutRails가 레인 수와 칩 방향으로 접게 한다(원점이 0 아래로 내려가면 칩의 left<0 판정이 무의미해진다).
  // 거터가 47보다 좁아지면 이격을 그만큼 줄인다 — 막대가 사라지는 것보다 본문에 붙는 편이 낫다
  const railGutter = $derived(Math.min(GUTTER, Math.max(0, bodyLeft)));
  const railGap = $derived(Math.min(RAIL_TEXT_GAP, Math.max(0, railGutter - RAIL_WIDTH)));

  const columnReserved = $derived(insetRight > 0);

  let detailOpen = $state(false);

  // 회차가 갈리면 이전 회차의 총평이 열린 채 남지 않는다 — 문이 사라졌다 돌아와도 닫힌 상태로 시작한다
  $effect(() => {
    void margin.selectedRoundId;
    detailOpen = false;
  });

  // 탭은 목록의 표지라 본문 밖 — 제목·부제목과 같은 높이에 선다. 컬럼은 본문 전체를 그대로 덮는다.
  const segmentItems = $derived([
    { label: `이번 회차 ${margin.segmentCounts.open}`, value: 'open' as const },
    { label: `지난 회차 ${margin.segmentCounts.settled}`, value: 'settled' as const },
    { label: `자리 잃음 ${margin.segmentCounts.lost}`, value: 'lost' as const },
  ]);

  // 리플로우·줌·인셋 전환은 확장 영역의 크기로 나타난다
  $effect(() => {
    const area = ctx.editor?.extensionAreaEl;
    if (!area) return;
    const observer = new ResizeObserver(() => measure());
    observer.observe(area);
    return () => observer.disconnect();
  });
</script>

{#if margin.ready}
  <!-- 이 레이어는 에디터의 포인터 표면(extensionAreaEl) 안에 산다 — 막지 않으면 레일·카드의 pointerdown이
       handlePointerDown까지 올라가 캐럿이 튀고 포인터 캡처가 잡혀 click이 버튼 대신 본문에 떨어진다.
       focusin은 editor.focus()로 답글 입력의 포커스를 뺏고, userSelect:none은 카드 글자 선택을 막는다.
       드롭은 막기만 하면 브라우저 기본 동작이 파일을 열어 버리므로, 여기서 삼켜 아무 일도 없게 만든다.
       팝오버는 스크롤러로 포탈되어 이 경로 밖이고, 여기서 막히는 것은 레일·룰러·컬럼이다.
       tabindex는 포커스 탐색을 여기서 멈추기 위한 것이다 — 없으면 클릭이 레이어를 지나쳐
       에디터 쪽 조상에 포커스를 앉히고, 그 focusin이 원고 입력을 도로 잡아채 카드 선택이 지워진다. -->
  <div
    class={css({ position: 'absolute', inset: '0', pointerEvents: 'none', userSelect: 'text', WebkitUserSelect: 'text' })}
    draggable={false}
    onclick={(event) => event.stopPropagation()}
    oncontextmenu={(event) => event.stopPropagation()}
    ondragenter={(event) => event.stopPropagation()}
    ondragleave={(event) => event.stopPropagation()}
    ondragover={(event) => {
      event.preventDefault();
      event.stopPropagation();
    }}
    ondrop={(event) => {
      event.preventDefault();
      event.stopPropagation();
    }}
    onfocusin={(event) => event.stopPropagation()}
    onpointerdown={(event) => event.stopPropagation()}
    role="presentation"
    tabindex="-1"
  >
    <!-- 레일 좌표계의 원점은 본문 왼쪽에서 거터만큼 왼쪽이다 — 컬럼 모드는 인셋이 그 자리를 만들어 주지만
         팝오버 모드는 인셋이 0이라 컨테이너를 직접 그 자리로 옮겨야 막대가 본문 위로 올라오지 않는다.
         빈 거터는 클릭을 본문에 넘긴다 — 막대·칩만 받는다. -->
    <div
      style:left={`${bodyLeft - railGutter}px`}
      style:width={`${railGutter}px`}
      class={css({ position: 'absolute', top: '0', bottom: '0', '& > button': { pointerEvents: 'auto' } })}
    >
      <PrismRail gap={railGap} gutter={railGutter} {spans} />
    </div>

    {#if margin.mode === 'column'}
      {#if columnReserved}
        <!-- 헤더 띠 — 확장 영역 위쪽이라 음수 top으로 올라간다 -->
        <div
          style:left={`${pageRight + COLUMN_GAP}px`}
          style:top={`${-headerHeight}px`}
          style:height={`${headerHeight}px`}
          style:width={`${COLUMN_WIDTH}px`}
          class={css({
            position: 'absolute',
            display: 'flex',
            flexDirection: 'column',
            justifyContent: 'flex-end',
            gap: '8px',
            pointerEvents: 'auto',
          })}
        >
          {#if margin.detailRound !== null}
            <Button style={css.raw({ width: 'full' })} onclick={() => (detailOpen = true)} variant="secondary">총평 읽기</Button>
          {/if}
          <SegmentButtons
            style={css.raw({ width: 'full' })}
            items={segmentItems}
            onselect={margin.setSegment}
            size="sm"
            value={margin.segment}
          />
        </div>

        <div
          style:left={`${pageRight + COLUMN_GAP}px`}
          style:width={`${COLUMN_WIDTH}px`}
          class={css({ position: 'absolute', top: '0', bottom: '0', pointerEvents: 'auto' })}
        >
          <PrismCardColumn {desiredTops} />
        </div>
      {/if}
      <!-- 컬럼은 지적만 세운다 — 강점은 여기서도 팝오버가 받는다 -->
      <PrismCardPopover strengthsOnly />
    {:else}
      <PrismCardPopover />
    {/if}

    <!-- 룰러도 가드 안에 선다 — 밖에 두면 마크의 pointerdown이 본문까지 올라가 캐럿이 튀고,
         잡힌 포인터 캡처가 click을 가로채 활성화 자체가 불발된다.
         변형 조상이 없으므로 position:fixed는 여기서도 뷰포트 기준 그대로다. -->
    <PrismOverviewRuler {marks} />
  </div>

  {#if margin.detailRound !== null}
    <PrismReviewDetail round={margin.detailRound} bind:open={detailOpen} />
  {/if}
{/if}
