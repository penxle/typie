<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { untrack } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import { enhance } from '$app/forms';
  import { computeSegments } from '$lib/domain/highlight.ts';
  import FeedbackSetPanel from './FeedbackSetPanel.svelte';
  import type { JudgmentResult, PairVerdict } from '$lib/domain/types.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  const startedAt = Date.now();
  const labels = ['A', 'B', 'C', 'D'];

  const draftResult = untrack(() => data.draft?.result as JudgmentResult | null);

  // draft에서 복원할 때도 이 태스크의 setId·피드백 id만 신뢰한다 — 과거 버그로 다른 태스크의
  // 항목이 섞여 저장된 draft가 있어도 여기서 걸러진다.
  const taskFeedbackIds = untrack(() => new Set(data.sets.flatMap((s) => s.feedbacks.map((f) => f.id))));
  const flagged = untrack(
    () =>
      new SvelteSet<string>(((data.draft?.falsePositiveFeedbackIds as string[] | undefined) ?? []).filter((id) => taskFeedbackIds.has(id))),
  );
  let comment = $state(untrack(() => data.draft?.comment ?? ''));
  let hoveredFeedbackId = $state<string | null>(null);
  let focusedFeedbackId = $state<string | null>(null);
  let activeSetIndex = $state(0);
  let savedAt = $state<string | null>(null);
  let saving = $state(false);
  let submitting = $state(false);
  const busy = $derived(saving || submitting);
  let focusTimer: ReturnType<typeof setTimeout> | undefined;

  let ranks = $state<Record<string, number>>(
    untrack(() => {
      const draftRanks = draftResult?.kind === 'ranking' ? Object.fromEntries(draftResult.ranks.map((r) => [r.setId, r.rank])) : {};
      return Object.fromEntries(data.task.setIds.map((setId) => [setId, draftRanks[setId] ?? 0]));
    }),
  );
  let verdict = $state<PairVerdict | null>(draftResult?.kind === 'pair' ? draftResult.verdict : null);

  const isRanking = $derived(data.task.kind === 'ranking');

  const result = $derived.by((): JudgmentResult | null => {
    if (isRanking) {
      if (Object.values(ranks).includes(0)) return null;
      return { kind: 'ranking', ranks: Object.entries(ranks).map(([setId, rank]) => ({ setId, rank })) };
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

  const toggleFlag = (feedbackId: string) => {
    if (flagged.has(feedbackId)) flagged.delete(feedbackId);
    else flagged.add(feedbackId);
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
    <section class={css({ overflowY: 'auto', paddingY: '32px', paddingX: '24px' })}>
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
            <mark
              class={css({
                backgroundColor: active ? 'amber.300' : 'amber.100',
                borderBottomWidth: '2px',
                borderColor: 'amber.400',
                borderRadius: '2px',
                color: '[inherit]',
                transition: '[background-color 0.15s ease]',
              })}
            >
              {#each segment.feedbackIds as fid (fid)}
                {#if firstSegmentOf[fid] === i}
                  <sup
                    class={css({
                      display: 'inline-flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      width: '16px',
                      height: '16px',
                      marginRight: '3px',
                      borderRadius: 'full',
                      backgroundColor: 'surface.dark',
                      color: 'text.bright',
                      fontSize: '10px',
                      fontWeight: 'bold',
                      lineHeight: '[1]',
                      verticalAlign: '[super]',
                      cursor: 'default',
                      userSelect: 'none',
                    })}
                    data-anchor={fid}
                  >
                    {feedbackNumbers[fid]}
                  </sup>
                {/if}
              {/each}{segment.text}
            </mark>
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
          {flagged}
          onHover={(id) => (hoveredFeedbackId = id)}
          onSelect={scrollToFeedback}
          onToggleFlag={toggleFlag}
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
              순위
              <span class={css({ fontWeight: 'normal', color: 'text.faint' })}>(1 = 최고, 동률 허용)</span>
            </legend>
            {#each data.task.setIds as setId, i (setId)}
              <div class={flex({ align: 'center', gap: '8px' })}>
                <span class={css({ width: '44px', fontSize: '13px', color: 'text.subtle' })}>세트 {labels[i]}</span>
                <div class={grid({ columns: 4, gap: '4px', flex: '1' })}>
                  {#each data.task.setIds as rankSetId, rank (rankSetId)}
                    <button
                      class={css({
                        paddingY: '6px',
                        borderRadius: '6px',
                        borderWidth: '1px',
                        borderColor: ranks[setId] === rank + 1 ? 'border.strong' : 'border.default',
                        backgroundColor: ranks[setId] === rank + 1 ? 'surface.dark' : 'surface.default',
                        color: ranks[setId] === rank + 1 ? 'text.bright' : 'text.subtle',
                        fontSize: '13px',
                        fontWeight: ranks[setId] === rank + 1 ? 'bold' : 'normal',
                        cursor: 'pointer',
                        transition: '[background-color 0.15s ease, border-color 0.15s ease, color 0.15s ease]',
                      })}
                      onclick={() => (ranks[setId] = ranks[setId] === rank + 1 ? 0 : rank + 1)}
                      type="button"
                    >
                      {rank + 1}
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
        <input name="falsePositiveFeedbackIds" type="hidden" value={JSON.stringify([...flagged])} />
        <input name="elapsedSeconds" type="hidden" value={Math.round((Date.now() - startedAt) / 1000)} />

        <div class={flex({ gap: '8px', marginTop: '10px', align: 'center' })}>
          <button
            class={css({
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
            })}
            disabled={busy}
            formaction="?/save"
            type="submit"
          >
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
        </div>
        <p class={css({ marginTop: '6px', height: '16px', fontSize: '12px', color: result ? 'text.success' : 'text.faint' })}>
          {#if result}
            제출하면 다음 평가로 바로 이동합니다.
          {:else}
            {isRanking ? '모든 세트에 순위를 지정하면 제출할 수 있습니다.' : '판정을 선택하면 제출할 수 있습니다.'}
          {/if}
        </p>
      </form>
    </aside>
  </div>
</div>
