<script lang="ts">
  import { shift, size } from '@floating-ui/dom';
  import { css } from '@typie/styled-system/css';
  import { createFloatingActions } from '@typie/ui/actions';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import { untrack } from 'svelte';
  import { quintOut } from 'svelte/easing';
  import { fade } from 'svelte/transition';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { pageRectsToVirtualElement } from '$lib/editor-ffi/geometry';
  import { getMarginContext } from './context.svelte.ts';
  import { COLUMN_GAP, COLUMN_WIDTH } from './margin-view.ts';
  import PrismCard from './PrismCard.svelte';
  import type { ReferenceElement } from '@floating-ui/dom';
  import type { PageRect } from '@typie/editor-ffi/browser';
  import type { Editor } from '$lib/editor-ffi/editor.svelte';

  // 컬럼 모드에서도 강점만은 팝오버로 선다 — 강점은 카드를 세우지 않기로 했고(오너 결정),
  // 그러면 컬럼 모드에서 막대·칩·눈금이 눌리는데 아무것도 뜨지 않는 자리가 남는다
  type Props = { strengthsOnly?: boolean };
  let { strengthsOnly = false }: Props = $props();

  const ctx = getEditorContext();
  const margin = getMarginContext();

  const active = $derived.by(() => {
    const found = margin.items.find((item) => item.id === margin.activeId) ?? null;
    return strengthsOnly && found?.kind !== 'strength' ? null : found;
  });

  const range = $derived.by(() => {
    const editor = ctx.editor;
    // published는 프레임마다 갈리므로 재계산의 방아쇠로 쓰고, 자리는 코어가 든 지금 것으로 잡는다 —
    // 스냅숏 사본의 rects는 tracked_ranges 필드가 설 때만 갈려 리플로우 뒤 옛 자리를 가리킨다
    if (!active || active.rangeIds.length === 0 || !editor?.published) return null;
    // 설치한 id 중 자리를 잃은 것은 코어가 비운다 — 살아 있는 첫 range를 잡는다(첫 대목만 지워진 다중 앵커 지적)
    for (const id of active.rangeIds) {
      const found = editor.trackedRangeForSnapshot(id, editor.appliedSnapshot);
      if (found) return found;
    }
    return null;
  });

  // 카드는 큰 블록이다 — 짧으면 전환이 있었는지조차 보이지 않는다. quintOut ≈ cubic-bezier(0.22, 1, 0.36, 1)
  const FADE_MS = 300;
  const cardFade = { duration: FADE_MS, easing: quintOut };

  const scroller = $derived(ctx.editor?.scrollContainerEl);

  // 가로 자리는 앵커 글자가 아니라 원고 페이지의 가장자리에 건다. 글자에 걸면 그 줄의 폭이 바뀔 때마다
  // 오른쪽 끝이 따라 움직여 카드가 좌우로 흔들리고, 짧게 끝난 줄에서는 카드가 본문 한가운데에 선다.
  // 세로만 앵커를 따르고 가로는 컬럼이 서는 기준선(pageRight + COLUMN_GAP)과 같은 수를 쓴다.
  const pageAnchor = (editor: Editor, rects: PageRect[]): ReferenceElement => {
    const measure = () => {
      const band = pageRectsToVirtualElement(editor, rects).getBoundingClientRect();
      const page = editor.pageEls[rects[0].page_idx]?.getBoundingClientRect();
      return page ? new DOMRect(page.left, band.top, page.width, band.height) : band;
    };
    return { getBoundingClientRect: measure, getClientRects: () => [measure()] };
  };

  const { anchor, floating } = createFloatingActions({
    // -start: 카드 윗변을 앵커 윗변에 맞춘다. 컬럼이 카드를 앵커의 첫 줄 top에 세우는 것과 같은 기준이다
    placement: 'right-start',
    // 본문↔카드 간격은 컬럼 모드와 같은 수를 쓴다 — 모드가 바뀐다고 거리가 달라질 이유가 없다
    offset: COLUMN_GAP,
    middleware: [
      // 세로는 스크롤러가 경계다 — 그래야 툴바 위로 올라가지 않는다
      shift({
        crossAxis: false,
        get boundary() {
          return scroller ?? undefined;
        },
        padding: 8,
      }),
      // 가로는 화면이 경계다. 팝오버 모드는 원고 옆 자리를 포기하고 옆 패널 위로 넘어가는 모드이므로
      // 스크롤러로 가두면 카드가 원고 위로 밀려 들어와 모드의 목적과 정반대가 된다.
      // 이 clamp는 패널조차 없을 만큼 창이 좁을 때 화면 밖으로 나가는 것만 막는다
      shift({ mainAxis: false, crossAxis: true, padding: 8 }),
      size({
        // 높이만 에디터 화면에 묶는다. 폭까지 이 경계로 재면 남은 여백만큼 좁아진다
        get boundary() {
          return scroller ?? undefined;
        },
        padding: 8,
        // 넘치는 높이는 카드가 제 안에서 접는다 — 여기서 스크롤을 걸면 카드의 테두리와 그림자까지
        // 함께 밀려 올라가 상자가 잘린 것처럼 보인다
        apply({ availableHeight, elements }) {
          Object.assign(elements.floating.style, {
            maxWidth: `${COLUMN_WIDTH}px`,
            maxHeight: `${availableHeight}px`,
          });
        },
      }),
    ],
  });

  $effect(() => {
    const editor = ctx.editor;
    if (editor && range && range.rects.length > 0) anchor(pageAnchor(editor, range.rects));
  });

  // 앵커가 에디터 화면 밖으로 나가면 팝오버를 숨긴다 — 닫지는 않는다.
  // visibility로 숨기지 않는다: 카드 안쪽이 visibility를 전환에 싣고 있어 겉과 속이 따로 사라진다.
  // 마운트를 유지해야 쓰던 답글 초안과 펼침 상태가 살아남고, 다시 들어오면 그대로 보인다.
  let anchorVisible = $state(true);
  let popoverEl = $state<HTMLDivElement>();

  const syncAnchorVisible = () => {
    const editor = ctx.editor;
    const el = editor?.scrollContainerEl;
    const rects = range?.rects;
    if (!editor || !el || !rects || rects.length === 0) return;
    const anchorRect = pageRectsToVirtualElement(editor, rects).getBoundingClientRect();
    const box = el.getBoundingClientRect();
    const visible = anchorRect.bottom >= box.top && anchorRect.top <= box.bottom;
    // 안 보이는 곳에 글자가 들어가지 않게 — opacity로 숨기면 포커스는 그대로 남는다
    if (!visible && anchorVisible && popoverEl?.contains(window.document.activeElement)) {
      (window.document.activeElement as HTMLElement).blur();
    }
    anchorVisible = visible;
  };

  // 스크롤은 문서 판을 바꾸지 않아 파생값 재계산으로는 감지되지 않는다 — 스크롤을 직접 듣는다
  $effect(() => {
    const el = ctx.editor?.scrollContainerEl;
    if (!el || !active) {
      anchorVisible = true;
      return;
    }
    untrack(syncAnchorVisible);
    // 원고는 캔버스 렌더다 — 앵커는 페이지 좌표에서 만든 가상 요소라 관찰할 DOM 노드 자체가 없다
    // eslint-disable-next-line unicorn/prefer-observer-apis -- IntersectionObserver를 쓸 대상이 없음
    el.addEventListener('scroll', syncAnchorVisible, { passive: true });
    return () => el.removeEventListener('scroll', syncAnchorVisible);
  });

  // 편집·줌으로 자리가 옮겨진 경우 — 판이 바뀔 때마다 다시 잰다
  $effect(() => {
    void range;
    untrack(syncAnchorVisible);
  });

  // window 리스너로는 원고에 포커스가 있을 때 Escape가 오지 않는다 — 에디터가 먼저 삼킨다.
  // 에디터는 자기 바인딩보다 escape 스택을 먼저 돌리므로, 스택에 실어야 포커스 위치와 무관해진다
  $effect(() => {
    if (margin.activeId === null) return;
    return pushEscapeHandler(() => {
      if (margin.activeId === null) return false;
      margin.activate(null);
      return true;
    });
  });
