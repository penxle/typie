<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { MediaQuery } from 'svelte/reactivity';
  import IconX from '~icons/lucide/x';
  import { anchorQuote } from '$lib/feedback/anchors.ts';
  import { VERDICT_POINTS, verdictLabel } from '$lib/feedback/verdicts.ts';
  import IssueChips from './IssueChips.svelte';
  import Paragraphs from './Paragraphs.svelte';
  import ReviewReaction from './ReviewReaction.svelte';
  import type { FeedbackConclusion, FeedbackResult } from '$lib/feedback/types.ts';
  import type { Verdict } from '$lib/feedback/verdicts.ts';
  import type { PageData } from './$types';

  type Thread = PageData['threads'][number];

  // 총평의 정독 자리 — 편집자가 보낸 편지 한 통으로 조판한다(오너 확정: 에디토리얼 레터형, 대시보드 기각).
  // 좌측 패널이 확장되듯 화면을 덮되 오른쪽에 원고·카드가 비치는 틈(128px)을 남긴다 — 닫을 수 있다는
  // 어포던스다. 뒤의 3컬럼은 리플로우하지 않는다(오버레이). 칩·인용 클릭은 드로어를 닫으며 그 자리로 간다.
  type Props = {
    open: boolean;
    conclusion: FeedbackConclusion;
    // 특질 판정 — 결과의 최상위 필드라 총평과 따로 온다. 기준표를 세우는 티어에만 실린다.
    verdicts: Verdict[];
    // 격상 — 자리 잡은(3점) 관점의 다음 단계 제안. 결함이 아니라 지적 스레드가 없고 총평에만 선다.
    elevations: NonNullable<FeedbackResult['elevations']>;
    // 머리글의 피드백 수도 칩 연동도 전부 표시 회차 축이다 — 편지가 이 회차의 것이라 세션 누계를 섞지 않는다.
    conclusionThreads: Thread[];
    content: string;
    title: string;
    // 표시 회차 서수 — 실패·중단을 건너뛴 번호라(displayRoundNumbers) 내부 round와 다를 수 있다.
    roundNumber: number;
    finishedAt: number | null;
    activeId: string | null;
    reaction: PageData['reaction'];
    onClose: () => void;
    onActivate: (id: string) => void;
  };

  const {
    open,
    conclusion,
    verdicts,
    elevations,
    conclusionThreads,
    content,
    title,
    roundNumber,
    finishedAt,
    activeId,
    reaction,
    onClose,
    onActivate,
  }: Props = $props();

  const reducedMotion = new MediaQuery('(prefers-reduced-motion: reduce)', false);

  const understandingLines = $derived((conclusion.understanding ?? '').split('\n').filter((line) => line.trim().length > 0));
  // 진전 서술은 재리뷰 회차에만 온다 — 1회차·구 데이터는 필드가 없어 섹션 자체가 서지 않는다.
  const progressLines = $derived((conclusion.progress ?? '').split('\n').filter((line) => line.trim().length > 0));

  const dateLabel = $derived(
    finishedAt === null
      ? null
      : new Intl.DateTimeFormat('ko-KR', { month: 'long', day: 'numeric', timeZone: 'Asia/Seoul' }).format(finishedAt),
  );

  // 강점은 의미 진술 티어(high)의 총평에만 선다 — 없는 티어(medium)는 빈 목록으로 접어 섹션이 서지 않게 한다.
  const strengths = $derived(conclusion.strengths ?? []);

  const quoteOf = (strength: (typeof strengths)[number]) => {
    const quote = anchorQuote(content, [strength]);
    if (quote.length > 0) return { text: quote, linked: true };
    const fallback = strength.head === strength.tail ? strength.head : `${strength.head} ⋯ ${strength.tail}`;
    return { text: fallback, linked: false };
  };

  // 번호는 보이는 섹션끼리 잇달아 센다 — 빈 섹션이 빠져도 01부터 구멍 없이 흐른다.
  const sectionKeys = $derived(
    (
      [
        progressLines.length > 0 ? 'progress' : null,
        strengths.length > 0 ? 'strengths' : null,
        verdicts.length > 0 ? 'verdicts' : null,
        elevations.length > 0 ? 'elevations' : null,
        conclusion.patterns.length > 0 ? 'patterns' : null,
        conclusion.priorities.length > 0 ? 'priorities' : null,
      ] as const
    ).filter((key) => key !== null),
  );

  const numberFor = (key: (typeof sectionKeys)[number]) => String(sectionKeys.indexOf(key) + 1).padStart(2, '0');

  const onKeydown = (event: KeyboardEvent) => {
    if (open && event.key === 'Escape') onClose();
  };

  // 여닫이 모션 — 패널 폭(300)에서 자라나 틈만 남기고 화면을 덮는다. 열림 420ms·닫힘 300ms(내용 페이드
  // 120ms 선행), 이징은 화면 공통 곡선. 닫힘 상태는 visibility로 접근·클릭까지 끊는다.
  const rootRecipe = cva({
    base: { position: 'fixed', inset: '0', zIndex: 'modal', transition: '[visibility 0.3s]' },
    variants: {
      open: { true: {}, false: { visibility: 'hidden', pointerEvents: 'none' } },
    },
  });

  const scrimRecipe = cva({
    base: {
      position: 'absolute',
      inset: '0',
      backgroundColor: '[rgba(23, 24, 28, 0.35)]',
      cursor: 'pointer',
      transition: '[opacity 0.32s cubic-bezier(0.2, 0, 0, 1)]',
    },
    variants: {
      open: { true: { opacity: '100' }, false: { opacity: '0' } },
      instant: { true: { transition: '[none]' }, false: {} },
    },
  });

  const drawerRecipe = cva({
    base: {
      position: 'absolute',
      left: '0',
      top: '0',
      bottom: '0',
      display: 'flex',
      flexDirection: 'column',
      backgroundColor: 'surface.default',
      borderRightWidth: '1px',
      borderColor: 'border.default',
      boxShadow: '[8px 0 32px -8px rgba(9, 9, 12, 0.28)]',
      overflow: 'hidden',
    },
    variants: {
      open: {
        true: { width: '[calc(100% - 128px)]', opacity: '100', transition: '[width 0.42s cubic-bezier(0.2, 0, 0, 1), opacity 0.2s ease]' },
        false: { width: '300px', opacity: '0', transition: '[width 0.3s cubic-bezier(0.2, 0, 0, 1), opacity 0.18s ease 0.12s]' },
      },
      instant: { true: { transition: '[none]' }, false: {} },
    },
  });

  const letterRecipe = cva({
    base: {},
    variants: {
      open: {
        true: {
          opacity: '100',
          transform: '[none]',
          transition: '[opacity 0.28s cubic-bezier(0.2, 0, 0, 1) 0.16s, transform 0.28s cubic-bezier(0.2, 0, 0, 1) 0.16s]',
        },
        false: { opacity: '0', transform: '[translateY(12px)]', transition: '[opacity 0.12s ease]' },
      },
      instant: { true: { transition: '[none]' }, false: {} },
    },
  });

  const sectionHeadClass = css({
    display: 'flex',
    alignItems: 'baseline',
    gap: '12px',
    marginTop: '56px',
    paddingTop: '26px',
    borderTopWidth: '1px',
    borderColor: 'border.subtle',
  });
  const sectionNumberClass = css({ fontFamily: 'FiraCode', fontSize: '12px', fontWeight: 'semibold', color: 'text.brand' });
  const sectionTitleClass = css({ fontSize: '14px', fontWeight: 'bold', color: 'text.default' });
  const sectionCaptionClass = css({ fontSize: '11px', color: 'text.faint' });
  const proseClass = css({ fontFamily: 'RIDIBatang', fontSize: '16px', lineHeight: '[2.05]', color: 'text.default' });
  const noteClass = css({ marginTop: '6px', fontSize: '12px', lineHeight: '[1.75]', color: 'text.faint' });

  // 판정 눈금 — 네 칸 중 채운 만큼이 그 관점이 서 있는 자리다. 색으로 좋고 나쁨을 칠하지 않는다(먹색 채움뿐):
  // 낮은 자리에 경고색을 두면 편지가 성적표로 읽힌다. 정독 자리에만 세우고 좁은 패널은 낱말로만 간다.
  const stepRecipe = cva({
    base: { width: '16px', height: '3px', borderRadius: 'full' },
    variants: {
      filled: { true: { backgroundColor: 'text.default' }, false: { backgroundColor: 'border.default' } },
    },
  });
