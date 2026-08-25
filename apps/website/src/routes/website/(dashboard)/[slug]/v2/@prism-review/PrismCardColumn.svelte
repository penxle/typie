<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { tick, untrack } from 'svelte';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { layoutCards } from './column-layout.ts';
  import { getMarginContext } from './context.svelte.ts';
  import { lanePresentation } from './margin-motion.ts';
  import { COLUMN_WIDTH, edgeJumpLabel } from './margin-view.ts';
  import PrismCard from './PrismCard.svelte';

  type Props = {
    desiredTops: Record<string, number>;
    preparationKey: string;
    onPrepared: (key: string) => void;
  };
  let { desiredTops, preparationKey, onPrepared }: Props = $props();

  const ctx = getEditorContext();
  const margin = getMarginContext();
  const emptyBoxClass = css({
    flex: 'none',
    marginTop: '12px',
    paddingX: '16px',
    paddingY: '36px',
    borderWidth: '1px',
    borderColor: 'border.subtle',
    borderRadius: '10px',
    backgroundColor: 'surface.subtle',
    textAlign: 'center',
    fontSize: '12px',
    color: 'text.faint',
  });

  const cards = $derived(margin.segmentCards);
  const anchored = $derived(margin.segment !== 'lost');
  const presentation = $derived(lanePresentation(margin.presentationProgress));
  const presentationAnimating = $derived(margin.presentationProgress < 1);

  // 갈래가 바뀌면 카드가 새로 마운트되어 top 0에서 시작한다 — 전환이 켜진 채면 전 카드가 위에서 미끄러진다.
  // 첫 배치와 같은 취급으로 되돌리면 relayout의 이중 rAF가 제자리에 세운 뒤 다시 켠다.
  $effect(() => {
    void margin.segment;
    untrack(() => (animated = false));
  });

  const emptyCopy = $derived(
    margin.segment === 'settled'
      ? '지난 회차에서 정리된 피드백이 여기 모여요'
      : margin.segment === 'lost'
        ? '자리를 잃은 피드백이 없어요'
        : '이번 회차에서는 짚은 곳이 없어요',
  );

  // items는 적용 스냅숏마다 새로 지어지고 desiredTops도 그때마다 새 객체다 — 신원으로 재배치를 걸면
  // 타이핑 한 번마다 전 카드의 offsetHeight를 읽는다. 배치는 카드 집합과 좌표가 실제로 바뀔 때만 돈다
  const cardsKey = $derived(cards.map((item) => item.id).join(' '));
  const desiredKey = $derived(
    Object.entries(desiredTops)
      .map(([id, top]) => `${id}:${top}`)
      .join(' '),
  );

  let columnEl = $state<HTMLDivElement>();
  let cardEls = $state<Record<string, HTMLDivElement | undefined>>({});
  let tops = $state<Record<string, number>>({});
  let spacer = $state(0);
  let animated = $state(false);
  let preparedKey: string | null = null;
  let preparationFrame: number | undefined;
  let preparationPaintFrame: number | undefined;

  const TOGGLE_WINDOW_MS = 270;
  let heightOverrides: Record<string, number> = {};
  let suppressUntil = 0;
  let settleTimer: ReturnType<typeof setTimeout> | undefined;
  let previousActive: string | null = null;

  const cancelPreparation = () => {
    if (preparationFrame !== undefined) cancelAnimationFrame(preparationFrame);
    if (preparationPaintFrame !== undefined) cancelAnimationFrame(preparationPaintFrame);
    preparationFrame = undefined;
    preparationPaintFrame = undefined;
  };

  const schedulePrepared = () => {
    const key = preparationKey;
    if (preparedKey === key) return;
    cancelPreparation();
    // 첫 rAF에서 배치 결과가 DOM에 반영되고 한 번 그려진 뒤, 다음 rAF에서 열림을 admit한다.
    preparationFrame = requestAnimationFrame(() => {
      preparationFrame = undefined;
      preparationPaintFrame = requestAnimationFrame(() => {
        preparationPaintFrame = undefined;
        if (preparationKey !== key) return;
        preparedKey = key;
        onPrepared(key);
      });
    });
  };

  const syncContentBottomOverflow = (cardExtent: number) => {
    const editor = ctx.editor;
    const scroll = ctx.scroll;
    const area = editor?.extensionAreaEl;
    const lastPage = editor?.pageEls[editor.pageSizes.length - 1];
    if (!scroll || !area || !lastPage) return;

    const contentBottom = lastPage.getBoundingClientRect().bottom - area.getBoundingClientRect().top;
    scroll.setContentBottomOverflow(Math.max(0, cardExtent - contentBottom));
  };

  // 성장 중간 높이로 재배치하면 이웃이 성장을 뒤쫓으며 늦게 밀린다 — 최종 높이를 미리 확정한다
  const predictHeight = (id: string, willExpand: boolean): number | null => {
    const el = cardEls[id];
    if (!el) return null;
    const detail = el.querySelector('[data-reveal="detail"]')?.scrollHeight ?? 0;
    const snippet = el.querySelector('[data-reveal="snippet"]')?.scrollHeight ?? 0;
    return el.offsetHeight + (willExpand ? detail - snippet : snippet - detail);
  };

  const relayout = () => {
    const entries = cards
      .map((item) => {
        const el = cardEls[item.id];
        if (!el) return null;
        return {
          id: item.id,
          desired: desiredTops[item.id] ?? Infinity,
          height: heightOverrides[item.id] ?? el.offsetHeight,
        };
      })
      .filter((entry) => entry !== null);

    const result = layoutCards(entries, margin.activeId);
    tops = result.tops;
    spacer = result.spacer;
    syncContentBottomOverflow(result.spacer);

    // 같은 커밋에서 전환을 켜면 top 0→N 이동 자체가 애니메이션되어 전 카드가 위에서 미끄러진다
    if (!animated && entries.length > 0) {
      requestAnimationFrame(() => requestAnimationFrame(() => (animated = true)));
    }

    updateEdgeCounts();
    // 카드 이동·성장이 0.25s 곡선을 달리는 동안의 셈은 이동 전 좌표다 — 곡선이 끝난 뒤 한 번 다시 센다
    clearTimeout(edgeTimer);
    edgeTimer = setTimeout(updateEdgeCounts, 300);

    if (entries.length === cards.length) schedulePrepared();
  };

  // 화면 밖 카드 수의 가장자리 어포던스 — 스크롤 통 위·아래로 완전히 벗어난 카드를 센다.
  // 셈은 실좌표(rect) 대조라 배치 계산과 독립이고, 갱신은 스크롤·재배치·재배치 후 정착 시점뿐이다
  let hiddenAbove = $state(0);
  let hiddenBelow = $state(0);
  // 표시용 수는 0으로 안 내린다 — 사라지는 페이드 중에 "0개"가 비치면 어포던스가 거짓말처럼 보인다
  let labelAbove = $state(0);
  let labelBelow = $state(0);
  let edgeTimer: ReturnType<typeof setTimeout> | undefined;

  const updateEdgeCounts = () => {
    const scrollEl = ctx.editor?.scrollContainerEl;
    if (!scrollEl) return;
    const scrollRect = scrollEl.getBoundingClientRect();
    let above = 0;
    let below = 0;
    for (const item of cards) {
      const el = cardEls[item.id];
      if (!el || tops[item.id] === undefined) continue;
      const rect = el.getBoundingClientRect();
      if (rect.bottom <= scrollRect.top + 4) above += 1;
      else if (rect.top >= scrollRect.bottom - 4) below += 1;
    }
    hiddenAbove = above;
    hiddenBelow = below;
    if (above > 0) labelAbove = above;
    if (below > 0) labelBelow = below;
  };

  $effect(() => {
    const scrollEl = ctx.editor?.scrollContainerEl;
    if (!scrollEl) return;
    const handler = () => updateEdgeCounts();
    scrollEl.addEventListener('scroll', handler, { passive: true });
    return () => scrollEl.removeEventListener('scroll', handler);
  });

  // 어포던스 클릭 = 그 방향의 가장 가까운(가려진) 카드로 스무스 스크롤. 후보는 클릭 시점의 실좌표로
  // 다시 고른다 — 위는 하단이 가장 낮은 카드, 아래는 상단이 가장 높은 카드. 착지는 화면 중앙(오너 결정 —
  // 가장자리에 붙여 세우면 밴드에 깔리고 눈이 못 잡는다)
  const jumpEdge = (direction: 'up' | 'down') => {
    const scrollEl = ctx.editor?.scrollContainerEl;
    if (!scrollEl) return;
    const scrollRect = scrollEl.getBoundingClientRect();
    let target: HTMLDivElement | null = null;
    let best = direction === 'up' ? -Infinity : Infinity;
    for (const item of cards) {
      const el = cardEls[item.id];
      if (!el || tops[item.id] === undefined) continue;
      const rect = el.getBoundingClientRect();
      if (direction === 'up' && rect.bottom <= scrollRect.top + 4 && rect.bottom > best) {
        best = rect.bottom;
        target = el;
      } else if (direction === 'down' && rect.top >= scrollRect.bottom - 4 && rect.top < best) {
        best = rect.top;
        target = el;
      }
    }
    target?.scrollIntoView({ behavior: 'smooth', block: 'center' });
  };

  $effect(() => () => clearTimeout(edgeTimer));

  $effect(() => {
    void preparationKey;
    void cardsKey;
    void desiredKey;
    void tick().then(relayout);
  });

  $effect(() => () => cancelPreparation());

  $effect(() => {
    const current = margin.activeId;
    untrack(() => {
      if (previousActive === current) return;
      heightOverrides = {};
      for (const [id, willExpand] of [
        [previousActive, false],
        [current, true],
      ] as [string | null, boolean][]) {
        if (id === null) continue;
        const predicted = predictHeight(id, willExpand);
        if (predicted !== null) heightOverrides[id] = predicted;
      }
      previousActive = current;
      if (Object.keys(heightOverrides).length > 0) {
        suppressUntil = performance.now() + TOGGLE_WINDOW_MS;
        clearTimeout(settleTimer);
        settleTimer = setTimeout(() => {
          heightOverrides = {};
          relayout();
        }, TOGGLE_WINDOW_MS + 20);
      }
    });
    void tick().then(relayout);
  });

  $effect(() => () => clearTimeout(settleTimer));

  $effect(() => {
    const scroll = ctx.scroll;
    return () => scroll?.setContentBottomOverflow(0);
  });

  // 원고 리플로우는 컨테이너로, 카드 성장은 카드 자신으로 감지한다.
  // 토글 창 안의 발화는 무시한다 — 성장 중간 높이로 재배치하면 확정된 목표가 흔들린다
  $effect(() => {
    const observer = new ResizeObserver(() => {
      if (performance.now() < suppressUntil) return;
      relayout();
    });
    const container = columnEl?.parentElement;
    if (container) observer.observe(container);
    for (const el of Object.values(cardEls)) {
      if (el) observer.observe(el);
    }
    return () => observer.disconnect();
  });

  // 좌표가 없는 프레임에 그리면 전 카드가 top 0에 포개진다
  const wrapperRecipe = cva({
    base: { position: 'absolute', insetX: '0' },
    variants: {
      positioned: { true: {}, false: { visibility: 'hidden' } },
      animated: { true: { transition: '[top 0.25s cubic-bezier(0.2, 0, 0, 1)]' }, false: {} },
    },
  });

  // 가장자리 어포던스의 자리 — 높이 0의 sticky 라인이라 카드 배치·스페이서 셈에 영향을 주지 않고,
  // 필은 그 라인에서 넘쳐 그려진다(아래쪽은 라인 위로 끌어올린다). 높이 0의 sticky 라인은 flex 기본값
  // (stretch)이 자식 밴드를 0 높이로 누르므로, 밴드가 라인 바깥으로 자라도록 정렬로 방향을 준다
  const edgeLineRecipe = cva({
    base: {
      position: 'sticky',
      zIndex: '1',
      display: 'flex',
      justifyContent: 'center',
      height: '0',
      overflow: 'visible',
      pointerEvents: 'none',
    },
    variants: {
      edge: {
        top: { top: '28px', alignItems: 'flex-start' },
        bottom: { bottom: '28px', alignItems: 'flex-end' },
      },
    },
  });

  // 필이 아니라 선과 글자다(오너 결정) — 라벨 좌우로 가는 선이 컬럼 폭을 채우는 구분선 형태.
  // 카드 위를 지날 때의 겹침은 뒤에 깔린 프로스트 층이 눕힌다.
  // 클릭면이기도 하다 — 누르면 그 방향의 가장 가까운 카드로 간다. 숨은 동안(opacity 0)은 클릭도 죽인다
  const edgeRowRecipe = cva({
    base: {
      position: 'relative',
      display: 'flex',
      alignItems: 'center',
      gap: '8px',
      width: 'full',
      paddingX: '8px',
      paddingY: '10px',
      cursor: 'pointer',
      transition: '[opacity 0.25s cubic-bezier(0.2, 0, 0, 1)]',
    },
    variants: {
      shown: { true: { pointerEvents: 'auto' }, false: { opacity: '0', pointerEvents: 'none' } },
    },
  });

  const edgeLineSegClass = css({ flexGrow: '1', height: '1px', backgroundColor: 'border.default' });

  // 서로 다른 radius의 층으로 세로 progressive blur를 근사한다. 범위는 원래보다 좁히고, 같은 요소를
  // 계속 마운트한 채 0px에서 각 radius로 바꿔 어포던스의 opacity와 함께 보간한다.
  const edgeBlurRecipe = cva({
    base: {
      position: 'absolute',
      insetX: '0',
      zIndex: '[-2]',
      pointerEvents: 'none',
      backdropFilter: 'auto',
      transition: '[backdrop-filter 0.2s cubic-bezier(0.2, 0, 0, 1)]',
    },
    variants: {
      layer: {
        outermost: {
          insetY: '-10px',
          maskImage: '[linear-gradient(to bottom, transparent, black 42%, black 58%, transparent)]',
        },
        outer: {
          insetY: '-6px',
          maskImage: '[linear-gradient(to bottom, transparent, black 38%, black 62%, transparent)]',
        },
        inner: {
          insetY: '-2px',
          maskImage: '[linear-gradient(to bottom, transparent, black 34%, black 66%, transparent)]',
        },
        innermost: {
          insetY: '2px',
          maskImage: '[linear-gradient(to bottom, transparent, black 28%, black 72%, transparent)]',
        },
      },
      shown: {
        true: {},
        false: { backdropBlur: '[0px]' },
      },
    },
    compoundVariants: [
      { layer: 'outermost', shown: true, css: { backdropBlur: '[0.5px]' } },
      { layer: 'outer', shown: true, css: { backdropBlur: '1px' } },
      { layer: 'inner', shown: true, css: { backdropBlur: '[1.5px]' } },
      { layer: 'innermost', shown: true, css: { backdropBlur: '2px' } },
    ],
  });

  // tint는 blur층마다 중첩하지 않고 별도 한 층으로만 얹는다. 중앙의 surface 색이 복잡한 카드 경계를
  // 정돈하되, alpha mask가 위아래를 완전 투명으로 보내 별도의 판이나 띠가 남지 않게 한다.
  const edgeScrimClass = css({
    position: 'absolute',
    insetX: '0',
    insetY: '-8px',
    zIndex: '[-1]',
    pointerEvents: 'none',
    backgroundColor: 'surface.default/45',
    maskImage: '[linear-gradient(to bottom, transparent, black 38%, black 62%, transparent)]',
  });

  const edgeTextClass = css({
    display: 'inline-flex',
    alignItems: 'center',
    gap: '4px',
    flex: 'none',
    fontSize: '11px',
    color: 'text.faint',
    whiteSpace: 'nowrap',
  });

  const presentationClass = css({ transformOrigin: 'center' });