</script>

{#if active && range && range.rects.length > 0}
  <!-- body로 포탈되어 여백 레이어의 가드 밖이다 — 같은 차단을 여기서 한 번 더 세운다 -->
  <div
    bind:this={popoverEl}
    style:width={`${COLUMN_WIDTH}px`}
    style:opacity={anchorVisible ? undefined : '0'}
    style:pointer-events={anchorVisible ? undefined : 'none'}
    style:transition={`opacity ${FADE_MS}ms cubic-bezier(0.22, 1, 0.36, 1)`}
    class={css({
      display: 'flex',
      flexDirection: 'column',
      zIndex: 'menu',
      pointerEvents: 'auto',
      userSelect: 'text',
      WebkitUserSelect: 'text',
    })}
    onclick={(event) => event.stopPropagation()}
    oncontextmenu={(event) => event.stopPropagation()}
    onfocusin={(event) => event.stopPropagation()}
    onpointerdown={(event) => event.stopPropagation()}
    role="presentation"
    tabindex="-1"
    use:floating
  >
    <!-- 전환은 안쪽에서 받는다 — 바깥 요소의 opacity는 앵커 이탈 숨김이 쓰고 있어,
         켜짐/꺼짐 페이드까지 같은 요소에 얹으면 두 기제가 같은 속성을 다툰다 -->
    <div class={css({ display: 'flex', flexDirection: 'column', minHeight: '0' })} in:fade={cardFade} out:fade={cardFade}>
      <PrismCard expanded={true} item={active} onClose={() => margin.activate(null)} onToggle={() => margin.activate(null)} scrollable />
    </div>
  </div>
{/if}
