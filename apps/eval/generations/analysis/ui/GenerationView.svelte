<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import type { Snippet } from 'svelte';
  import type { ViewItem } from '$lib/server/run-view.ts';

  // 이 세대는 단계가 하나뿐이고 산출물을 남기지 않는다 — stageKey·artifacts를 쓰지 않는다.
  type Props = {
    items: ViewItem[];
    numbers: Record<string, number>;
    stageKey: string | null;
    artifacts: { label: string; value: unknown } | null;
    focusedId?: string | null;
    onHover?: (itemId: string | null) => void;
    onSelect?: (itemId: string, anchorIndex: number) => void;
    // 총평이 가리키는 지적으로 건너뛴다 — 목록과 본문을 함께 옮겨야 대조가 이어진다.
    onReveal?: (itemId: string) => void;
    control?: Snippet<[ViewItem]>;
    runReview?: Snippet;
  };
  // eslint-disable-next-line svelte/no-unused-props
  const { items, numbers, focusedId = null, onHover, onSelect, onReveal, control, runReview }: Props = $props();

  const of = (kind: string) => items.filter((i) => i.kind === kind).toSorted((a, b) => a.ord - b.ord);

  const characterization = $derived(of('characterization')[0] ?? null);
  const strengths = $derived(of('strength'));
  const patterns = $derived(of('pattern'));
  const priority = $derived(of('priority'));
  const findings = $derived(of('finding'));

  // 동결 세대에는 층위가 없다 — 지적을 한 목록으로 둔다.
  const planFindings = $derived(findings);
  const localFindings: typeof findings = [];

  let tab = $state<'review' | 'plan' | 'local'>('review');
  const shownFindings = $derived(tab === 'local' ? localFindings : planFindings);

  // 총평과 지적을 한 스크롤에 두면, 총평이 가리키는 지적으로 뛴 순간 총평을 잃는다.
  // 각자 스크롤을 갖게 하고 전환 시 위치를 되돌려 읽던 자리를 지킨다.
  let reviewPaneEl = $state<HTMLElement | undefined>();
  let findingsPaneEl = $state<HTMLElement | undefined>();
  const paneScrollTops: Record<'review' | 'plan' | 'local', number> = { review: 0, plan: 0, local: 0 };

  const paneOf = (t: 'review' | 'plan' | 'local') => (t === 'review' ? reviewPaneEl : findingsPaneEl);

  const switchTab = (next: 'review' | 'plan' | 'local') => {
    if (next === tab) return;
    const current = paneOf(tab);
    if (current) paneScrollTops[tab] = current.scrollTop;
    tab = next;
    requestAnimationFrame(() => {
      const el = paneOf(next);
      if (el) el.scrollTop = paneScrollTops[next] ?? 0;
    });
  };

  const reducedMotion = () => globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;

  // scrollIntoView는 조상 컨테이너까지 함께 굴린다. 판정 화면은 본문·패널이 각자 스크롤하므로
  // 대상이 든 컨테이너 하나만 직접 굴려야 다른 쪽이 제자리를 잃지 않는다.
  const scrollWithin = (container: HTMLElement, target: HTMLElement) => {
    const containerRect = container.getBoundingClientRect();
    const targetRect = target.getBoundingClientRect();
    const offset = targetRect.top - containerRect.top - containerRect.height / 2 + targetRect.height / 2;
    container.scrollTo({ top: container.scrollTop + offset, behavior: reducedMotion() ? 'auto' : 'smooth' });
  };

  // 카드로 이동. 총평 탭에 있었다면 지적 탭으로 옮기되, 총평의 스크롤 위치는 그대로 남는다.
  export const focus = (itemId: string) => {
    switchTab(localFindings.some((f) => f.id === itemId) ? 'local' : 'plan');
    requestAnimationFrame(() => {
      const el = document.querySelector<HTMLElement>(`[data-item-card="${CSS.escape(itemId)}"]`);
      if (el && findingsPaneEl) scrollWithin(findingsPaneEl, el);
    });
  };

  export const toggleTab = () => {
    switchTab(tab === 'review' ? 'plan' : tab === 'plan' && localFindings.length > 0 ? 'local' : 'review');
  };

  // 목록이 층위별로 나뉘어도 번호는 전역이다 — 총평 참조와 레일이 같은 번호를 가리켜야 한다.
  const byId = $derived(new Map(items.map((i) => [i.id, i])));

  const located = (item: ViewItem) => item.anchors.filter((a) => a.matchStart !== null && a.matchEnd !== null).length;

  let cursors = $state<Record<string, number>>({});

  const step = (item: ViewItem, delta: number) => {
    const count = located(item);
    if (count === 0) return;
    const next = ((cursors[item.id] ?? 0) + delta + count) % count;
    cursors = { ...cursors, [item.id]: next };
    onSelect?.(item.id, next);
  };

  const sectionClass = css({
    paddingY: '20px',
    borderTopWidth: '1px',
    borderColor: 'border.subtle',
    ['&:first-child']: { paddingTop: '0', borderTopWidth: '0' },
  });
  // 제목은 제목처럼 쓴다. 작은 회색 라벨은 내용을 부속물처럼 보이게 만든다.
  const headingClass = css({ marginBottom: '8px', fontSize: '13px', fontWeight: 'bold', color: 'text.default' });
  const subHeadingClass = css({ marginBottom: '2px', fontSize: '14px', fontWeight: 'bold', color: 'text.default' });
  const bodyClass = css({ fontSize: '14px', lineHeight: '[1.85]', color: 'text.default', whiteSpace: 'pre-wrap' });

  // 밑줄이 번호와 이름을 하나로 묶어야 한다 — 사이가 끊기면 링크가 둘로 보인다.
  // 본문과 같은 색을 쓰면 눌러야 할 것인지 알 수 없으므로 한 단계 흐린 색으로 낮춘다.
  const refClass = css({
    fontSize: '12px',
    color: 'text.subtle',
    textDecoration: 'underline',
    textUnderlineOffset: '[3px]',
    textDecorationColor: 'border.strong',
    cursor: 'pointer',
    transition: '[color 0.12s ease, text-decoration-color 0.12s ease]',
    _hover: { color: 'text.default', textDecorationColor: 'text.default' },
  });

  // 앵커가 하나든 셋이든 같은 결로 보여야 한다. 테두리 버튼은 여백 대비 너무 요란해서
  // 조용한 텍스트 링크로 통일하되, 좌우 여백으로 누를 만한 표적을 확보한다.
  const linkClass = css({
    display: 'inline-flex',
    alignItems: 'center',
    height: '22px',
    paddingX: '6px',
    borderRadius: '4px',
    fontSize: '12px',
    color: 'text.faint',
    cursor: 'pointer',
    fontVariantNumeric: 'tabular-nums',
    transition: '[background-color 0.12s ease, color 0.12s ease]',
    _hover: { backgroundColor: 'surface.muted', color: 'text.default' },
  });

  const tabClass = (selected: boolean) =>
    flex({
      align: 'center',
      justify: 'center',
      gap: '6px',
      flex: '1',
      paddingY: '9px',
      borderBottomWidth: '1px',
      borderColor: selected ? 'text.default' : '[transparent]',
      color: selected ? 'text.default' : 'text.faint',
      fontSize: '13px',
      fontWeight: selected ? 'bold' : 'normal',
      cursor: 'pointer',
      transition: '[color 0.15s ease, border-color 0.15s ease]',
      _hover: { color: 'text.default' },
    });
