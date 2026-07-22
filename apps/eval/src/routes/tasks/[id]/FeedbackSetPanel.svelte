<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import type { SvelteSet } from 'svelte/reactivity';

  type Feedback = { id: string; category: string | null; body: string; matchStart: number | null };
  type Props = {
    feedbacks: Feedback[];
    flagged: SvelteSet<string>;
    onToggleFlag: (feedbackId: string) => void;
    onHover: (feedbackId: string | null) => void;
    onSelect: (feedbackId: string) => void;
  };
  const { feedbacks, flagged, onToggleFlag, onHover, onSelect }: Props = $props();
</script>

<div class={flex({ direction: 'column', gap: '10px' })}>
  {#if feedbacks.length === 0}
    <p class={css({ paddingY: '32px', textAlign: 'center', fontSize: '14px', color: 'text.faint' })}>이 세트에는 피드백이 없습니다.</p>
  {/if}

  {#each feedbacks as feedback, i (feedback.id)}
    <article
      class={css({
        borderWidth: '1px',
        borderColor: flagged.has(feedback.id) ? 'border.danger' : 'border.default',
        borderRadius: '10px',
        padding: '14px',
        backgroundColor: 'surface.default',
        cursor: feedback.matchStart === null ? 'default' : 'pointer',
        opacity: flagged.has(feedback.id) ? '55' : '100',
        transition: '[border-color 0.15s ease, box-shadow 0.15s ease, opacity 0.15s ease]',
        _hover: { borderColor: 'border.strong', boxShadow: 'small' },
      })}
      onmouseenter={() => onHover(feedback.id)}
      onmouseleave={() => onHover(null)}
    >
      <div
        onclick={() => feedback.matchStart !== null && onSelect(feedback.id)}
        onkeydown={(e) => e.key === 'Enter' && feedback.matchStart !== null && onSelect(feedback.id)}
        role="presentation"
      >
        <div class={flex({ align: 'center', gap: '8px', marginBottom: '8px' })}>
          <span
            class={css({
              display: 'inline-flex',
              alignItems: 'center',
              justifyContent: 'center',
              width: '20px',
              height: '20px',
              borderRadius: 'full',
              backgroundColor: 'surface.dark',
              color: 'text.bright',
              fontSize: '11px',
              fontWeight: 'bold',
              flexShrink: '0',
            })}
          >
            {i + 1}
          </span>
          {#if feedback.category}
            <span
              class={css({
                paddingX: '8px',
                paddingY: '2px',
                borderRadius: 'full',
                backgroundColor: 'surface.muted',
                fontSize: '12px',
                color: 'text.subtle',
              })}
            >
              {feedback.category}
            </span>
          {/if}
          {#if feedback.matchStart === null}
            <span class={css({ fontSize: '12px', color: 'text.faint' })}>본문 위치 없음</span>
          {/if}
          <span class={css({ marginLeft: 'auto' })}>
            <label
              class={flex({
                align: 'center',
                gap: '4px',
                fontSize: '12px',
                color: flagged.has(feedback.id) ? 'text.danger' : 'text.subtle',
                cursor: 'pointer',
              })}
            >
              <input
                class={css({
                  appearance: 'none',
                  width: '16px',
                  height: '16px',
                  borderWidth: '1px',
                  borderColor: 'border.strong',
                  borderRadius: '4px',
                  backgroundColor: 'surface.default',
                  cursor: 'pointer',
                  transition: '[background-color 0.15s ease, border-color 0.15s ease]',
                  _checked: { backgroundColor: 'accent.danger.default', borderColor: 'border.danger' },
                })}
                checked={flagged.has(feedback.id)}
                onchange={() => onToggleFlag(feedback.id)}
                onclick={(e) => e.stopPropagation()}
                type="checkbox"
              />
              오탐
            </label>
          </span>
        </div>
        <p class={css({ fontSize: '14px', lineHeight: '[1.7]', color: 'text.default' })}>{feedback.body}</p>
      </div>
    </article>
  {/each}
</div>
