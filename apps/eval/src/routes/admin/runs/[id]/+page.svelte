<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { untrack } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import { invalidateAll } from '$app/navigation';
  import FeedbackSetPanel from '../../../tasks/[id]/FeedbackSetPanel.svelte';
  import { usePolling } from '../../lib/poll.svelte.ts';
  import {
    etaSeconds,
    formatDuration,
    formatProgressSummary,
    KIND_LABELS,
    primaryMetric,
    primaryTotal,
    progressRatio,
    throughputPerMinute,
  } from '../progress.ts';
  import RunStatusBadge from '../RunStatusBadge.svelte';
  import ConfirmCancelDialog from './ConfirmCancelDialog.svelte';
  import type { RunDocStatus } from '$lib/domain/admin-types.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  type RunFetchResponse = { run: typeof data.run; docs: typeof data.docs };

  // data.run/data.docs는 폴링으로 갱신되므로 로컬 $state로 들고, summary/preview는 완료 시점에만 필요하고
  // invalidateAll()로 재계산되므로 data 프롭을 그대로 읽는다(별도 $state 불필요).
  let run = $state(untrack(() => data.run));
  let docs = $state(untrack(() => data.docs));

  let showCancelDialog = $state(false);
  let cancelling = $state(false);
  let cancelError = $state<string | null>(null);
  let retrying = $state(false);
  let retryError = $state<string | null>(null);
  let selectedFailedDocId = $state<string | null>(null);

  // 세션(페이지 진입) 시작 시점 샘플 — 처리율·ETA를 이 시점 대비 실측한다.
  const sessionStart = untrack(() => ({ at: Date.now(), done: primaryMetric(data.run) }));

  // 프리뷰 피드백의 "오탐" 체크박스는 이 화면에서 저장되지 않는 로컬 표시일 뿐이다(FeedbackSetPanel 재사용).
  // feedback id는 전역 유일(nanoid)하므로 프리뷰 문서 전체에 걸쳐 하나의 집합만 있어도 안전하다.
  const previewFlagged = untrack(() => new SvelteSet<string>());
  const togglePreviewFlag = (feedbackId: string) => {
    if (previewFlagged.has(feedbackId)) previewFlagged.delete(feedbackId);
    else previewFlagged.add(feedbackId);
  };
  // eslint-disable-next-line @typescript-eslint/no-empty-function -- 프리뷰는 원문 하이라이트가 없어 hover/select가 할 일이 없다
  const previewNoop = () => {};

  const pollRun = async () => {
    const response = await fetch(`/admin/api/runs/${run.id}`);
    if (!response.ok) return;
    const fresh = (await response.json()) as RunFetchResponse;

    const wasRunning = run.status === 'running';
    run = fresh.run;
    docs = fresh.docs;

    if (wasRunning && run.status !== 'running') {
      // 방금 종료됨 — 기계 지표 요약·프리뷰를 서버에서 다시 계산해오도록 전체 load를 한 번 재실행한다.
      await invalidateAll();
    }
  };

  usePolling(pollRun, 3000, { enabled: () => run.status === 'running' });

  const current = $derived({ at: Date.now(), done: primaryMetric(run) });
  const elapsedSeconds = $derived((current.at - new Date(run.createdAt).getTime()) / 1000);
  const rate = $derived(throughputPerMinute(sessionStart, current));
  const eta = $derived(run.status === 'running' ? etaSeconds(sessionStart, current, primaryTotal(run)) : null);
  const rateUnit = $derived(run.kind === 'pipeline' ? '청크' : '문서');

  const retryableCount = $derived(docs.filter((d) => d.status === 'failed' || d.status === 'cancelled').length);
  const selectedFailedDoc = $derived(docs.find((d) => d.id === selectedFailedDocId && d.status === 'failed') ?? null);

  const STATUS_LABEL: Record<RunDocStatus, string> = {
    pending: '대기',
    running: '실행 중',
    done: '완료',
    failed: '실패',
    cancelled: '취소됨',
  };

  const openCancelDialog = () => {
    cancelError = null;
    showCancelDialog = true;
  };

  const confirmCancel = async () => {
    cancelling = true;
    cancelError = null;
    try {
      const response = await fetch(`/admin/api/runs/${run.id}/cancel`, { method: 'POST' });
      if (!response.ok) {
        cancelError = `취소에 실패했습니다 (${response.status}).`;
        return;
      }
      showCancelDialog = false;
      await pollRun();
    } finally {
      cancelling = false;
    }
  };

  const retryFailed = async () => {
    retrying = true;
    retryError = null;
    try {
      const response = await fetch(`/admin/api/runs/${run.id}/retry-failed`, { method: 'POST' });
      if (!response.ok) {
        retryError = `재실행에 실패했습니다 (${response.status}).`;
        return;
      }
      await pollRun();
    } finally {
      retrying = false;
    }
  };

  const percent = (value: number) => (Number.isNaN(value) ? '—' : `${(value * 100).toFixed(1)}%`);

  const statCardClass = css({ backgroundColor: 'surface.subtle', borderRadius: '10px', padding: '12px' });
  const sectionClass = css({
    backgroundColor: 'surface.default',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '12px',
    padding: '20px',
    boxShadow: 'small',
    marginBottom: '16px',
  });
  const outlineButtonClass = css({
    paddingX: '14px',
    paddingY: '9px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '8px',
    fontSize: '13px',
    fontWeight: 'bold',
    color: 'text.default',
    cursor: 'pointer',
    transition: '[background-color 0.15s ease]',
    _disabled: { color: 'text.disabled', cursor: 'not-allowed' },
    ['&:hover:not(:disabled)']: { backgroundColor: 'surface.muted' },
  });
