<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { REVIEW_VERDICT_AXES } from '$lib/domain/verdicts.ts';
  import VerdictSwitch from './VerdictSwitch.svelte';
  import type { ReviewVerdict, WorkReview } from '$lib/domain/verdicts.ts';

  type Props = {
    review: WorkReview;
    verdict: ReviewVerdict;
    // 총평이 가리키는 피드백을 순번으로 되찾기 위한 목록(세트 표시 순서와 같아야 한다).
    feedbackLabels: { id: string; category: string | null }[];
    onUpdate: (verdict: ReviewVerdict) => void;
    onSelectFeedback: (feedbackId: string) => void;
    // 읽기 전용 열람에서는 판정 문항을 아예 걸지 않는다 — 저장되지 않는 입력을 보여주면
    // 답하라는 뜻으로 읽힌다.
    readOnly?: boolean;
  };
  const { review, verdict, feedbackLabels, onUpdate, onSelectFeedback, readOnly = false }: Props = $props();

  const bodyClass = css({ fontSize: '14px', lineHeight: '[1.85]', color: 'text.default', whiteSpace: 'pre-wrap' });
  // 제목은 제목처럼 쓴다. 작은 회색 라벨은 내용을 부속물처럼 보이게 만든다.
  const headingClass = css({ marginBottom: '8px', fontSize: '13px', fontWeight: 'bold', color: 'text.default' });
  const sectionClass = css({
    paddingY: '20px',
    borderTopWidth: '1px',
    borderColor: 'border.subtle',
    ['&:first-child']: { paddingTop: '0', borderTopWidth: '0' },
  });

  // 누를 수 있다는 것은 쉬고 있을 때도 보여야 하지만, 색을 쓸 자리는 아니다 —
  // 원고 옆에 적힌 참조는 밑줄 하나로 충분하고, 색이 들어가면 종이에서 뜬다.
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

  // 범위 밖 순번은 버린다 — 총평은 피드백이 확정되기 전 번호를 참조할 수 있다.
  const resolve = (indexes: number[]) =>
    indexes
      .filter((i) => i < feedbackLabels.length)
      .map((i) => ({ number: i + 1, id: feedbackLabels[i].id, category: feedbackLabels[i].category }));
</script>

{#snippet links(indexes: number[])}
  {@const targets = resolve(indexes)}
  {#if targets.length > 0}
    <p class={flex({ wrap: 'wrap', gap: '10px', marginTop: '8px' })}>
      {#each targets as target (target.id)}
        <button class={refClass} onclick={() => onSelectFeedback(target.id)} type="button">
          <span class={css({ fontWeight: 'bold', fontVariantNumeric: 'tabular-nums' })}>{target.number}</span>
          &nbsp;{target.category ?? '피드백'}
        </button>
      {/each}
    </p>
  {/if}
{/snippet}

<div class={css({ paddingX: '20px' })}>
  {#if review.characterization}
    <section class={sectionClass}>
      <h2 class={headingClass}>이 작품을 이렇게 읽었습니다</h2>
      <p class={bodyClass}>{review.characterization}</p>
    </section>
  {/if}

  {#if review.strengths.length > 0}
    <section class={sectionClass}>
      <h2 class={headingClass}>잘 되고 있는 것</h2>
      <div class={flex({ direction: 'column', gap: '16px' })}>
        {#each review.strengths as strength, i (i)}
          <div>
            <p class={bodyClass}>{strength.body}</p>
            <!-- 인용은 강점이 어느 대목인지 가리키는 유일한 단서다. 옛 실행에는 없어 조건부로 둔다. -->
            {#if strength.quoteStart}
              <p class={css({ marginTop: '4px', fontSize: '12px', color: 'text.subtle' })}>
                {strength.quoteStart}
                {#if strength.quoteEnd && strength.quoteEnd !== strength.quoteStart}
                  … {strength.quoteEnd}
                {/if}
              </p>
            {/if}
          </div>
        {/each}
      </div>
    </section>
  {/if}

  {#if review.cleared.length > 0}
    <section class={sectionClass}>
      <h2 class={headingClass}>살펴봤지만 문제가 없던 것</h2>
      <div class={flex({ direction: 'column', gap: '16px' })}>
        {#each review.cleared as item, i (i)}
          <div>
            <h3 class={css({ marginBottom: '2px', fontSize: '14px', fontWeight: 'bold', color: 'text.default' })}>
              {item.axis}
            </h3>
            <p class={bodyClass}>{item.note}</p>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  {#if review.patterns.length > 0}
    <section class={sectionClass}>
      <h2 class={headingClass}>되풀이되는 경향</h2>
      <div class={flex({ direction: 'column', gap: '16px' })}>
        {#each review.patterns as pattern, i (i)}
          <div>
            <h3 class={css({ marginBottom: '2px', fontSize: '14px', fontWeight: 'bold', color: 'text.default' })}>
              {pattern.theme}
            </h3>
            <p class={bodyClass}>{pattern.body}</p>
            {@render links(pattern.feedbackIndexes)}
          </div>
        {/each}
      </div>
    </section>
  {/if}

  {#if review.priority.length > 0}
    <section class={sectionClass}>
      <h2 class={headingClass}>먼저 손댈 것</h2>
      <!-- 우선순위는 실제로 순서가 정보다. 번호가 장식이 아니라 '이 차례로 하라'는 내용이다. -->
      <ol class={flex({ direction: 'column', gap: '16px' })}>
        {#each review.priority as item, i (i)}
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
              {@render links(item.feedbackIndexes)}
            </div>
          </li>
        {/each}
      </ol>
    </section>
  {/if}

  {#if !readOnly}
    <section class={sectionClass}>
      <h2 class={headingClass}>총평 판정</h2>
      <div class={flex({ direction: 'column', gap: '4px' })}>
        {#each REVIEW_VERDICT_AXES as axis (axis.key)}
          <VerdictSwitch
            negative={axis.negative}
            onChange={(value) => onUpdate({ ...verdict, [axis.key]: value })}
            question={axis.question}
            value={verdict[axis.key]}
          />
        {/each}

        {#if verdict.readCorrectly === false || verdict.priorityUseful === false}
          <input
            class={css({
              width: 'full',
              marginTop: '4px',
              borderBottomWidth: '1px',
              borderColor: 'border.default',
              paddingY: '4px',
              fontSize: '12px',
              backgroundColor: '[transparent]',
              _focus: { borderColor: 'border.strong' },
            })}
            oninput={(e) => onUpdate({ ...verdict, note: e.currentTarget.value })}
            placeholder="어디가 어떻게 어긋났는지 적어주세요"
            type="text"
            value={verdict.note ?? ''}
          />
        {/if}
      </div>
    </section>
  {/if}
</div>
