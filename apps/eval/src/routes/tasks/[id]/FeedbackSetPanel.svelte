<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { SvelteSet } from 'svelte/reactivity';
  import IconCheck from '~icons/lucide/check';
  import IconChevronDown from '~icons/lucide/chevron-down';
  import IconChevronUp from '~icons/lucide/chevron-up';
  import IconMapPinOff from '~icons/lucide/map-pin-off';
  import { FEEDBACK_LABELS } from '$lib/domain/feedback-labels.ts';
  import type { FeedbackLabelEntry, FeedbackLabelMap } from '$lib/domain/feedback-labels.ts';

  type Feedback = { id: string; category: string | null; body: string; matchStart: number | null };
  type Props = {
    feedbacks: Feedback[];
    labelMap: FeedbackLabelMap;
    highlightedId?: string | null;
    onUpdateLabels: (feedbackId: string, entry: FeedbackLabelEntry | null) => void;
    onHover: (feedbackId: string | null) => void;
    onSelect: (feedbackId: string) => void;
  };
  const { feedbacks, labelMap, highlightedId = null, onUpdateLabels, onHover, onSelect }: Props = $props();

  const negativeLabels = FEEDBACK_LABELS.filter((label) => label.kind === 'negative');
  const positiveLabels = FEEDBACK_LABELS.filter((label) => label.kind === 'positive');
  const labelByKey = new Map(FEEDBACK_LABELS.map((label) => [label.key, label]));

  const expandedIds = new SvelteSet<string>();

  const toggleExpanded = (feedbackId: string) => {
    if (expandedIds.has(feedbackId)) expandedIds.delete(feedbackId);
    else expandedIds.add(feedbackId);
  };

  const commit = (feedbackId: string, entryLabels: string[], comment: string) => {
    if (entryLabels.length === 0 && !comment) {
      onUpdateLabels(feedbackId, null);
      return;
    }
    onUpdateLabels(feedbackId, comment ? { labels: entryLabels, comment } : { labels: entryLabels });
  };

  const toggleLabel = (feedbackId: string, key: string) => {
    const current = labelMap[feedbackId]?.labels ?? [];
    const next = current.includes(key) ? current.filter((k) => k !== key) : [...current, key];
    commit(feedbackId, next, labelMap[feedbackId]?.comment ?? '');
  };

  const updateComment = (feedbackId: string, comment: string) => {
    commit(feedbackId, labelMap[feedbackId]?.labels ?? [], comment);
  };

  const chipStyle = (selected: boolean) =>
    css({
      paddingX: '8px',
      paddingY: '4px',
      borderRadius: 'full',
      borderWidth: '1px',
      borderColor: selected ? 'border.strong' : 'border.default',
      backgroundColor: selected ? 'surface.dark' : 'surface.default',
      color: selected ? 'text.bright' : 'text.subtle',
      fontSize: '12px',
      cursor: 'pointer',
      transition: '[background-color 0.15s ease, border-color 0.15s ease, color 0.15s ease]',
    });
</script>

