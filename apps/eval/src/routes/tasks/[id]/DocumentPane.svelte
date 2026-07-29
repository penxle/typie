<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { computeSegments } from '$lib/domain/highlight.ts';

  type Anchor = { key: string; itemId: string; start: number; end: number };

  type Props = {
    content: string;
    anchors: Anchor[];
    // 지적에 매긴 전역 번호. 본문 어깨에 매다는 뱃지가 목록·레일과 같은 번호를 쓴다.
    numbers?: Record<string, number>;
    hoveredId?: string | null;
    focusedId?: string | null;
    focusedKey?: string | null;
    onHover?: (itemId: string | null) => void;
    onSelect?: (itemId: string) => void;
    onViewport?: (viewport: { start: number; end: number } | null) => void;
  };
  const {
    content,
    anchors,
    numbers,
    hoveredId = null,
    focusedId = null,
    focusedKey = null,
    onHover,
    onSelect,
    onViewport,
  }: Props = $props();

  const itemOfKey = $derived(new Map(anchors.map((a) => [a.key, a.itemId])));
  const segments = $derived(
    computeSegments(
      content,
      anchors.map((a) => ({ start: a.start, end: a.end, feedbackId: a.key })),
    ),
  );

  // 뱃지는 각 앵커가 처음 등장하는 조각에만 매단다 — 조각이 쪼개질 때마다 붙으면 번호가 줄줄이 늘어선다.
  const firstSegmentOf = $derived.by(() => {
    const seen: Record<string, number> = {};
    for (const [i, segment] of segments.entries()) {
      for (const key of segment.feedbackIds) seen[key] ??= i;
    }
    return seen;
  });

  let pane = $state<HTMLElement | undefined>();

  const reducedMotion = () => globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;

  // scrollIntoView는 조상 컨테이너까지 함께 굴린다. 판정 화면은 본문·패널이 각자 스크롤하므로
  // 대상이 든 컨테이너 하나만 직접 굴려야 다른 쪽이 제자리를 잃지 않는다.
  export const seek = (key: string) => {
    const container = pane;
    const target = container?.querySelector<HTMLElement>(`[data-anchor="${CSS.escape(key)}"]`);
    if (!container || !target) return;
    const containerRect = container.getBoundingClientRect();
    const targetRect = target.getBoundingClientRect();
    const offset = targetRect.top - containerRect.top - containerRect.height / 2 + targetRect.height / 2;
    container.scrollTo({ top: container.scrollTop + offset, behavior: reducedMotion() ? 'auto' : 'smooth' });
  };

  export const seekFraction = (fraction: number) => {
    if (!pane) return;
    pane.scrollTop = fraction * pane.scrollHeight - pane.clientHeight / 2;
  };

  const trackViewport = () => {
    const el = pane;
    if (!el || el.scrollHeight <= el.clientHeight) {
      onViewport?.(null);
      return;
    }
    onViewport?.({ start: el.scrollTop / el.scrollHeight, end: (el.scrollTop + el.clientHeight) / el.scrollHeight });
  };
</script>

<section
  bind:this={pane}
  class={css({
    flex: '1',
    minWidth: '0',
    overflowY: 'auto',
    overflowAnchor: 'none',
    paddingY: '32px',
    paddingX: '20px',
    // 레일이 스크롤바 역할을 대신한다 — 같은 일을 하는 막대를 둘 두지 않는다.
    scrollbarWidth: 'none',
    ['&::-webkit-scrollbar']: { display: 'none' },
  })}
  onscroll={trackViewport}
>
  <article
    class={css({
      maxWidth: '[720px]',
      marginX: 'auto',
      backgroundColor: 'surface.default',
      borderRadius: '12px',
      boxShadow: 'small',
      paddingX: '56px',
      paddingY: '48px',
      whiteSpace: 'pre-wrap',
      fontSize: '17px',
      lineHeight: '[1.9]',
      color: 'text.default',
      wordBreak: 'break-word',
    })}
  >
    {#each segments as segment, i (i)}
      {#if segment.feedbackIds.length > 0}
        {@const owners = segment.feedbackIds.map((key) => itemOfKey.get(key))}
        {@const active = owners.includes(hoveredId ?? '') || owners.includes(focusedId ?? '')}
        {@const current = segment.feedbackIds.includes(focusedKey ?? '')}
        <span
          class={css({
            position: 'relative',
            backgroundColor: current ? 'amber.400' : active ? 'amber.300' : 'amber.100',
            borderBottomWidth: '2px',
            borderColor: current ? 'amber.600' : 'amber.400',
            _dark: {
              backgroundColor: current ? '[#8a7619]' : active ? '[#6e5f16]' : '[#4a4012]',
              borderColor: current ? '[#c9ad25]' : '[#93801c]',
            },
            borderRadius: '2px',
            color: '[inherit]',
            cursor: 'pointer',
            transition: '[background-color 0.15s ease]',
            scrollMarginBlock: '80px',
          })}
          onclick={() => owners[0] && onSelect?.(owners[0])}
          onkeydown={(e) => {
            if ((e.key === 'Enter' || e.key === ' ') && owners[0]) {
              e.preventDefault();
              onSelect?.(owners[0]);
            }
          }}
          onmouseenter={() => onHover?.(owners[0] ?? null)}
          onmouseleave={() => onHover?.(null)}
          role="button"
          tabindex="0"
        >
          {#each segment.feedbackIds as key, bi (key)}
            {@const itemId = itemOfKey.get(key) ?? key}
            {#if firstSegmentOf[key] === i && numbers?.[itemId] !== undefined}
              <span
                style:left={`${bi * 16}px`}
                class={css({
                  position: 'absolute',
                  top: '[-10px]',
                  zIndex: '1',
                  display: 'inline-flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  width: '14px',
                  height: '14px',
                  borderRadius: 'full',
                  backgroundColor: 'surface.dark',
                  color: 'text.bright',
                  fontSize: '9px',
                  fontWeight: 'bold',
                  lineHeight: '[1]',
                  cursor: 'pointer',
                  userSelect: 'none',
                })}
                data-anchor={key}
                onclick={(e) => {
                  e.stopPropagation();
                  onSelect?.(itemId);
                }}
                onkeydown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    e.stopPropagation();
                    onSelect?.(itemId);
                  }
                }}
                role="button"
                tabindex="0"
              >
                {numbers[itemId]}
              </span>
            {:else if firstSegmentOf[key] === i}
              <span data-anchor={key}></span>
            {/if}
          {/each}{segment.text}
        </span>
      {:else}
        <span>{segment.text}</span>
      {/if}
    {/each}
  </article>
</section>
