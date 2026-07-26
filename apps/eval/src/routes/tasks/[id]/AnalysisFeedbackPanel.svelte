<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { EMPTY_FEEDBACK_VERDICT, FEEDBACK_VERDICT_AXES, hasRejection } from '$lib/domain/verdicts.ts';
  import VerdictSwitch from './VerdictSwitch.svelte';
  import type { FeedbackVerdict, FeedbackVerdictMap } from '$lib/domain/verdicts.ts';

  // 피드백은 카드가 아니라 원고 여백에 적힌 메모다. 마흔 건에 상자를 하나씩 두르면
  // 상자끼리 경쟁해 눈이 갈 곳을 잃는다 — 매다는 번호와 여백만으로 구분한다.
  type Anchor = { matchStart: number | null; matchEnd: number | null };
  type Feedback = { id: string; category: string | null; polarity: string | null; body: string; anchors: Anchor[] };
  type Props = {
    feedbacks: Feedback[];
    verdicts: FeedbackVerdictMap;
    focusedId?: string | null;
    onUpdate: (feedbackId: string, verdict: FeedbackVerdict) => void;
    onHover: (feedbackId: string | null) => void;
    onSelect: (feedbackId: string, anchorIndex: number) => void;
    // 읽기 전용 열람에서는 판정 문항을 아예 걸지 않는다 — 저장되지 않는 입력을 보여주면
    // 답하라는 뜻으로 읽힌다.
    readOnly?: boolean;
  };
  const { feedbacks, verdicts, focusedId = null, onUpdate, onHover, onSelect, readOnly = false }: Props = $props();

  let anchorCursors = $state<Record<string, number>>({});

  const locatable = (feedback: Feedback) => feedback.anchors.filter((a) => a.matchStart !== null && a.matchEnd !== null).length;

  const step = (feedback: Feedback, delta: number) => {
    const count = locatable(feedback);
    if (count === 0) return;
    const next = ((anchorCursors[feedback.id] ?? 0) + delta + count) % count;
    anchorCursors = { ...anchorCursors, [feedback.id]: next };
    onSelect(feedback.id, next);
  };

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
</script>

<div>
  {#if feedbacks.length === 0}
    <p class={css({ paddingY: '32px', textAlign: 'center', fontSize: '14px', color: 'text.faint' })}>이 세트에는 피드백이 없습니다.</p>
  {/if}

  {#each feedbacks as feedback, i (feedback.id)}
    {@const verdict = verdicts[feedback.id] ?? EMPTY_FEEDBACK_VERDICT}
    {@const anchorCount = locatable(feedback)}
    {@const strength = feedback.polarity === 'highlight'}
    {@const focused = focusedId === feedback.id}
    <article
      class={css({
        display: 'grid',
        gridTemplateColumns: '[26px minmax(0, 1fr)]',
        columnGap: '10px',
        paddingX: '20px',
        paddingY: '18px',
        backgroundColor: focused ? 'surface.subtle' : '[transparent]',
        transition: '[background-color 0.15s ease]',
        ['& + &']: { borderTopWidth: '1px', borderColor: 'border.subtle' },
      })}
      data-feedback-card={feedback.id}
      onmouseenter={() => onHover(feedback.id)}
      onmouseleave={() => onHover(null)}
      role="presentation"
    >
      <!-- 번호는 본문 왼쪽에 매달린다. 아직 판정하지 않았다는 사실은 비어 있는 예/아니오가
           그대로 말하므로 따로 표시를 붙이지 않는다. -->
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
        {i + 1}
      </span>

      <div class={css({ minWidth: '0' })}>
        <div class={flex({ align: 'center', gap: '8px', marginBottom: '4px', minHeight: '22px' })}>
          {#if feedback.category}
            <span class={css({ fontSize: '12px', fontWeight: 'bold', color: 'text.subtle' })}>{feedback.category}</span>
          {/if}
          <!-- 강점이라는 사실은 한 곳에서만 말한다 — 이름까지 물들이면 같은 말을 두 번 하게 된다. -->
          {#if strength}
            <span class={css({ fontSize: '12px', color: 'text.success' })}>강점</span>
          {/if}
          <span class={flex({ align: 'center', marginLeft: 'auto', flexShrink: '0' })}>
            {#if anchorCount === 0}
              <span class={css({ fontSize: '12px', color: 'text.faint', paddingX: '6px' })}>본문 위치 없음</span>
            {:else if anchorCount === 1}
              <button class={linkClass} onclick={() => step(feedback, 0)} type="button">본문 보기</button>
            {:else}
              <button class={linkClass} aria-label="이전 위치" onclick={() => step(feedback, -1)} type="button">‹</button>
              <button class={linkClass} onclick={() => step(feedback, 1)} type="button">
                본문 {((anchorCursors[feedback.id] ?? 0) % anchorCount) + 1}/{anchorCount}
              </button>
              <button class={linkClass} aria-label="다음 위치" onclick={() => step(feedback, 1)} type="button">›</button>
            {/if}
          </span>
        </div>

        <p class={css({ fontSize: '14px', lineHeight: '[1.8]', color: 'text.default' })}>{feedback.body}</p>

        {#if !readOnly}
          <div class={flex({ direction: 'column', gap: '4px', marginTop: '10px' })}>
            {#each FEEDBACK_VERDICT_AXES as axis (axis.key)}
              <VerdictSwitch
                negative={axis.negative}
                onChange={(value) => onUpdate(feedback.id, { ...verdict, [axis.key]: value })}
                question={axis.question}
                value={verdict[axis.key]}
              />
            {/each}

            {#if hasRejection(verdicts[feedback.id])}
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
                oninput={(e) => onUpdate(feedback.id, { ...verdict, note: e.currentTarget.value })}
                placeholder="왜 아니라고 보셨나요"
                type="text"
                value={verdict.note ?? ''}
              />
            {/if}
          </div>
        {/if}
      </div>
    </article>
  {/each}
</div>