</script>

<!-- 세그먼트 행은 카드 좌표계 **밖**에 둔다 — 안에 두면 절대 배치된 카드가 행 높이만큼 밀린다.
     안쪽 컨테이너는 남은 높이를 그대로 채운다: sticky 가장자리의 컨테이닝 블록이 콘텐츠 높이에 머물면
     카드가 원고보다 짧을 때 스크롤 말미에 sticky가 바닥에 눌려 마지막 카드 밑으로 딸려 올라간다 -->
<div style:width={`${COLUMN_WIDTH}px`} class={flex({ direction: 'column', flex: 'none', minWidth: '0', height: 'full' })}>
  {#if cards.length === 0}
    <p
      style:opacity={presentation.opacity}
      style:transform={`scale(${presentation.scale})`}
      style:will-change={presentationAnimating ? 'opacity, transform' : 'auto'}
      class={`${emptyBoxClass} ${presentationClass}`}
    >
      {emptyCopy}
    </p>
  {:else if anchored}
    <div bind:this={columnEl} class={css({ position: 'relative', flexGrow: '1' })}>
      <div style:opacity={presentation.opacity} class={css(edgeLineRecipe.raw({ edge: 'top' }))}>
        <button class={css(edgeRowRecipe.raw({ shown: hiddenAbove > 0 }))} onclick={() => jumpEdge('up')} type="button">
          <div class={css(edgeBlurRecipe.raw({ layer: 'outermost', shown: hiddenAbove > 0 }))}></div>
          <div class={css(edgeBlurRecipe.raw({ layer: 'outer', shown: hiddenAbove > 0 }))}></div>
          <div class={css(edgeBlurRecipe.raw({ layer: 'inner', shown: hiddenAbove > 0 }))}></div>
          <div class={css(edgeBlurRecipe.raw({ layer: 'innermost', shown: hiddenAbove > 0 }))}></div>
          <div class={edgeScrimClass}></div>
          <span class={edgeLineSegClass}></span>
          <span class={edgeTextClass}>
            <Icon icon={ChevronUpIcon} size={10} />
            {edgeJumpLabel('top', labelAbove)}
          </span>
          <span class={edgeLineSegClass}></span>
        </button>
      </div>

      {#each cards as item (item.id)}
        <div
          bind:this={cardEls[item.id]}
          style:opacity={presentation.opacity}
          style:top={`${tops[item.id] ?? 0}px`}
          style:transform={`scale(${presentation.scale})`}
          style:will-change={presentationAnimating ? 'opacity, transform' : 'auto'}
          class={`${css(wrapperRecipe.raw({ positioned: tops[item.id] !== undefined, animated }))} ${presentationClass}`}
        >
          <PrismCard
            expanded={margin.activeId === item.id}
            {item}
            onClose={() => margin.activate(null, 'card')}
            onToggle={() => margin.activate(margin.activeId === item.id ? null : item.id, 'card')}
          />
        </div>
      {/each}

      <div style:height={`${spacer}px`}></div>

      <div style:opacity={presentation.opacity} class={css(edgeLineRecipe.raw({ edge: 'bottom' }))}>
        <button class={css(edgeRowRecipe.raw({ shown: hiddenBelow > 0 }))} onclick={() => jumpEdge('down')} type="button">
          <div class={css(edgeBlurRecipe.raw({ layer: 'outermost', shown: hiddenBelow > 0 }))}></div>
          <div class={css(edgeBlurRecipe.raw({ layer: 'outer', shown: hiddenBelow > 0 }))}></div>
          <div class={css(edgeBlurRecipe.raw({ layer: 'inner', shown: hiddenBelow > 0 }))}></div>
          <div class={css(edgeBlurRecipe.raw({ layer: 'innermost', shown: hiddenBelow > 0 }))}></div>
          <div class={edgeScrimClass}></div>
          <span class={edgeLineSegClass}></span>
          <span class={edgeTextClass}>
            <Icon icon={ChevronDownIcon} size={10} />
            {edgeJumpLabel('bottom', labelBelow)}
          </span>
          <span class={edgeLineSegClass}></span>
        </button>
      </div>
    </div>
  {:else}
    <!-- 자리를 잃은 카드는 맞출 앵커가 없다 — 위에서부터 흐름으로 쌓는다 -->
    <div class={flex({ direction: 'column', gap: '10px', marginTop: '12px' })}>
      {#each cards as item (item.id)}
        <div
          style:opacity={presentation.opacity}
          style:transform={`scale(${presentation.scale})`}
          style:will-change={presentationAnimating ? 'opacity, transform' : 'auto'}
          class={presentationClass}
        >
          <PrismCard
            expanded={margin.activeId === item.id}
            {item}
            onClose={() => margin.activate(null, 'card')}
            onToggle={() => margin.activate(margin.activeId === item.id ? null : item.id, 'card')}
          />
        </div>
      {/each}
    </div>
  {/if}
</div>