</script>

<svelte:window onkeydown={onKeydown} />

<div class={css(rootRecipe.raw({ open }))} aria-hidden={!open} inert={!open}>
  <button
    class={css(scrimRecipe.raw({ open, instant: reducedMotion.current }))}
    aria-label="총평 닫기"
    onclick={onClose}
    type="button"
  ></button>

  <button
    class={flex({
      position: 'absolute',
      right: '48px',
      top: '[50%]',
      transform: '[translateY(-50%)]',
      direction: 'column',
      align: 'center',
      gap: '7px',
      cursor: 'pointer',
    })}
    aria-label="총평 닫기"
    onclick={onClose}
    type="button"
  >
    <span
      class={flex({
        align: 'center',
        justify: 'center',
        size: '32px',
        borderRadius: 'full',
        backgroundColor: 'surface.default',
        color: 'text.subtle',
        boxShadow: 'medium',
      })}
    >
      <Icon icon={IconX} size={12} />
    </span>
    <span class={css({ fontSize: '10px', fontWeight: 'semibold', color: 'text.bright', letterSpacing: '[0.04em]' })}>ESC</span>
  </button>

  <section class={css(drawerRecipe.raw({ open, instant: reducedMotion.current }))} aria-label="편집자의 총평">
    <div
      class={flex({
        align: 'center',
        gap: '8px',
        flex: 'none',
        paddingX: '20px',
        paddingY: '14px',
        borderBottomWidth: '1px',
        borderColor: 'border.subtle',
      })}
    >
      <span class={css({ flex: 'none', fontSize: '11px', fontWeight: 'bold', color: 'text.brand', letterSpacing: '[0.09em]' })}>
        편집자의 총평
      </span>
      <span class={css({ fontSize: '11px', color: 'text.faint' })}>AI 리뷰 {roundNumber}회차 · 피드백 {conclusionThreads.length}개</span>
      <button
        class={flex({
          align: 'center',
          justify: 'center',
          flex: 'none',
          marginLeft: 'auto',
          size: '28px',
          borderWidth: '1px',
          borderColor: 'border.default',
          borderRadius: '6px',
          backgroundColor: 'surface.default',
          color: 'text.subtle',
          cursor: 'pointer',
          _hover: { borderColor: 'border.strong' },
        })}
        aria-label="총평 닫기"
        onclick={onClose}
        type="button"
      >
        <Icon icon={IconX} size={12} />
      </button>
    </div>

    <div class={css({ flexGrow: '1', minHeight: '0', overflowY: 'auto' })}>
      <div
        class={css(letterRecipe.raw({ open, instant: reducedMotion.current }), {
          maxWidth: '660px',
          marginX: 'auto',
          paddingX: '20px',
          paddingTop: '56px',
          paddingBottom: '64px',
        })}
      >
        <div class={flex({ align: 'center', gap: '10px', marginBottom: '34px' })}>
          <span
            class={flex({
              align: 'center',
              justify: 'center',
              flex: 'none',
              size: '34px',
              borderRadius: 'full',
              backgroundColor: 'accent.brand.subtle',
              fontSize: '12px',
              fontWeight: 'bold',
              color: 'text.brand',
            })}
          >
            AI
          </span>
          <div>
            <div class={css({ fontSize: '13px', fontWeight: 'bold', color: 'text.default' })}>AI 편집자</div>
            <div class={css({ marginTop: '1px', fontSize: '11px', color: 'text.faint' })}>
              {title}{dateLabel ? ` · ${dateLabel} 리뷰` : ''}
            </div>
          </div>
        </div>

        {#if understandingLines.length > 0}
          <div class={css({ marginBottom: '14px', fontSize: '14px', fontWeight: 'bold', color: 'text.default' })}>
            작품을 이렇게 읽었어요
          </div>
          <div class={flex({ direction: 'column', gap: '22px' })}>
            {#each understandingLines as line, index (index)}
              <p class={proseClass}>{line}</p>
            {/each}
          </div>
        {/if}

        {#if progressLines.length > 0}
          <div class={sectionHeadClass}>
            <span class={sectionNumberClass}>{numberFor('progress')}</span>
            <span class={sectionTitleClass}>지난 리뷰에서 나아진 점</span>
          </div>
          <div class={flex({ direction: 'column', gap: '22px', marginTop: '22px' })}>
            {#each progressLines as line, index (index)}
              <p class={proseClass}>{line}</p>
            {/each}
          </div>
        {/if}

        {#if strengths.length > 0}
          <div class={sectionHeadClass}>
            <span class={sectionNumberClass}>{numberFor('strengths')}</span>
            <span class={sectionTitleClass}>읽는 사람에게 잘 닿은 대목</span>
            <span class={sectionCaptionClass}>{strengths.length}곳 — 다음 원고에서도 믿고 쓰셔도 좋은 힘이에요</span>
          </div>
          <div class={flex({ direction: 'column', gap: '26px', marginTop: '22px' })}>
            {#each strengths as strength, index (index)}
              {@const quote = quoteOf(strength)}
              <div>
                <button
                  class={css({
                    width: 'full',
                    textAlign: 'left',
                    fontFamily: 'RIDIBatang',
                    fontSize: '15px',
                    lineHeight: '[1.85]',
                    color: 'text.default',
                    cursor: quote.linked ? 'pointer' : 'default',
                  })}
                  disabled={!quote.linked}
                  onclick={() => onActivate(`strength.${index}`)}
                  type="button"
                >
                  「{quote.text}」
                </button>
                {#if strength.body}
                  <Paragraphs
                    class={css({ marginTop: '8px', fontSize: '12px', lineHeight: '[1.75]', color: 'text.faint' })}
                    text={strength.body}
                  />
                {/if}
              </div>
            {/each}
          </div>
        {/if}

        {#if verdicts.length > 0}
          <div class={sectionHeadClass}>
            <span class={sectionNumberClass}>{numberFor('verdicts')}</span>
            <span class={sectionTitleClass}>관점마다 어디까지 왔는지</span>
            <span class={sectionCaptionClass}>{verdicts.length}가지 — 이 작품에서 특히 중요하게 본 관점들이에요</span>
          </div>
          <div class={flex({ direction: 'column', gap: '18px', marginTop: '22px' })}>
            {#each verdicts as verdict, index (index)}
              {@const level = verdictLabel(verdict.point)}
              <div>
                <div class={flex({ align: 'center', gap: '10px' })}>
                  <span class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.default' })}>{verdict.trait}</span>
                  <div class={flex({ align: 'center', gap: '3px' })} aria-hidden="true">
                    {#each VERDICT_POINTS as point (point)}
                      <span class={css(stepRecipe.raw({ filled: point <= verdict.point }))}></span>
                    {/each}
                  </div>
                  {#if level}
                    <span class={css({ fontSize: '11px', color: 'text.faint' })}>{level}</span>
                  {/if}
                </div>
                {#if verdict.note}
                  <Paragraphs class={noteClass} text={verdict.note} />
                {/if}
              </div>
            {/each}
          </div>
        {/if}

        {#if elevations.length > 0}
          <div class={sectionHeadClass}>
            <span class={sectionNumberClass}>{numberFor('elevations')}</span>
            <span class={sectionTitleClass}>한 걸음 더 가 볼 자리</span>
            <span class={sectionCaptionClass}>
              {elevations.length}곳 — 고칠 곳이 아니라, 이미 잘 되고 있는 곳에서 해 볼 수 있는 제안이에요
            </span>
          </div>
          <div class={flex({ direction: 'column', gap: '26px', marginTop: '22px' })}>
            {#each elevations as elevation, index (index)}
              {@const quote = anchorQuote(content, elevation.anchors)}
              <div>
                <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.default' })}>{elevation.trait}</div>
                {#if quote.length > 0}
                  <div
                    class={css({
                      marginTop: '8px',
                      fontFamily: 'RIDIBatang',
                      fontSize: '14px',
                      lineHeight: '[1.85]',
                      color: 'text.subtle',
                    })}
                  >
                    「{quote}」
                  </div>
                {/if}
                {#if elevation.body}
                  <Paragraphs class={noteClass} text={elevation.body} />
                {/if}
              </div>
            {/each}
          </div>
        {/if}

        {#if conclusion.patterns.length > 0}
          <div class={sectionHeadClass}>
            <span class={sectionNumberClass}>{numberFor('patterns')}</span>
            <span class={sectionTitleClass}>반복해서 나타나는 습관</span>
            <span class={sectionCaptionClass}>{conclusion.patterns.length}가지 — 하나를 고치면 여러 곳이 함께 풀려요</span>
          </div>
          <div class={flex({ direction: 'column', gap: '24px', marginTop: '22px' })}>
            {#each conclusion.patterns as pattern, index (index)}
              <div>
                {#if pattern.theme}
                  <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.default' })}>{pattern.theme}</div>
                {/if}
                <Paragraphs class={noteClass} text={pattern.body} />
                <IssueChips {activeId} issues={pattern.issues} {onActivate} threads={conclusionThreads} />
              </div>
            {/each}
          </div>
        {/if}

        {#if conclusion.priorities.length > 0}
          <div class={sectionHeadClass}>
            <span class={sectionNumberClass}>{numberFor('priorities')}</span>
            <span class={sectionTitleClass}>손보실 순서</span>
            <span class={sectionCaptionClass}>{conclusion.priorities.length}가지 — 먼저 고치면 뒤가 쉬워지는 순서예요</span>
          </div>
          <div class={flex({ direction: 'column', gap: '22px', marginTop: '22px' })}>
            {#each conclusion.priorities as priority, index (index)}
              <div class={flex({ gap: '12px' })}>
                <span
                  class={flex({
                    align: 'center',
                    justify: 'center',
                    flex: 'none',
                    size: '20px',
                    marginTop: '2px',
                    borderWidth: '1px',
                    borderColor: 'border.default',
                    borderRadius: 'full',
                    fontSize: '11px',
                    fontWeight: 'bold',
                    color: 'text.subtle',
                  })}
                >
                  {index + 1}
                </span>
                <div class={css({ flexGrow: '1', minWidth: '0' })}>
                  <Paragraphs class={css({ fontSize: '13px', lineHeight: '[1.75]', color: 'text.subtle' })} text={priority.body} />
                  <IssueChips {activeId} issues={priority.issues} {onActivate} threads={conclusionThreads} />
                </div>
              </div>
            {/each}
          </div>
        {/if}

        <div class={css({ marginTop: '56px', paddingTop: '30px', borderTopWidth: '1px', borderColor: 'border.subtle' })}>
          <div class={flex({ align: 'center', gap: '9px' })}>
            <span
              class={flex({
                align: 'center',
                justify: 'center',
                flex: 'none',
                size: '28px',
                borderRadius: 'full',
                backgroundColor: 'accent.brand.subtle',
                fontSize: '11px',
                fontWeight: 'bold',
                color: 'text.brand',
              })}
            >
              AI
            </span>
            <div>
              <div class={css({ fontSize: '12px', fontWeight: 'semibold', color: 'text.default' })}>AI 편집자</div>
              <div class={css({ fontSize: '11px', color: 'text.faint' })}>이 리뷰는 AI 편집자가 원고 전체를 읽고 작성했어요</div>
            </div>
          </div>
          <div class={css({ marginTop: '26px' })}>
            <ReviewReaction {reaction} variant="letter" />
          </div>
        </div>
      </div>
    </div>
  </section>
</div>