<div class={flex({ direction: 'column', gap: '10px' })}>
  {#if feedbacks.length === 0}
    <p class={css({ paddingY: '32px', textAlign: 'center', fontSize: '14px', color: 'text.faint' })}>이 세트에는 피드백이 없습니다.</p>
  {/if}

  {#each feedbacks as feedback, i (feedback.id)}
    {@const entry = labelMap[feedback.id]}
    {@const expanded = expandedIds.has(feedback.id)}
    <article
      class={css({
        borderWidth: '1px',
        borderColor: highlightedId === feedback.id ? 'border.strong' : 'border.default',
        borderRadius: '10px',
        padding: '14px',
        backgroundColor: highlightedId === feedback.id ? 'surface.subtle' : 'surface.default',
        cursor: feedback.matchStart === null ? 'default' : 'pointer',
        transition: '[border-color 0.15s ease, box-shadow 0.15s ease, background-color 0.15s ease]',
        _hover: { borderColor: 'border.strong', boxShadow: 'small' },
      })}
      data-feedback-card={feedback.id}
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
            <span class={flex({ align: 'center', gap: '3px', fontSize: '12px', color: 'text.faint' })}>
              <Icon icon={IconMapPinOff} size={12} />
              본문 위치 없음
            </span>
          {/if}
          <span class={css({ marginLeft: 'auto' })}>
            <button
              class={css({
                display: 'inline-flex',
                alignItems: 'center',
                gap: '4px',
                paddingX: '8px',
                paddingY: '3px',
                borderWidth: '1px',
                borderColor: entry ? 'border.strong' : 'border.default',
                borderRadius: '6px',
                backgroundColor: expanded ? 'surface.muted' : 'surface.default',
                color: entry ? 'text.default' : 'text.subtle',
                fontSize: '12px',
                cursor: 'pointer',
                transition: '[background-color 0.15s ease, border-color 0.15s ease, color 0.15s ease]',
              })}
              onclick={(e) => {
                e.stopPropagation();
                toggleExpanded(feedback.id);
              }}
              type="button"
            >
              {#if entry}
                <Icon style={css.raw({ color: 'text.success' })} icon={IconCheck} size={12} />
              {/if}
              이 피드백 평가
              <Icon icon={expanded ? IconChevronUp : IconChevronDown} size={12} />
            </button>
          </span>
        </div>
        <p class={css({ fontSize: '14px', lineHeight: '[1.7]', color: 'text.default' })}>{feedback.body}</p>

        {#if entry && entry.labels.length > 0}
          <div class={flex({ wrap: 'wrap', gap: '4px', marginTop: '8px' })}>
            {#each entry.labels as key (key)}
              {@const label = labelByKey.get(key)}
              {#if label}
                <span
                  class={css({
                    paddingX: '6px',
                    paddingY: '2px',
                    borderRadius: 'full',
                    fontSize: '11px',
                    backgroundColor: label.kind === 'negative' ? 'accent.danger.subtle' : 'accent.success.subtle',
                    color: label.kind === 'negative' ? 'text.danger' : 'text.success',
                  })}
                >
                  {label.name}
                </span>
              {/if}
            {/each}
          </div>
        {/if}

        {#if expanded}
          <div
            class={flex({
              direction: 'column',
              gap: '8px',
              marginTop: '10px',
              paddingTop: '10px',
              borderTopWidth: '1px',
              borderColor: 'border.subtle',
            })}
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.stopPropagation()}
            role="presentation"
          >
            <div class={flex({ direction: 'column', gap: '4px' })}>
              <span class={css({ fontSize: '11px', color: 'text.faint' })}>부정</span>
              <div class={flex({ wrap: 'wrap', gap: '6px' })}>
                {#each negativeLabels as label (label.key)}
                  <button
                    class={chipStyle((entry?.labels ?? []).includes(label.key))}
                    onclick={() => toggleLabel(feedback.id, label.key)}
                    type="button"
                  >
                    {label.name}
                  </button>
                {/each}
              </div>
            </div>
            <div class={flex({ direction: 'column', gap: '4px' })}>
              <span class={css({ fontSize: '11px', color: 'text.faint' })}>긍정</span>
              <div class={flex({ wrap: 'wrap', gap: '6px' })}>
                {#each positiveLabels as label (label.key)}
                  <button
                    class={chipStyle((entry?.labels ?? []).includes(label.key))}
                    onclick={() => toggleLabel(feedback.id, label.key)}
                    type="button"
                  >
                    {label.name}
                  </button>
                {/each}
              </div>
            </div>
            <div class={css({ borderTopWidth: '1px', borderColor: 'border.subtle', paddingTop: '10px' })}>
              <input
                class={css({
                  width: 'full',
                  borderWidth: '1px',
                  borderColor: 'border.default',
                  borderRadius: '6px',
                  paddingX: '8px',
                  paddingY: '6px',
                  fontSize: '12px',
                  backgroundColor: 'surface.default',
                })}
                oninput={(e) => updateComment(feedback.id, e.currentTarget.value)}
                placeholder={(entry?.labels ?? []).includes('etc') ? '어떤 문제인지 적어주세요' : '코멘트 (선택)'}
                type="text"
                value={entry?.comment ?? ''}
              />
            </div>
          </div>
        {/if}
      </div>
    </article>
  {/each}
</div>