</script>

<!-- 누를 수 있다는 것은 쉬고 있을 때도 보여야 하지만, 색을 쓸 자리는 아니다 —
     원고 옆에 적힌 참조는 밑줄 하나로 충분하고, 색이 들어가면 종이에서 뜬다. -->
{#snippet refs(item: ViewItem)}
  {@const targets = item.links.map((id) => byId.get(id)).filter((i) => i !== undefined)}
  {#if targets.length > 0}
    <p class={flex({ wrap: 'wrap', gap: '10px', marginTop: '8px' })}>
      {#each targets as target (target.id)}
        <button class={refClass} onclick={() => onReveal?.(target.id)} type="button">
          <span class={css({ fontWeight: 'bold', fontVariantNumeric: 'tabular-nums' })}>{numbers[target.id]}</span>
          &nbsp;{target.facets.category ?? '지적'}
        </button>
      {/each}
    </p>
  {/if}
{/snippet}

<div class={flex({ direction: 'column', minHeight: '0', height: 'full' })}>
  <div class={flex({ paddingX: '16px', borderBottomWidth: '1px', borderColor: 'border.default', flexShrink: '0' })}>
    <button class={tabClass(tab === 'review')} onclick={() => switchTab('review')} type="button">작품 총평</button>
    <button class={tabClass(tab === 'plan')} onclick={() => switchTab('plan')} type="button">
      작품 검토 {planFindings.length}건
    </button>
    {#if localFindings.length > 0}
      <button class={tabClass(tab === 'local')} onclick={() => switchTab('local')} type="button">
        문면 교열 {localFindings.length}건
      </button>
    {/if}
  </div>

  <div
    bind:this={reviewPaneEl}
    style:display={tab === 'review' ? 'block' : 'none'}
    class={css({ paddingX: '20px', paddingY: '20px', overflowY: 'auto', flex: '1', minHeight: '0' })}
  >
    {#if characterization}
      <section class={sectionClass}>
        <h2 class={headingClass}>이 작품을 이렇게 읽었습니다</h2>
        <p class={bodyClass}>{characterization.body}</p>
      </section>
    {/if}

    {#if strengths.length > 0}
      <section class={sectionClass}>
        <h2 class={headingClass}>잘 되고 있는 것</h2>
        <div class={flex({ direction: 'column', gap: '16px' })}>
          {#each strengths as item (item.id)}
            <div>
              <p class={bodyClass}>{item.body}</p>
              <!-- 인용은 강점이 어느 대목인지 가리키는 유일한 단서다. 앵커가 없는 실행도 있어 조건부로 둔다. -->
              {#if item.anchors[0]?.startText}
                <p class={css({ marginTop: '4px', fontSize: '12px', color: 'text.subtle' })}>
                  {item.anchors[0].startText}
                  {#if item.anchors[0].endText && item.anchors[0].endText !== item.anchors[0].startText}
                    … {item.anchors[0].endText}
                  {/if}
                </p>
              {/if}
            </div>
          {/each}
        </div>
      </section>
    {/if}

    {#if patterns.length > 0}
      <section class={sectionClass}>
        <h2 class={headingClass}>되풀이되는 경향</h2>
        <div class={flex({ direction: 'column', gap: '16px' })}>
          {#each patterns as item (item.id)}
            <div>
              {#if item.facets.theme}
                <h3 class={subHeadingClass}>{item.facets.theme}</h3>
              {/if}
              <p class={bodyClass}>{item.body}</p>
              {@render refs(item)}
            </div>
          {/each}
        </div>
      </section>
    {/if}

    {#if priority.length > 0}
      <section class={sectionClass}>
        <h2 class={headingClass}>먼저 손댈 것</h2>
        <!-- 우선순위는 실제로 순서가 정보다. 번호가 장식이 아니라 '이 차례로 하라'는 내용이다. -->
        <ol class={flex({ direction: 'column', gap: '16px' })}>
          {#each priority as item, i (item.id)}
            <li class={css({ display: 'grid', gridTemplateColumns: '[18px minmax(0, 1fr)]', columnGap: '10px' })}>
              <span
                class={css({
                  fontSize: '13px',
                  fontWeight: 'bold',
                  fontVariantNumeric: 'tabular-nums',
                  color: 'text.subtle',
                  lineHeight: '[1.85]',
                  textAlign: 'right',
                })}
              >
                {i + 1}
              </span>
              <div class={css({ minWidth: '0' })}>
                <p class={bodyClass}>{item.body}</p>
                {@render refs(item)}
              </div>
            </li>
          {/each}
        </ol>
      </section>
    {/if}

    {#if runReview}
      <section class={sectionClass}>
        <h2 class={headingClass}>총평 판정</h2>
        {@render runReview()}
      </section>
    {/if}
  </div>

  <div
    bind:this={findingsPaneEl}
    style:display={tab === 'review' ? 'none' : 'block'}
    class={css({ overflowY: 'auto', flex: '1', minHeight: '0', paddingY: '4px' })}
  >
    {#each shownFindings as item (item.id)}
      {@const count = located(item)}
      <!-- 지적은 카드가 아니라 원고 여백에 적힌 메모다. 마흔 건에 상자를 하나씩 두르면
             상자끼리 경쟁해 눈이 갈 곳을 잃는다 — 매다는 번호와 여백만으로 구분한다. -->
      <article
        class={css({
          display: 'grid',
          gridTemplateColumns: '[26px minmax(0, 1fr)]',
          columnGap: '10px',
          paddingX: '20px',
          paddingY: '18px',
          backgroundColor: focusedId === item.id ? 'surface.subtle' : '[transparent]',
          transition: '[background-color 0.15s ease]',
          ['& + &']: { borderTopWidth: '1px', borderColor: 'border.subtle' },
        })}
        data-item-card={item.id}
        onfocus={() => onHover?.(item.id)}
        onmouseenter={() => onHover?.(item.id)}
        onmouseleave={() => onHover?.(null)}
        role="note"
      >
        <span
          class={css({
            display: 'block',
            height: '22px',
            lineHeight: '[22px]',
            textAlign: 'right',
            fontSize: '13px',
            fontWeight: 'bold',
            fontVariantNumeric: 'tabular-nums',
            color: 'text.subtle',
          })}
        >
          {numbers[item.id]}
        </span>

        <div class={css({ minWidth: '0' })}>
          <div class={flex({ align: 'center', gap: '8px', marginBottom: '4px', minHeight: '22px' })}>
            {#if item.facets.category}
              <span class={css({ fontSize: '12px', fontWeight: 'bold', color: 'text.subtle' })}>{item.facets.category}</span>
            {/if}
            <span class={flex({ align: 'center', marginLeft: 'auto', flexShrink: '0' })}>
              {#if count === 0}
                <span class={css({ fontSize: '12px', color: 'text.faint', paddingX: '6px' })}>본문 위치 없음</span>
              {:else if count === 1}
                <button class={linkClass} onclick={() => step(item, 0)} type="button">본문 보기</button>
              {:else}
                <button class={linkClass} aria-label="이전 위치" onclick={() => step(item, -1)} type="button">‹</button>
                <button class={linkClass} onclick={() => step(item, 1)} type="button">
                  본문 {((cursors[item.id] ?? 0) % count) + 1}/{count}
                </button>
                <button class={linkClass} aria-label="다음 위치" onclick={() => step(item, 1)} type="button">›</button>
              {/if}
            </span>
          </div>

          <p class={bodyClass}>{item.body}</p>

          <div class={css({ marginTop: '10px' })}>
            {@render control?.(item)}
          </div>
        </div>
      </article>
    {:else}
      <p class={css({ paddingY: '32px', textAlign: 'center', fontSize: '14px', color: 'text.faint' })}>이 층위의 지적이 없습니다.</p>
    {/each}
  </div>
</div>
