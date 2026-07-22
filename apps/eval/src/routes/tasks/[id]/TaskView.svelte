<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { untrack } from 'svelte';
  import { enhance } from '$app/forms';
  import { FEEDBACK_LABEL_KEYS } from '$lib/domain/feedback-labels.ts';
  import { computeSegments } from '$lib/domain/highlight.ts';
  import FeedbackSetPanel from './FeedbackSetPanel.svelte';
  import type { FeedbackLabelEntry, FeedbackLabelMap } from '$lib/domain/feedback-labels.ts';
  import type { JudgmentResult, PairVerdict } from '$lib/domain/types.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  const startedAt = Date.now();
  const labels = ['A', 'B', 'C', 'D'];
  const SCORE_ANCHORS = [
    { score: 1, anchor: '매우 부실' },
    { score: 2, anchor: '부실' },
    { score: 3, anchor: '보통' },
    { score: 4, anchor: '좋음' },
    { score: 5, anchor: '훌륭' },
  ];

  const draftResult = untrack(() => data.draft?.result as JudgmentResult | null);

  // draft에서 복원할 때도 이 태스크의 setId·피드백 id만 신뢰한다 — 과거 버그로 다른 태스크의
  // 항목이 섞여 저장된 draft가 있어도 여기서 걸러진다.
  const taskFeedbackIds = untrack(() => new Set(data.sets.flatMap((s) => s.feedbacks.map((f) => f.id))));
  let labelMap = $state<FeedbackLabelMap>(
    untrack(() => {
      const draftLabels = (data.draft?.feedbackLabels as FeedbackLabelMap | undefined) ?? {};
      const filtered: FeedbackLabelMap = {};
      for (const [feedbackId, entry] of Object.entries(draftLabels)) {
        if (!taskFeedbackIds.has(feedbackId)) continue;
        const validLabels = entry.labels.filter((key) => FEEDBACK_LABEL_KEYS.has(key));
        if (validLabels.length === 0 && !entry.comment) continue;
        filtered[feedbackId] = entry.comment ? { labels: validLabels, comment: entry.comment } : { labels: validLabels };
      }
      return filtered;
    }),
  );
  let comment = $state(untrack(() => data.draft?.comment ?? ''));
  let hoveredFeedbackId = $state<string | null>(null);
  let focusedFeedbackId = $state<string | null>(null);
  let activeSetIndex = $state(0);
  let savedAt = $state<string | null>(null);
  let saving = $state(false);
  let submitting = $state(false);
  let confirmingRelease = $state(false);
  const busy = $derived(saving || submitting);
  let focusTimer: ReturnType<typeof setTimeout> | undefined;

  const outlineButtonClass = css({
    paddingX: '14px',
    paddingY: '9px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '8px',
    fontSize: '13px',
    color: 'text.subtle',
    cursor: 'pointer',
    transition: '[background-color 0.15s ease]',
    _disabled: { color: 'text.disabled', cursor: 'not-allowed' },
    ['&:hover:not(:disabled)']: { backgroundColor: 'surface.muted' },
  });

  let scores = $state<Record<string, number>>(
    untrack(() => {
      const draftScores = draftResult?.kind === 'scores' ? Object.fromEntries(draftResult.scores.map((s) => [s.setId, s.score])) : {};
      return Object.fromEntries(data.task.setIds.map((setId) => [setId, draftScores[setId] ?? 0]));
    }),
  );
  let verdict = $state<PairVerdict | null>(draftResult?.kind === 'pair' ? draftResult.verdict : null);

  const isRanking = $derived(data.task.kind === 'ranking');

  const result = $derived.by((): JudgmentResult | null => {
    if (isRanking) {
      if (Object.values(scores).includes(0)) return null;
      return { kind: 'scores', scores: Object.entries(scores).map(([setId, score]) => ({ setId, score })) };
    }
    return verdict ? { kind: 'pair', verdict } : null;
  });

  const activeSet = $derived(data.sets[activeSetIndex]);

  const segments = $derived(
    computeSegments(
      data.document.content,
      activeSet.feedbacks.reduce<{ start: number; end: number; feedbackId: string }[]>((anchors, f) => {
        if (f.matchStart !== null && f.matchEnd !== null) {
          anchors.push({ start: f.matchStart, end: f.matchEnd, feedbackId: f.id });
        }
        return anchors;
      }, []),
    ),
  );

  const feedbackNumbers = $derived<Record<string, number>>(Object.fromEntries(activeSet.feedbacks.map((f, i) => [f.id, i + 1])));

  const firstSegmentOf = $derived.by(() => {
    const seen: Record<string, number> = {};
    for (const [i, segment] of segments.entries()) {
      for (const fid of segment.feedbackIds) {
        seen[fid] ??= i;
      }
    }
    return seen;
  });

  const updateLabels = (feedbackId: string, entry: FeedbackLabelEntry | null) => {
    if (entry) {
      labelMap = { ...labelMap, [feedbackId]: entry };
      return;
    }
    labelMap = Object.fromEntries(Object.entries(labelMap).filter(([id]) => id !== feedbackId));
  };

  const scrollToPanelCard = (feedbackId: string) => {
    const el = document.querySelector(`[data-feedback-card="${feedbackId}"]`);
    if (!el) return;
    const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    el.scrollIntoView({ behavior: reduced ? 'auto' : 'smooth', block: 'center' });
    focusedFeedbackId = feedbackId;
    clearTimeout(focusTimer);
    focusTimer = setTimeout(() => (focusedFeedbackId = null), 2000);
  };

  const scrollToFeedback = (feedbackId: string) => {
    const el = document.querySelector(`[data-anchor="${feedbackId}"]`);
    if (!el) return;
    const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    el.scrollIntoView({ behavior: reduced ? 'auto' : 'smooth', block: 'center' });
    focusedFeedbackId = feedbackId;
    clearTimeout(focusTimer);
    focusTimer = setTimeout(() => (focusedFeedbackId = null), 2000);
  };

  const onKeydown = (e: KeyboardEvent) => {
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    const target = e.target as HTMLElement | null;
    if (target && ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)) return;
    const index = Number(e.key) - 1;
    if (index >= 0 && index < data.sets.length) {
      activeSetIndex = index;
    }
  };

  const readingMinutes = $derived(Math.max(1, Math.round(data.document.characterCount / 500)));
