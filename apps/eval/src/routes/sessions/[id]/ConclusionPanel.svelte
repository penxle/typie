<script lang="ts">
  import { css, cva, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { untrack } from 'svelte';
  import { MediaQuery } from 'svelte/reactivity';
  import IconChevronDown from '~icons/lucide/chevron-down';
  import IconChevronRight from '~icons/lucide/chevron-right';
  import IconMaximize2 from '~icons/lucide/maximize-2';
  import { anchorQuote } from '$lib/feedback/anchors.ts';
  import { issueRefsInclude } from '$lib/feedback/conclusion.ts';
  import { verdictLabel } from '$lib/feedback/verdicts.ts';
  import IssueChips from './IssueChips.svelte';
  import ReviewReaction from './ReviewReaction.svelte';
  import type { FeedbackConclusion, FeedbackResult } from '$lib/feedback/types.ts';
  import type { Verdict } from '$lib/feedback/verdicts.ts';
  import type { PageData } from './$types';

  type Thread = PageData['threads'][number];
  type SectionKey = 'understanding' | 'progress' | 'strengths' | 'verdicts' | 'elevations' | 'clearances' | 'patterns' | 'priorities';

  // 총평의 상주 자리 — 원고·카드와 나란히 서서 오가며 읽는 좌측 패널이다. 다섯 섹션 전부 아코디언이고
  // 펼치면 드로어와 같은 전문이 좁은 폭으로 흐른다(요약본 아님). 기본은 전부 펼침(오너 결정 — 목업의 개폐
  // 상태는 예시였다). 폭이 모자라면(패널 300 + 원고 740 + 카드 최소폭이 안 들어가는 ~1360px 미만)
  // 스트립으로 접히고, 정독은 드로어가 맡는다.
  type Props = {
    conclusion: FeedbackConclusion;
    // 특질 판정 — 결과의 최상위 필드라 총평과 따로 온다. 기준표를 세우는 티어에만 실린다.
    verdicts: Verdict[];
    // 격상 — 자리 잡은(3점) 관점의 다음 단계 제안. 결함이 아니라 지적 스레드가 없고 총평에만 선다.
    elevations: NonNullable<FeedbackResult['elevations']>;
    // 총평 연동에만 쓰는 목록이라 표시 회차가 만든 스레드만 온다(+page.svelte conclusionThreads) — 칩·활성 대조가 전부 이 축이다.
    threads: Thread[];
    content: string;
    activeId: string | null;
    reaction: PageData['reaction'];
    onActivate: (id: string) => void;
    onExpand: () => void;
  };

  const { conclusion, verdicts, elevations, threads, content, activeId, reaction, onActivate, onExpand }: Props = $props();

  const narrow = new MediaQuery('(max-width: 1359px)', false);

  const understandingLines = $derived((conclusion.understanding ?? '').split('\n').filter((line) => line.trim().length > 0));
  // 진전 서술은 재리뷰 회차에만 온다 — 1회차·구 데이터는 필드가 없어 섹션 자체가 서지 않는다.
  const progressLines = $derived((conclusion.progress ?? '').split('\n').filter((line) => line.trim().length > 0));

  // 인용은 카드의 인용 블록과 같은 규칙으로 자른다(anchorQuote). 앵커가 원문과 안 맞으면 발췌를 그대로 잇는다 —
  // 원고에 설 자리(스팬)가 없으니 클릭 연동도 함께 접는다.
  const quoteOf = (strength: (typeof conclusion.strengths)[number]) => {
    const quote = anchorQuote(content, [strength]);
    if (quote.length > 0) return { text: quote, linked: true };
    const fallback = strength.head === strength.tail ? strength.head : `${strength.head} ⋯ ${strength.tail}`;
    return { text: fallback, linked: false };
  };

  let expanded = $state<Record<SectionKey, boolean>>({
    understanding: true,
    progress: true,
    strengths: true,
    verdicts: true,
    elevations: true,
    clearances: true,
    patterns: true,
    priorities: true,
  });

  // 칩 연동 — 활성 카드(또는 강점)를 참조하는 자리가 접힌 섹션에 숨어 있으면 펼치고, 펼침(250ms)이 끝난 뒤
  // 그 자리로 스크롤한다. 같은 스레드의 칩은 습관·순서 여러 항목에 설 수 있어 첫 매치로 점프하지 않는다 —
  // nearest의 뜻(보이면 안 움직임)을 매치 전체로 넓혀, 이미 보이는 자리(클릭한 칩 자신 포함)가 있으면 그대로
  // 두고 없을 때만 스크롤이 가장 적게 드는 자리를 집는다. 하이라이트는 칩·항목 자신의 activeId 대조가
  // 담당한다(칩 단위만 — 오너 결정).
  let bodyEl = $state<HTMLDivElement>();
  let scrollTimer: ReturnType<typeof setTimeout> | undefined;

  // 참조 대조는 스레드 자체로 한다 — 번호(구 결과)와 id(지적을 id로 다루는 티어)가 양쪽 다 오므로 인덱스 하나로
  // 좁힐 수 없다(issueRefsInclude).
  const activeThread = $derived.by(() => {
    if (activeId === null || activeId.startsWith('strength.')) return null;
    return threads.find((thread) => thread.id === activeId) ?? null;
  });

  const patternsHaveActive = $derived(conclusion.patterns.some((entry) => issueRefsInclude(entry.issues, activeThread)));
  const prioritiesHaveActive = $derived(conclusion.priorities.some((entry) => issueRefsInclude(entry.issues, activeThread)));

  $effect(() => {
    const id = activeId;
    if (id === null) return;
    untrack(() => {
      if (id.startsWith('strength.')) {
        if (conclusion.strengths.length === 0) return;
        expanded.strengths = true;
      } else {
        const thread = threads.find((entry) => entry.id === id);
        if (!thread) return;
        const inPatterns = conclusion.patterns.some((pattern) => issueRefsInclude(pattern.issues, thread));
        const inPriorities = conclusion.priorities.some((priority) => issueRefsInclude(priority.issues, thread));
        if (!inPatterns && !inPriorities) return;
        if (inPatterns) expanded.patterns = true;
        if (inPriorities) expanded.priorities = true;
      }
      clearTimeout(scrollTimer);
      scrollTimer = setTimeout(() => {
        if (!bodyEl) return;
        const viewport = bodyEl.getBoundingClientRect();
        let target: Element | null = null;
        let best = Infinity;
        for (const mark of bodyEl.querySelectorAll(`[data-conclusion-mark~="${id}"]`)) {
          const rect = mark.getBoundingClientRect();
          const distance = Math.max(0, viewport.top - rect.top, rect.bottom - viewport.bottom);
          if (distance < best) {
            best = distance;
            target = mark;
          }
        }
        if (target && best > 0) target.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      }, 260);
    });
  });

  $effect(() => () => clearTimeout(scrollTimer));

  const rowClass = css({
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
    width: 'full',
    paddingX: '14px',
    paddingY: '11px',
    textAlign: 'left',
    cursor: 'pointer',
  });

  const rowTitleRecipe = cva({
    base: { flexGrow: '1', minWidth: '0', fontSize: '12px', fontWeight: 'semibold', transition: '[color 0.25s ease]' },
    variants: {
      open: {
        true: { color: 'text.default' },
        false: { color: 'text.subtle' },
      },
    },
  });

  const chevronRecipe = cva({
    base: { display: 'inline-flex', flex: 'none', color: 'text.disabled', transition: '[transform 0.25s cubic-bezier(0.2, 0, 0, 1)]' },
    variants: {
      open: { true: { transform: '[rotate(180deg)]' }, false: {} },
    },
  });

  // ThreadCard의 개폐와 같은 문법(grid-rows 0fr↔1fr · 0.25s) — 화면 전체가 한 곡선을 쓴다.
  const revealRecipe = cva({
    base: {
      display: 'grid',
      transition: '[grid-template-rows 0.25s cubic-bezier(0.2, 0, 0, 1), visibility 0.25s]',
    },
    variants: {
      shown: {
        true: { gridTemplateRows: '[1fr]' },
        false: { gridTemplateRows: '[0fr]', visibility: 'hidden' },
      },
    },
  });

  const revealInnerClass = css({ overflow: 'hidden', minHeight: '0' });
  const sectionBodyClass = css({ display: 'flex', flexDirection: 'column', gap: '14px', paddingX: '14px', paddingBottom: '16px' });

  // 활성 항목이 있는 섹션에서 나머지 항목은 잠시 낮은 명도로 물러난다 — 배경을 칠하지 않고도 자리가 잡힌다.
  const itemRecipe = cva({
    base: { transition: '[opacity 0.25s cubic-bezier(0.2, 0, 0, 1)]' },
    variants: {
      dimmed: { true: { opacity: '55' }, false: {} },
    },
  });

  // 활성 강점의 표시는 원고(초록 하이라이트·레일 반전)와 주변 항목의 명도 후퇴가 맡는다 — 인용 자체엔 장식이 없다.
  const quoteRecipe = cva({
    base: { width: 'full', textAlign: 'left' },
    variants: {
      linked: { true: { cursor: 'pointer' }, false: {} },
    },
  });

  const themeClass = css({ fontSize: '12px', fontWeight: 'semibold', color: 'text.default', lineHeight: '[1.5]' });
  // 좁은 상주 패널에서는 판정을 낱말로만 세운다 — 눈금은 정독 자리(드로어)의 몫이다.
  const levelClass = css({ flex: 'none', fontSize: '11px', color: 'text.faint' });
  const noteClass = css({ marginTop: '3px', fontSize: '12px', lineHeight: '[1.7]', color: 'text.faint' });
</script>

{#snippet sectionHead(title: string, count: number | null, key: SectionKey)}
  <button class={rowClass} aria-expanded={expanded[key]} onclick={() => (expanded[key] = !expanded[key])} type="button">
    <span class={css(rowTitleRecipe.raw({ open: expanded[key] }))}>{title}</span>
    {#if count !== null}
      <span class={css({ flex: 'none', fontSize: '10px', fontWeight: 'semibold', color: 'text.faint' })}>{count}</span>
    {/if}
    <span class={css(chevronRecipe.raw({ open: expanded[key] }))}>
      <Icon icon={IconChevronDown} size={12} />
    </span>
  </button>
{/snippet}

{#if narrow.current}
  <button
    class={flex({
      direction: 'column',
      align: 'center',
      gap: '6px',
      width: '52px',
      flex: 'none',
      paddingY: '14px',
      borderRightWidth: '1px',
      borderColor: 'border.default',
      backgroundColor: 'surface.default',
      cursor: 'pointer',
    })}
    aria-label="편집자의 총평 열기"
    data-drawer-return
    onclick={onExpand}
    type="button"
  >
    <span
      class={flex({
        align: 'center',
        justify: 'center',
        flex: 'none',
        size: '26px',
        borderRadius: 'full',
        backgroundColor: 'accent.brand.subtle',
        fontSize: '10px',
        fontWeight: 'bold',
        color: 'text.brand',
      })}
    >
      AI
    </span>
    <span class={css({ fontSize: '11px', fontWeight: 'semibold', color: 'text.subtle' })}>총평</span>
    <span
      class={flex({
        align: 'center',
        justify: 'center',
        flex: 'none',
        size: '24px',
        marginTop: 'auto',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '6px',
        color: 'text.subtle',
      })}
    >
      <Icon icon={IconChevronRight} size={12} />
    </span>
  </button>
{:else}
  <aside
    class={flex({
      direction: 'column',
      width: '300px',
      flex: 'none',
      minHeight: '0',
      borderRightWidth: '1px',
      borderColor: 'border.default',
      backgroundColor: 'surface.default',
    })}
  >
    <div
      class={flex({
        align: 'center',
        gap: '8px',
        flex: 'none',
        paddingX: '14px',
        paddingY: '12px',
        borderBottomWidth: '1px',
        borderColor: 'border.subtle',
      })}
    >
      <span class={css({ flexGrow: '1', fontSize: '13px', fontWeight: 'bold', color: 'text.default' })}>편집자의 총평</span>
      <button
        class={flex({
          align: 'center',
          justify: 'center',
          flex: 'none',
          size: '24px',
          borderWidth: '1px',
          borderColor: 'border.default',
          borderRadius: '6px',
          backgroundColor: 'surface.default',
          color: 'text.subtle',
          cursor: 'pointer',
          _hover: { borderColor: 'border.strong' },
        })}
        aria-label="펼쳐 읽기"
        data-drawer-return
        onclick={onExpand}
        title="펼쳐 읽기"
        type="button"
      >
        <Icon icon={IconMaximize2} size={12} />
      </button>
    </div>

    <div bind:this={bodyEl} class={css({ flexGrow: '1', minHeight: '0', overflowY: 'auto', scrollbarWidth: 'none' })}>
      {#if understandingLines.length > 0}
        <div class={css({ borderBottomWidth: '1px', borderColor: 'border.subtle' })}>
          {@render sectionHead('작품을 이렇게 읽었어요', null, 'understanding')}
          <div class={css(revealRecipe.raw({ shown: expanded.understanding }))}>
            <div class={revealInnerClass}>
              <div class={css({ display: 'flex', flexDirection: 'column', gap: '10px', paddingX: '14px', paddingBottom: '16px' })}>
                {#each understandingLines as line, index (index)}
                  <p class={css({ fontSize: '12px', lineHeight: '[1.75]', color: 'text.subtle' })}>{line}</p>
                {/each}
              </div>
            </div>
          </div>
        </div>
      {/if}

      {#if progressLines.length > 0}
        <div class={css({ borderBottomWidth: '1px', borderColor: 'border.subtle' })}>
          {@render sectionHead('지난 리뷰와 달라진 점', null, 'progress')}
          <div class={css(revealRecipe.raw({ shown: expanded.progress }))}>
            <div class={revealInnerClass}>
              <div class={css({ display: 'flex', flexDirection: 'column', gap: '10px', paddingX: '14px', paddingBottom: '16px' })}>
                {#each progressLines as line, index (index)}
                  <p class={css({ fontSize: '12px', lineHeight: '[1.75]', color: 'text.subtle' })}>{line}</p>
                {/each}
              </div>
            </div>
          </div>
        </div>
      {/if}

      {#if conclusion.strengths.length > 0}
        <div class={css({ borderBottomWidth: '1px', borderColor: 'border.subtle' })}>
          {@render sectionHead('잘 작동하는 대목', conclusion.strengths.length, 'strengths')}
          <div class={css(revealRecipe.raw({ shown: expanded.strengths }))}>
            <div class={revealInnerClass}>
              <div class={sectionBodyClass}>
                {#each conclusion.strengths as strength, index (index)}
                  {@const quote = quoteOf(strength)}
                  {@const id = `strength.${index}`}
                  <div class={css(itemRecipe.raw({ dimmed: activeId?.startsWith('strength.') === true && activeId !== id }))}>
                    <button
                      class={css(quoteRecipe.raw({ linked: quote.linked }))}
                      data-conclusion-mark={id}
                      disabled={!quote.linked}
                      onclick={() => onActivate(id)}
                      type="button"
                    >
                      <span class={css({ fontFamily: 'RIDIBatang', fontSize: '13px', lineHeight: '[1.8]', color: 'text.default' })}>
                        「{quote.text}」
                      </span>
                    </button>
                    {#if strength.body}
                      <p class={css({ marginTop: '5px', fontSize: '11px', lineHeight: '[1.65]', color: 'text.faint' })}>{strength.body}</p>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          </div>
        </div>
      {/if}

      {#if verdicts.length > 0}
        <div class={css({ borderBottomWidth: '1px', borderColor: 'border.subtle' })}>
          {@render sectionHead('관점마다 서 있는 자리', verdicts.length, 'verdicts')}
          <div class={css(revealRecipe.raw({ shown: expanded.verdicts }))}>
            <div class={revealInnerClass}>
              <div class={sectionBodyClass}>
                {#each verdicts as verdict, index (index)}
                  {@const level = verdictLabel(verdict.point)}
                  <div>
                    <div class={flex({ align: 'baseline', gap: '6px' })}>
                      <span class={cx(css({ flexGrow: '1', minWidth: '0' }), themeClass)}>{verdict.trait}</span>
                      {#if level}
                        <span class={levelClass}>{level}</span>
                      {/if}
                    </div>
                    {#if verdict.note}
                      <p class={noteClass}>{verdict.note}</p>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          </div>
        </div>
      {/if}

      {#if elevations.length > 0}
        <div class={css({ borderBottomWidth: '1px', borderColor: 'border.subtle' })}>
          {@render sectionHead('한 걸음 더 가 볼 자리', elevations.length, 'elevations')}
          <div class={css(revealRecipe.raw({ shown: expanded.elevations }))}>
            <div class={revealInnerClass}>
              <div class={sectionBodyClass}>
                {#each elevations as elevation, index (index)}
                  <div>
                    <div class={themeClass}>{elevation.trait}</div>
                    {#if elevation.body}
                      <p class={noteClass}>{elevation.body}</p>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          </div>
        </div>
      {/if}

      {#if (conclusion.clearances?.length ?? 0) > 0}
        <div class={css({ borderBottomWidth: '1px', borderColor: 'border.subtle' })}>
          {@render sectionHead('살펴봤지만 짚지 않은 관점', conclusion.clearances?.length ?? 0, 'clearances')}
          <div class={css(revealRecipe.raw({ shown: expanded.clearances }))}>
            <div class={revealInnerClass}>
              <div class={sectionBodyClass}>
                {#each conclusion.clearances ?? [] as clearance, index (index)}
                  <div>
                    <div class={themeClass}>{clearance.trait}</div>
                    <p class={noteClass}>{clearance.note}</p>
                  </div>
                {/each}
              </div>
            </div>
          </div>
        </div>
      {/if}

      {#if conclusion.patterns.length > 0}
        <div class={css({ borderBottomWidth: '1px', borderColor: 'border.subtle' })}>
          {@render sectionHead('반복해서 나타나는 습관', conclusion.patterns.length, 'patterns')}
          <div class={css(revealRecipe.raw({ shown: expanded.patterns }))}>
            <div class={revealInnerClass}>
              <div class={sectionBodyClass}>
                {#each conclusion.patterns as pattern, index (index)}
                  <div class={css(itemRecipe.raw({ dimmed: patternsHaveActive && !issueRefsInclude(pattern.issues, activeThread) }))}>
                    {#if pattern.theme}
                      <div class={themeClass}>{pattern.theme}</div>
                    {/if}
                    <p class={noteClass}>{pattern.body}</p>
                    <IssueChips {activeId} issues={pattern.issues} {onActivate} {threads} />
                  </div>
                {/each}
              </div>
            </div>
          </div>
        </div>
      {/if}

      {#if conclusion.priorities.length > 0}
        <div>
          {@render sectionHead('손보실 순서', conclusion.priorities.length, 'priorities')}
          <div class={css(revealRecipe.raw({ shown: expanded.priorities }))}>
            <div class={revealInnerClass}>
              <div class={css({ display: 'flex', flexDirection: 'column', gap: '13px', paddingX: '14px', paddingBottom: '16px' })}>
                {#each conclusion.priorities as priority, index (index)}
                  <div
                    class={css(itemRecipe.raw({ dimmed: prioritiesHaveActive && !issueRefsInclude(priority.issues, activeThread) }), {
                      display: 'flex',
                      gap: '9px',
                    })}
                  >
                    <span
                      class={flex({
                        align: 'center',
                        justify: 'center',
                        flex: 'none',
                        size: '18px',
                        marginTop: '1px',
                        borderWidth: '1px',
                        borderColor: 'border.default',
                        borderRadius: 'full',
                        fontSize: '10px',
                        fontWeight: 'bold',
                        color: 'text.subtle',
                      })}
                    >
                      {index + 1}
                    </span>
                    <div class={css({ flexGrow: '1', minWidth: '0' })}>
                      <p class={css({ fontSize: '12px', lineHeight: '[1.7]', color: 'text.subtle' })}>{priority.body}</p>
                      <IssueChips {activeId} issues={priority.issues} {onActivate} {threads} />
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          </div>
        </div>
      {/if}
    </div>

    <div class={css({ flex: 'none', paddingX: '14px', paddingY: '10px', borderTopWidth: '1px', borderColor: 'border.subtle' })}>
      <ReviewReaction {reaction} variant="panel" />
    </div>
  </aside>
{/if}
