<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { tick, untrack } from 'svelte';
  import { anchorQuote } from '$lib/feedback/anchors.ts';
  import ThreadCard from './ThreadCard.svelte';
  import type { FeedbackResult } from '$lib/feedback/types.ts';
  import type { PageData } from './$types';

  type Thread = PageData['threads'][number];

  type Props = {
    threads: Thread[];
    comments: PageData['comments'];
    content: string;
    patterns: FeedbackResult['conclusion']['patterns'];
    activeId: string | null;
    onActivate: (threadId: string | null) => void;
  };

  const { threads, comments, content, patterns, activeId, onActivate }: Props = $props();

  const commentsOf = (threadId: string) => comments.filter((comment) => comment.threadId === threadId);

  // 여백 코멘트 배치 — 카드는 자기 앵커(원고의 첫 마크)와 같은 높이에 붙는다. 겹치면 아래로 밀리고,
  // 활성 카드는 자기 앵커 높이에 정확히 고정된 채 위쪽 카드들이 위로 물러난다. 마크가 없는 스레드는 끝에 쌓인다.
  const GAP = 12;

  let columnEl = $state<HTMLDivElement>();
  let cardEls = $state<Record<string, HTMLDivElement | undefined>>({});
  let tops = $state<Record<string, number>>({});
  let spacerHeight = $state(0);
  let animated = $state(false);

  // 토글 전환 창 — 카드 높이가 grid-rows 애니메이션으로 자라는 동안 중간 높이로 재배치하면 이웃이 성장을
  // 뒤쫓으며 늦게 밀린다(실측 150~300ms 지연). 대신 토글 시점에 최종 높이를 예측해(콘텐츠가 상시 DOM에
  // 있어 scrollHeight로 잰다) 목표를 한 번에 확정하고, 창이 열린 동안 ResizeObserver 재배치를 무시한다 —
  // 이웃 이동(top 0.25s)과 카드 성장(grid-rows 0.25s)이 같은 곡선을 나란히 달린다. 창이 닫히면 실측
  // 높이로 1회 보정한다(예측 오차는 수 px).
  const TOGGLE_WINDOW_MS = 270;
  let heightOverrides: Record<string, number> = {}; // 비반응 의도 — relayout 내부에서만 읽는다
  let suppressUntil = 0;
  let settleTimer: ReturnType<typeof setTimeout> | undefined;
  let prevActive: string | null = null;

  const predictHeight = (id: string, willExpand: boolean): number | null => {
    const el = cardEls[id];
    if (!el) return null;
    const detail = el.querySelector('[data-reveal="detail"]')?.scrollHeight ?? 0;
    const snippet = el.querySelector('[data-reveal="snippet"]')?.scrollHeight ?? 0;
    return el.offsetHeight + (willExpand ? detail - snippet : snippet - detail);
  };

  const relayout = () => {
    if (!columnEl) return;
    const columnTop = columnEl.getBoundingClientRect().top;
    const minTop = 0;
    const entries: { id: string; desired: number; height: number }[] = [];
    for (const thread of threads) {
      const el = cardEls[thread.id];
      if (!el) continue;
      // 구간 스팬은 data-thread-range에 id 여러 개를 실으므로 ~= 로 집는다(+page.svelte activate와 같은 선택자).
      const mark = document.querySelector(`[data-thread-range~="${thread.id}"]`);
      const desired = mark ? Math.max(minTop, mark.getBoundingClientRect().top - columnTop) : Infinity;
      entries.push({ id: thread.id, desired, height: heightOverrides[thread.id] ?? el.offsetHeight });
    }
    entries.sort((a, b) => a.desired - b.desired);

    const next: Record<string, number> = {};
    const layoutDown = (from: number, start: number) => {
      let cursor = start;
      for (let i = from; i < entries.length; i++) {
        const entry = entries[i];
        const top = Math.max(Number.isFinite(entry.desired) ? entry.desired : minTop, cursor);
        next[entry.id] = top;
        cursor = top + entry.height + GAP;
      }
    };

    const activeAt = entries.findIndex((entry) => entry.id === activeId);
    if (activeAt === -1) {
      layoutDown(0, minTop);
    } else {
      const active = entries[activeAt];
      const activeTop = Number.isFinite(active.desired) ? active.desired : minTop;
      next[active.id] = activeTop;
      let ceiling = activeTop;
      for (let i = activeAt - 1; i >= 0; i--) {
        const entry = entries[i];
        // 위 공간이 모자라면 컬럼 상단에서 클램프한다 — 드물게 겹치더라도 밖으로 나가는 것보다 낫다.
        next[entry.id] = Math.max(minTop, Math.min(entry.desired, ceiling - GAP - entry.height));
        ceiling = next[entry.id];
      }
      layoutDown(activeAt + 1, activeTop + active.height + GAP);
    }

    tops = next;
    const maxBottom = Math.max(0, ...entries.map((entry) => next[entry.id] + entry.height));
    // 카드는 absolute라 흐름 높이에 안 잡힌다 — 스페이서로 컬럼 높이를 카드 최하단까지 늘린다.
    spacerHeight = maxBottom + 8;
    if (!animated && entries.length > 0) {
      // 전환은 첫 배치가 "화면에 그려진 뒤"에만 켠다 — 같은 커밋에서 켜면 top 0→N 이동 자체가 애니메이션되어
      // 전 카드가 위에서 미끄러져 내려온다. 이중 rAF가 첫 페인트 이후를 보장한다.
      requestAnimationFrame(() => requestAnimationFrame(() => (animated = true)));
    }
  };

  $effect(() => {
    void threads;
    void tick().then(relayout);
  });

  $effect(() => {
    const current = activeId;
    untrack(() => {
      const prev = prevActive;
      prevActive = current;
      if (prev === current) return;
      heightOverrides = {};
      const toggles: [string | null, boolean][] = [
        [prev, false],
        [current, true],
      ];
      for (const [id, willExpand] of toggles) {
        if (id === null) continue;
        const predicted = predictHeight(id, willExpand);
        if (predicted !== null) heightOverrides[id] = predicted;
      }
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
    // 원고 리플로우(폰트 로드·창 크기)는 행 컨테이너 높이로, 카드 팽창(펼침·답글)은 카드 자신으로 감지한다.
    // 토글 창 안의 발화는 무시한다 — 성장 중간 높이로 재배치하면 확정된 목표가 흔들린다(위 주석 참조).
    const observer = new ResizeObserver(() => {
      if (performance.now() < suppressUntil) return;
      relayout();
    });
    const row = columnEl?.parentElement;
    if (row) observer.observe(row);
    for (const el of Object.values(cardEls)) {
      if (el) observer.observe(el);
    }
    return () => observer.disconnect();
  });

  // 좌표가 계산된 카드만 그린다 — 전역 플래그로 가리면 "플래그는 켜졌는데 좌표가 없는" 간헐 프레임에
  // 전 카드가 top 0 폴백에 포개져 그려진다(실측: 원고 우측 상단의 겹친 텍스트 띠). 카드별 좌표 유무가
  // 표시 조건이면 그 프레임이 성립할 수 없다. 전환은 첫 배치의 페인트 이후에만 켠다(이중 rAF).
  const wrapperRecipe = cva({
    base: { position: 'absolute', insetX: '0' },
    variants: {
      positioned: { true: {}, false: { visibility: 'hidden' } },
      animated: { true: { transition: '[top 0.25s cubic-bezier(0.2, 0, 0, 1)]' }, false: {} },
    },
  });

  // 반복 패턴 박스는 이 지적을 품은 패턴에서만 뜬다 — theme이 비면 문면이 성립하지 않아 접는다.
  const patternOf = (issueIndex: number) => {
    const owner = patterns.find((pattern) => pattern.theme && pattern.issues.includes(issueIndex));
    return owner?.theme ? { theme: owner.theme, count: owner.issues.length } : null;
  };
</script>

<div bind:this={columnEl} class={css({ flexGrow: '1', minWidth: '0', position: 'relative' })}>
  {#each threads as thread (thread.id)}
    <div
      bind:this={cardEls[thread.id]}
      style:top={`${tops[thread.id] ?? 0}px`}
      class={css(wrapperRecipe.raw({ positioned: tops[thread.id] !== undefined, animated }))}
    >
      <ThreadCard
        comments={commentsOf(thread.id)}
        expanded={activeId === thread.id}
        onToggle={() => onActivate(activeId === thread.id ? null : thread.id)}
        pattern={patternOf(thread.issueIndex)}
        quote={anchorQuote(content, thread.anchors)}
        {thread}
      />
    </div>
  {/each}

  {#if threads.length === 0}
    <p
      class={css({
        marginTop: '44px',
        paddingX: '16px',
        paddingY: '36px',
        borderWidth: '1px',
        borderColor: 'border.subtle',
        borderRadius: '10px',
        backgroundColor: 'surface.subtle',
        textAlign: 'center',
        fontSize: '12px',
        color: 'text.faint',
      })}
    >
      이번 리뷰에서는 짚은 곳이 없어요
    </p>
  {/if}

  <div style:height={`${spacerHeight}px`}></div>
</div>