</script>

<svelte:window onkeydown={onKeydown} />

<div class={css({ height: '[100dvh]', display: 'flex', flexDirection: 'column', backgroundColor: 'surface.subtle' })}>
  <header
    class={flex({
      align: 'center',
      gap: '16px',
      height: '52px',
      paddingX: '20px',
      borderBottomWidth: '1px',
      borderColor: 'border.default',
      backgroundColor: 'surface.default',
      flexShrink: '0',
    })}
  >
    <a class={css({ fontSize: '13px', color: 'text.subtle', _hover: { color: 'text.default' } })} href="/">← 평가 큐</a>
    <div class={flex({ align: 'center', gap: '8px' })}>
      <span class={css({ fontSize: '13px', color: 'text.subtle' })}>
        내 판정 {data.progress.done}건 · 라운드 {data.progress.roundDone} / {data.progress.roundRequired}
      </span>
      <div class={css({ width: '120px', height: '4px', borderRadius: 'full', backgroundColor: 'surface.muted', overflow: 'hidden' })}>
        <div
          style:width={`${data.progress.roundRequired === 0 ? 0 : Math.round((data.progress.roundDone / data.progress.roundRequired) * 100)}%`}
          class={css({ height: 'full', backgroundColor: 'accent.brand.default' })}
        ></div>
      </div>
    </div>
    <span class={css({ marginLeft: 'auto', fontSize: '13px', color: 'text.faint' })}>
      {data.document.characterCount.toLocaleString()}자 · 약 {readingMinutes}분
      {#if saving}
        · 저장 중…
      {:else if savedAt}
        · 임시 저장됨 {savedAt}
      {/if}
    </span>
  </header>

  <div class={grid({ columns: 2, gap: '0', gridTemplateColumns: '[minmax(0, 1fr) 480px]', flex: '1', minHeight: '0' })}>
    <section class={css({ overflowY: 'auto', overflowAnchor: 'none', paddingY: '32px', paddingX: '24px' })}>
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
            {@const active = segment.feedbackIds.includes(hoveredFeedbackId ?? '') || segment.feedbackIds.includes(focusedFeedbackId ?? '')}
            <span
              class={css({
                position: 'relative',
                backgroundColor: active ? 'amber.300' : 'amber.100',
                borderBottomWidth: '2px',
                borderColor: 'amber.400',
                borderRadius: '2px',
                color: '[inherit]',
                cursor: 'pointer',
                transition: '[background-color 0.15s ease]',
              })}
              onclick={() => segment.feedbackIds[0] && scrollToPanelCard(segment.feedbackIds[0])}
              onkeydown={(e) => {
                if ((e.key === 'Enter' || e.key === ' ') && segment.feedbackIds[0]) {
                  e.preventDefault();
                  scrollToPanelCard(segment.feedbackIds[0]);
                }
              }}
              role="button"
              tabindex="0"
            >
              {#each segment.feedbackIds as fid, bi (fid)}
                {#if firstSegmentOf[fid] === i}
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
                    data-anchor={fid}
                    onclick={(e) => {
                      e.stopPropagation();
                      scrollToPanelCard(fid);
                    }}
                    onkeydown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        e.stopPropagation();
                        scrollToPanelCard(fid);
                      }
                    }}
                    role="button"
                    tabindex="0"
                  >
                    {feedbackNumbers[fid]}
                  </span>
                {/if}
              {/each}{segment.text}
            </span>
          {:else}
            <span>{segment.text}</span>
          {/if}
        {/each}
      </article>
    </section>

    <aside
      class={css({
        display: 'flex',
        flexDirection: 'column',
        minHeight: '0',
        borderLeftWidth: '1px',
        borderColor: 'border.default',
        backgroundColor: 'surface.default',
      })}
    >
      <nav class={css({ padding: '16px', borderBottomWidth: '1px', borderColor: 'border.subtle', flexShrink: '0' })}>
        <div class={grid({ columns: 4, gap: '6px' })}>
          {#each data.sets as set, i (`${i}-${set.setId}`)}
            <button
              class={css({
                paddingY: '8px',
                borderRadius: '8px',
                borderWidth: '1px',
                borderColor: activeSetIndex === i ? 'border.strong' : 'border.default',
                backgroundColor: activeSetIndex === i ? 'surface.dark' : 'surface.default',
                color: activeSetIndex === i ? 'text.bright' : 'text.default',
                fontSize: '14px',
                fontWeight: activeSetIndex === i ? 'bold' : 'normal',
                textAlign: 'center',
                cursor: 'pointer',
                transition: '[background-color 0.15s ease, border-color 0.15s ease, color 0.15s ease]',
              })}
              onclick={() => (activeSetIndex = i)}
              type="button"
            >
              {labels[i]}
              <span class={css({ fontSize: '11px', fontWeight: 'normal', opacity: '70' })}>{set.feedbacks.length}건</span>
            </button>
          {/each}
        </div>
        <p class={css({ marginTop: '8px', fontSize: '12px', color: 'text.faint' })}>
          키보드 1–{data.sets.length}로 세트 전환 · 피드백을 누르면 본문 위치로 이동합니다
        </p>
      </nav>

      <div class={css({ flex: '1', overflowY: 'auto', padding: '16px', minHeight: '0', backgroundColor: 'surface.subtle' })}>
        <FeedbackSetPanel
          feedbacks={activeSet.feedbacks}
          highlightedId={focusedFeedbackId}
          {labelMap}
          onHover={(id) => (hoveredFeedbackId = id)}
          onSelect={scrollToFeedback}
          onUpdateLabels={updateLabels}
        />
      </div>

      <form
        class={css({ padding: '16px', borderTopWidth: '1px', borderColor: 'border.default', flexShrink: '0' })}
        method="post"
        use:enhance={({ action, cancel }) => {
          if (busy) {
            cancel();
            return;
          }
          if (action.search.includes('save')) saving = true;
          else submitting = true;
          return async ({ update }) => {
            await update({ reset: false });
            saving = false;
            submitting = false;
            savedAt = new Date().toLocaleTimeString('ko', { hour: '2-digit', minute: '2-digit' });
          };
        }}
      >
        {#if isRanking}
          <fieldset class={flex({ direction: 'column', gap: '6px' })}>
            <legend class={css({ fontSize: '13px', fontWeight: 'bold', marginBottom: '6px' })}>
              점수
              <span class={css({ fontWeight: 'normal', color: 'text.faint' })}>(같은 평가 허용)</span>
            </legend>
            {#each data.task.setIds as setId, i (setId)}
              <div
                class={`${flex({ align: 'center', gap: '8px', paddingX: '6px', paddingY: '4px', borderRadius: '8px', transition: '[background-color 0.15s ease]' })} ${
                  activeSetIndex === i ? css({ backgroundColor: 'surface.muted' }) : ''
                }`}
              >
                <span
                  class={css({
                    width: '44px',
                    fontSize: '13px',
                    color: activeSetIndex === i ? 'text.default' : 'text.subtle',
                    fontWeight: activeSetIndex === i ? 'bold' : 'normal',
                  })}
                >
                  세트 {labels[i]}
                </span>
                <div class={grid({ columns: 5, gap: '4px', flex: '1' })}>
                  {#each SCORE_ANCHORS as { score, anchor } (score)}
                    <button
                      class={css({
                        paddingY: '6px',
                        borderRadius: '6px',
                        borderWidth: '1px',
                        borderColor: scores[setId] === score ? 'border.strong' : 'border.default',
                        backgroundColor: scores[setId] === score ? 'surface.dark' : 'surface.default',
                        color: scores[setId] === score ? 'text.bright' : 'text.subtle',
                        fontSize: '12px',
                        fontWeight: scores[setId] === score ? 'bold' : 'normal',
                        cursor: 'pointer',
                        transition: '[background-color 0.15s ease, border-color 0.15s ease, color 0.15s ease]',
                      })}
                      onclick={() => (scores[setId] = scores[setId] === score ? 0 : score)}
                      type="button"
                    >
                      {anchor}
                    </button>
                  {/each}
                </div>
              </div>
            {/each}
          </fieldset>
        {:else}
          <fieldset>
            <legend class={css({ fontSize: '13px', fontWeight: 'bold', marginBottom: '6px' })}>어느 세트의 피드백이 더 나은가요?</legend>
            <div class={grid({ columns: 3, gap: '6px' })}>
              {#each [{ value: 'a', label: 'A 우세' }, { value: 'tie', label: '무승부' }, { value: 'b', label: 'B 우세' }] as option (option.value)}
                <button
                  class={css({
                    paddingY: '10px',
                    borderRadius: '8px',
                    borderWidth: '1px',
                    borderColor: verdict === option.value ? 'border.strong' : 'border.default',
                    backgroundColor: verdict === option.value ? 'surface.dark' : 'surface.default',
                    color: verdict === option.value ? 'text.bright' : 'text.default',
                    fontSize: '14px',
                    fontWeight: verdict === option.value ? 'bold' : 'normal',
                    cursor: 'pointer',
                    transition: '[background-color 0.15s ease, border-color 0.15s ease, color 0.15s ease]',
                  })}
                  onclick={() => (verdict = option.value as PairVerdict)}
                  type="button"
                >
                  {option.label}
                </button>
              {/each}
            </div>
          </fieldset>
        {/if}

        <textarea
          name="comment"
          class={css({
            width: 'full',
            marginTop: '10px',
            borderWidth: '1px',
            borderColor: 'border.default',
            borderRadius: '8px',
            padding: '8px',
            fontSize: '13px',
            minHeight: '44px',
            backgroundColor: 'surface.default',
          })}
          placeholder="코멘트 (선택)"
          bind:value={comment}></textarea>

        <input name="result" type="hidden" value={result ? JSON.stringify(result) : ''} />
        <input name="feedbackLabels" type="hidden" value={JSON.stringify(labelMap)} />
        <input name="elapsedSeconds" type="hidden" value={Math.round((Date.now() - startedAt) / 1000)} />

        <div class={flex({ wrap: 'wrap', gap: '8px', marginTop: '10px', align: 'center' })}>
          <button class={outlineButtonClass} disabled={busy} formaction="?/save" type="submit">
            {saving ? '저장 중…' : '임시 저장'}
          </button>
          <button
            class={css({
              flex: '1',
              paddingY: '9px',
              borderRadius: '8px',
              backgroundColor: 'accent.brand.default',
              color: 'text.bright',
              fontSize: '13px',
              fontWeight: 'bold',
              cursor: 'pointer',
              transition: '[background-color 0.15s ease]',
              _disabled: { backgroundColor: 'interactive.disabled', cursor: 'not-allowed' },
              ['&:hover:not(:disabled)']: { backgroundColor: 'accent.brand.hover' },
            })}
            disabled={!result || busy}
            formaction="?/submit"
            type="submit"
          >
            {submitting ? '제출 중…' : '제출하고 다음으로'}
          </button>
          {#if data.draft}
            {#if confirmingRelease}
              <span class={css({ fontSize: '12px', color: 'text.faint', whiteSpace: 'nowrap' })}>입력한 내용이 사라집니다</span>
              <button class={outlineButtonClass} disabled={busy} formaction="?/release" type="submit">반납</button>
              <button class={outlineButtonClass} disabled={busy} onclick={() => (confirmingRelease = false)} type="button">취소</button>
            {:else}
              <button class={outlineButtonClass} disabled={busy} onclick={() => (confirmingRelease = true)} type="button">반납</button>
            {/if}
          {/if}
        </div>
        <p class={css({ marginTop: '6px', height: '16px', fontSize: '12px', color: result ? 'text.success' : 'text.faint' })}>
          {#if result}
            제출하면 다음 평가로 바로 이동합니다.
          {:else}
            {isRanking
              ? '모든 세트에 점수를 매기면 제출할 수 있습니다. 개별 피드백 평가는 선택입니다.'
              : '판정을 선택하면 제출할 수 있습니다.'}
          {/if}
        </p>
      </form>
    </aside>
  </div>
</div>
