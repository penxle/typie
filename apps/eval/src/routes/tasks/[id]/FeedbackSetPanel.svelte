<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { SvelteSet } from 'svelte/reactivity';
  import IconCheck from '~icons/lucide/check';
  import IconChevronDown from '~icons/lucide/chevron-down';
  import IconChevronUp from '~icons/lucide/chevron-up';
  import IconMapPinOff from '~icons/lucide/map-pin-off';
  import { ALL_FEEDBACK_LABELS, FEEDBACK_LABELS } from '$lib/domain/feedback-labels.ts';
  import type { FeedbackLabelEntry, FeedbackLabelMap } from '$lib/domain/feedback-labels.ts';

  type Feedback = { id: string; category: string | null; layer?: string | null; body: string; matchStart: number | null };
  type Props = {
    feedbacks: Feedback[];
    labelMap: FeedbackLabelMap;
    highlightedId?: string | null;
    onUpdateLabels: (feedbackId: string, entry: FeedbackLabelEntry | null) => void;
    onHover: (feedbackId: string | null) => void;
    onSelect: (feedbackId: string) => void;
  };
  const { feedbacks, labelMap, highlightedId = null, onUpdateLabels, onHover, onSelect }: Props = $props();

  const labelGroups = [...new Set(FEEDBACK_LABELS.map((label) => label.group))].map((group) => ({
    group,
    labels: FEEDBACK_LABELS.filter((label) => label.group === group),
  }));
  const labelByKey = new Map(ALL_FEEDBACK_LABELS.map((label) => [label.key, label]));

  const expandedIds = new SvelteSet<string>();

  // 층위 분리 표시 — 작품 검토(계획 축)와 문면 교열(문장 결·원고 사고)은 작가가 받아들일
  // 무게가 다르다. layer가 없는 구 실행 행은 category로 보정한다.
  const LOCAL_CATEGORIES = new Set(['문장 결', '원고 사고']);
  const isLocal = (f: Feedback) => f.layer === 'local' || (!f.layer && f.category !== null && LOCAL_CATEGORIES.has(f.category));
  const numbers = $derived(new Map(feedbacks.map((f, idx) => [f.id, idx + 1])));
  const groups = $derived(
    [
      {
        key: 'plan',
        title: '작품 검토',
        desc: '이 원고의 위험을 겨냥한 검토 관점에서 나온 지적',
        items: feedbacks.filter((f) => !isLocal(f)),
      },
      { key: 'local', title: '문면 교열', desc: '문장 층위에서 대조로 확인된 지적', items: feedbacks.filter((f) => isLocal(f)) },
    ].filter((g) => g.items.length > 0),
  );
  const showHeaders = $derived(groups.length > 1);

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

  {#each groups as group (group.key)}
    {#if showHeaders}
      <div class={flex({ align: 'baseline', gap: '8px', marginTop: group.key === 'local' ? '10px' : '0' })}>
        <h3 class={css({ fontSize: '13px', fontWeight: 'bold', color: 'text.subtle' })}>{group.title}</h3>
        <span class={css({ fontSize: '12px', color: 'text.faint' })}>{group.desc}</span>
      </div>
    {/if}
    {#each group.items as feedback (feedback.id)}
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
              {numbers.get(feedback.id)}
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
                      backgroundColor:
                        label.kind === 'negative'
                          ? 'accent.danger.subtle'
                          : label.kind === 'system'
                            ? 'accent.warning.subtle'
                            : 'accent.success.subtle',
                      color:
                        label.kind === 'negative' ? 'text.danger' : label.kind === 'system' ? 'accent.warning.default' : 'text.success',
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
              {#each labelGroups as { group, labels } (group)}
                <div class={flex({ direction: 'column', gap: '4px' })}>
                  <span class={css({ fontSize: '11px', color: 'text.faint' })}>{group}</span>
                  <div class={flex({ wrap: 'wrap', gap: '6px' })}>
                    {#each labels as label (label.key)}
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
              {/each}
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
  {/each}
</div>