</script>

<div class={css({ maxWidth: '960px', marginX: 'auto', paddingY: '40px', paddingX: '32px' })}>
  <a class={css({ fontSize: '13px', color: 'text.subtle', _hover: { color: 'text.default' } })} href="/admin/runs">← 실행 목록</a>

  <header class={flex({ align: 'center', gap: '10px', marginTop: '8px', marginBottom: '20px' })}>
    <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>{KIND_LABELS[run.kind]} 실행</h1>
    <RunStatusBadge status={run.status} />
    <span class={css({ fontSize: '13px', color: 'text.faint' })}>{data.variantLabel ?? '—'} · {run.corpusVersion}</span>
  </header>

  {#if run.status === 'cancelled'}
    <div
      class={css({
        marginBottom: '16px',
        paddingX: '16px',
        paddingY: '12px',
        borderRadius: '10px',
        backgroundColor: 'surface.muted',
        fontSize: '13px',
        color: 'text.subtle',
      })}
    >
      부분 결과가 보존되었습니다 · 재실행 시 완료된 문서는 건너뜁니다.
    </div>
  {/if}

  {#if run.error}
    <div
      class={css({
        marginBottom: '16px',
        paddingX: '16px',
        paddingY: '12px',
        borderRadius: '10px',
        backgroundColor: 'accent.danger.subtle',
        fontSize: '13px',
        color: 'text.danger',
      })}
    >
      {run.error}
    </div>
  {/if}

  <section class={sectionClass}>
    <div class={flex({ align: 'center', justify: 'space-between', marginBottom: '6px' })}>
      <span class={css({ fontSize: '13px', fontWeight: 'bold' })}>{formatProgressSummary(run)}</span>
      <span class={css({ fontSize: '12px', color: 'text.faint' })}>{(progressRatio(run) * 100).toFixed(0)}%</span>
    </div>
    <div class={css({ height: '8px', borderRadius: 'full', backgroundColor: 'surface.muted', overflow: 'hidden' })}>
      <div
        style:width={`${progressRatio(run) * 100}%`}
        class={css({ height: 'full', backgroundColor: 'accent.brand.default', transition: '[width 0.15s ease]' })}
      ></div>
    </div>

    <div class={grid({ columns: 4, gap: '10px', marginTop: '16px' })}>
      <div class={statCardClass}>
        <p class={css({ fontSize: '12px', color: 'text.faint' })}>경과</p>
        <p class={css({ marginTop: '2px', fontSize: '16px', fontWeight: 'bold' })}>{formatDuration(elapsedSeconds)}</p>
      </div>
      <div class={statCardClass}>
        <p class={css({ fontSize: '12px', color: 'text.faint' })}>처리율</p>
        <p class={css({ marginTop: '2px', fontSize: '16px', fontWeight: 'bold' })}>
          {rate === null ? '계산 중…' : `${rate.toFixed(1)}${rateUnit}/분`}
        </p>
      </div>
      <div class={statCardClass}>
        <p class={css({ fontSize: '12px', color: 'text.faint' })}>ETA</p>
        <p class={css({ marginTop: '2px', fontSize: '16px', fontWeight: 'bold' })}>
          {run.status === 'running' ? (eta === null ? '계산 중…' : formatDuration(eta)) : '—'}
        </p>
      </div>
      <div class={statCardClass}>
        <p class={css({ fontSize: '12px', color: 'text.faint' })}>누적 토큰</p>
        <p class={css({ marginTop: '2px', fontSize: '16px', fontWeight: 'bold' })}>
          {(run.promptTokens + run.completionTokens).toLocaleString()}
        </p>
      </div>
    </div>

    <div class={flex({ gap: '8px', marginTop: '16px', align: 'center' })}>
      {#if run.status === 'running'}
        <button class={outlineButtonClass} onclick={openCancelDialog} type="button">실행 취소</button>
      {/if}
      {#if run.kind === 'pipeline' && retryableCount > 0}
        <button class={outlineButtonClass} disabled={retrying} onclick={retryFailed} type="button">
          {retrying ? '재실행하는 중…' : `미완료 문서만 재실행 (${retryableCount})`}
        </button>
      {/if}
    </div>
    <p class={css({ marginTop: '8px', height: '16px', fontSize: '12px', color: 'text.danger' })}>{cancelError ?? retryError ?? ''}</p>
  </section>

  {#if run.kind === 'pipeline'}
    <section class={sectionClass}>
      <div class={flex({ align: 'center', justify: 'space-between', marginBottom: '12px' })}>
        <h2 class={css({ fontSize: '13px', fontWeight: 'bold', color: 'text.subtle' })}>문서 상태 ({docs.length})</h2>
        <div class={flex({ gap: '10px', fontSize: '11px', color: 'text.faint' })}>
          <span>● 대기</span>
          <span class={css({ color: 'accent.brand.default' })}>● 실행 중</span>
          <span class={css({ color: 'accent.success.default' })}>● 완료</span>
          <span class={css({ color: 'accent.danger.default' })}>● 실패</span>
        </div>
      </div>

      <div class={grid({ columns: 10, gap: '6px' })}>
        {#each docs as doc (doc.id)}
          <button
            class={css({
              aspectRatio: '[1]',
              borderWidth: '0',
              borderRadius: '6px',
              backgroundColor:
                doc.status === 'running'
                  ? 'accent.brand.default'
                  : doc.status === 'done'
                    ? 'accent.success.default'
                    : doc.status === 'failed'
                      ? 'accent.danger.default'
                      : 'surface.muted',
              boxShadow: selectedFailedDocId === doc.id ? 'medium' : '[none]',
              cursor: doc.status === 'failed' ? 'pointer' : 'default',
              animation: doc.status === 'running' ? 'pulse 1.5s ease-in-out infinite' : 'none',
              transition: '[opacity 0.15s ease, box-shadow 0.15s ease]',
              ['&:hover:not(:disabled)']: doc.status === 'failed' ? { opacity: '80' } : {},
            })}
            disabled={doc.status !== 'failed'}
            onclick={() => (selectedFailedDocId = doc.id)}
            title={`${doc.documentId} · ${STATUS_LABEL[doc.status]}`}
            type="button"
          ></button>
        {/each}
      </div>

      <div
        class={css({
          marginTop: '12px',
          minHeight: '60px',
          padding: '12px',
          borderRadius: '8px',
          backgroundColor: 'surface.subtle',
          fontSize: '13px',
        })}
      >
        {#if selectedFailedDoc}
          <p class={css({ fontWeight: 'bold', marginBottom: '4px' })}>문서 {selectedFailedDoc.documentId}</p>
          <p class={css({ color: 'text.danger' })}>{selectedFailedDoc.error ?? '알 수 없는 오류'}</p>
        {:else}
          <p class={css({ color: 'text.faint' })}>실패한 문서를 클릭하면 오류 메시지가 여기에 표시됩니다.</p>
        {/if}
      </div>
    </section>
  {/if}

  {#if data.summary}
    <section class={sectionClass}>
      <h2 class={css({ fontSize: '13px', fontWeight: 'bold', color: 'text.subtle', marginBottom: '12px' })}>기계 지표 요약</h2>
      <div class={grid({ columns: 3, gap: '10px' })}>
        <div class={statCardClass}>
          <p class={css({ fontSize: '12px', color: 'text.faint' })}>앵커 매칭률</p>
          <p class={css({ marginTop: '2px', fontSize: '18px', fontWeight: 'bold' })}>{percent(data.summary.anchorMatchRate)}</p>
        </div>
        <div class={statCardClass}>
          <p class={css({ fontSize: '12px', color: 'text.faint' })}>건수 분포 (0건 / 10건 초과)</p>
          <p class={css({ marginTop: '2px', fontSize: '18px', fontWeight: 'bold' })}>
            {data.summary.feedbackDistribution.zero} / {data.summary.feedbackDistribution.over10}
            <span class={css({ fontSize: '12px', fontWeight: 'normal', color: 'text.faint' })}>
              (총 {data.summary.feedbackDistribution.total})
            </span>
          </p>
        </div>
        <div class={statCardClass}>
          <p class={css({ fontSize: '12px', color: 'text.faint' })}>카테고리 준수율</p>
          <p class={css({ marginTop: '2px', fontSize: '18px', fontWeight: 'bold' })}>{percent(data.summary.categoryCompliance)}</p>
        </div>
      </div>
    </section>

    {#if data.preview.length > 0}
      <section
        class={css({
          backgroundColor: 'surface.default',
          borderWidth: '1px',
          borderColor: 'border.default',
          borderRadius: '12px',
          padding: '20px',
          boxShadow: 'small',
        })}
      >
        <h2 class={css({ fontSize: '13px', fontWeight: 'bold', color: 'text.subtle', marginBottom: '12px' })}>피드백 프리뷰</h2>
        <div class={flex({ direction: 'column', gap: '20px' })}>
          {#each data.preview as doc (doc.documentId)}
            <div>
              <p class={css({ fontSize: '13px', fontWeight: 'bold', marginBottom: '8px' })}>{doc.refId}</p>
              <FeedbackSetPanel
                feedbacks={doc.feedbacks}
                flagged={previewFlagged}
                onHover={previewNoop}
                onSelect={previewNoop}
                onToggleFlag={togglePreviewFlag}
              />
            </div>
          {/each}
        </div>
      </section>
    {/if}
  {/if}
</div>

{#if showCancelDialog}
  <ConfirmCancelDialog error={cancelError} onCancel={() => (showCancelDialog = false)} onConfirm={confirmCancel} pending={cancelling} />
{/if}
