<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Helmet } from '@typie/ui/components';
  import { Dialog, Toast } from '@typie/ui/notification';
  import { untrack } from 'svelte';
  import { invalidateAll } from '$app/navigation';
  import { formatKrw } from '$lib/domain/pricing.ts';
  import FeedbackSetPanel from '../../../tasks/[id]/FeedbackSetPanel.svelte';
  import CostCell from '../../lib/CostCell.svelte';
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
  import type { RunDocStatus } from '$lib/domain/admin-types.ts';
  import type { FeedbackLabelEntry, FeedbackLabelMap } from '$lib/domain/feedback-labels.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  const STAGE_LABELS: Record<string, string> = {
    survey: '작품 파악',
    background: '원작 조사',
    review: '짚을 곳 찾기',
    dedupe: '중복 묶기',
    verify: '검증',
    research: '작품 조사',
    'plan-draft': '계획 초안',
    'plan-revise-0': '계획 수정 1',
    'plan-revise-1': '계획 수정 2',
    'plan-revise-2': '계획 수정 3',
    plan: '계획 검수',
    planReview: '계획 검수',
    execute: '작품 검토',
    local: '문면 교열',
    compose: '피드백 쓰기',
    composeReview: '총평 쓰기',
  };

  type RunFetchResponse = { run: typeof data.run; docs: typeof data.docs };

  // data.run/data.docs는 폴링으로 갱신되므로 로컬 $state로 들고, summary/preview는 완료 시점에만 필요하고
  // invalidateAll()로 재계산되므로 data 프롭을 그대로 읽는다(별도 $state 불필요).
  let run = $state(untrack(() => data.run));
  let docs = $state(untrack(() => data.docs));

  let retrying = $state(false);
  let retryError = $state<string | null>(null);
  let selectedFailedDocId = $state<string | null>(null);

  // 도구 원장(진단용) — 문서를 고르면 스테이지별 read/grep/search 호출과 게이트 이벤트를 보여준다.
  type LedgerTool =
    | { turn: number; tool: 'read'; start: number; end: number }
    | { turn: number; tool: 'grep'; pattern: string; total: number }
    | { turn: number; tool: 'search'; query: string; hits: number };
  type LedgerStage = { stage: string; tools: LedgerTool[]; events: { turn?: number; kind: string; detail: string }[]; live: boolean };
  let ledgerDocId = $state<string | null>(null);
  let ledgerLoading = $state(false);
  let ledgerError = $state<string | null>(null);
  let ledgerByDoc = $state<Record<string, LedgerStage[]>>({});

  const toolLine = (t: LedgerTool) =>
    t.tool === 'read'
      ? `[턴${t.turn}] read ${t.start}~${t.end}`
      : t.tool === 'grep'
        ? `[턴${t.turn}] grep '${t.pattern}' → ${t.total}건`
        : `[턴${t.turn}] search '${t.query}' → ${t.hits}건`;

  const openLedger = async (documentId: string) => {
    ledgerDocId = ledgerDocId === documentId ? null : documentId;
    if (!ledgerDocId || Object.hasOwn(ledgerByDoc, documentId)) return;
    ledgerLoading = true;
    ledgerError = null;
    try {
      const res = await fetch(`/admin/api/runs/${run.id}/ledger?documentId=${encodeURIComponent(documentId)}`);
      if (res.ok) {
        const body = (await res.json()) as { stages: LedgerStage[] };
        ledgerByDoc = { ...ledgerByDoc, [documentId]: body.stages };
      } else {
        ledgerError = `원장 조회 실패 (${res.status})`;
      }
    } catch (err) {
      ledgerError = `원장 조회 실패: ${String(err).slice(0, 120)}`;
    } finally {
      ledgerLoading = false;
    }
  };

  // 세션(페이지 진입) 시작 시점 샘플 — 처리율·ETA를 이 시점 대비 실측한다.
  const sessionStart = untrack(() => ({ at: Date.now(), done: primaryMetric(data.run) }));

  // 프리뷰 피드백의 라벨 편집은 이 화면에서 저장되지 않는 로컬 표시일 뿐이다(FeedbackSetPanel 재사용).
  // feedback id는 전역 유일(nanoid)하므로 프리뷰 문서 전체에 걸쳐 하나의 맵만 있어도 안전하다.
  let previewLabelMap = $state<FeedbackLabelMap>({});
  const updatePreviewLabels = (feedbackId: string, entry: FeedbackLabelEntry | null) => {
    if (entry) {
      previewLabelMap = { ...previewLabelMap, [feedbackId]: entry };
      return;
    }
    previewLabelMap = Object.fromEntries(Object.entries(previewLabelMap).filter(([id]) => id !== feedbackId));
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
  const hasDocs = $derived(run.kind === 'pipeline' || run.kind === 'analysis');

  const retryableCount = $derived(docs.filter((d) => d.status === 'failed' || d.status === 'cancelled').length);
  const selectedFailedDoc = $derived(docs.find((d) => d.id === selectedFailedDocId && d.status === 'failed') ?? null);

  const STATUS_LABEL: Record<RunDocStatus, string> = {
    pending: '대기',
    running: '실행 중',
    done: '완료',
    failed: '실패',
    cancelled: '취소됨',
  };

  // 파이프라인 세대별 단계. 구 파이프라인 문서에는 phase가 없다.
  // 순서가 곧 진행도다 — 셀에서 몇 번째 칸까지 찼는지로 어디까지 왔는지 바로 읽힌다.
  const ANALYSIS_PHASES = [
    { key: 'queued', label: '대기' },
    { key: 'survey', label: '작품 파악' },
    { key: 'review', label: '짚을 곳 찾기' },
    { key: 'dedupe', label: '중복 묶기' },
    { key: 'verify', label: '근거 확인' },
    { key: 'compose', label: '피드백 다듬기' },
    { key: 'done', label: '완료' },
  ];
  const EDITORIAL_PHASES = [
    { key: 'queued', label: '대기' },
    { key: 'research', label: '작품 조사' },
    { key: 'plan', label: '비평 계획' },
    { key: 'execute', label: '작품 검토' },
    { key: 'local', label: '문면 교열' },
    { key: 'compose', label: '피드백 다듬기' },
    { key: 'done', label: '완료' },
  ];
  const EDITORIAL_PHASE_KEYS = new Set(['research', 'plan', 'execute', 'local']);
  // 어느 세대의 실행인지는 세트 id(meta)로 먼저 판별하고, 없으면 문서들이 실제로 밟은 단계로
  // 보강한다 — 완료된 실행은 phase가 전부 'done'이라 단계 관측만으로는 판별이 안 된다.
  const PHASES = $derived.by(() => {
    const promptSetId = (run.meta as { promptSetId?: string } | null)?.promptSetId;
    if (promptSetId?.startsWith('aps-editorial')) return EDITORIAL_PHASES;
    if (docs.some((d) => d.phase !== null && EDITORIAL_PHASE_KEYS.has(d.phase))) return EDITORIAL_PHASES;
    return ANALYSIS_PHASES;
  });
  const PHASE_LABEL: Record<string, string> = Object.fromEntries([...ANALYSIS_PHASES, ...EDITORIAL_PHASES].map((p) => [p.key, p.label]));
  // 대기는 아직 시작 전이라 칸을 채우지 않는다.
  const PHASE_STEPS = $derived(PHASES.slice(1));
  const phaseIndex = (phase: string | null) => (phase === null ? -1 : PHASE_STEPS.findIndex((p) => p.key === phase));

  // 서른 편이 어느 단계에 몰려 있는지는 셀을 하나씩 세는 것보다 한 줄 요약이 빠르다.
  const phaseDistribution = $derived.by(() => {
    const counts: Record<string, number> = {};
    for (const doc of docs) {
      if (!doc.phase) continue;
      counts[doc.phase] = (counts[doc.phase] ?? 0) + 1;
    }
    return PHASES.filter((p) => (counts[p.key] ?? 0) > 0).map((p) => ({ label: p.label, count: counts[p.key] ?? 0 }));
  });

  const requestCancel = () => {
    Dialog.confirm({
      title: '실행을 취소할까요?',
      message: '진행 중인 문서는 중단되고 완료된 문서의 결과는 보존됩니다. 재실행하면 완료된 문서는 건너뜁니다.',
      action: 'danger',
      actionLabel: '실행 취소',
      cancelLabel: '되돌아가기',
      actionHandler: async () => {
        const response = await fetch(`/admin/api/runs/${run.id}/cancel`, { method: 'POST' });
        if (!response.ok) {
          Toast.error(`취소에 실패했습니다 (${response.status}).`);
          return false;
        }
        await pollRun();
      },
    });
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

<Helmet title="실행 상세" trailing="타이피 평가" />

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
        <p class={css({ marginTop: '2px', fontSize: '11px', color: 'text.faint' })}>
          입력 {run.promptTokens.toLocaleString()} · 출력 {run.completionTokens.toLocaleString()}
          {#if run.cachedTokens > 0}
            · 캐시 {run.cachedTokens.toLocaleString()}
          {/if}
        </p>
      </div>
      <div class={statCardClass}>
        <p class={css({ fontSize: '12px', color: 'text.faint' })}>비용</p>
        <p class={css({ marginTop: '2px', fontSize: '16px', fontWeight: 'bold' })}>
          {#if data.cost.kind !== 'exact' && data.stageTotal?.complete}
            <!-- 단계마다 모델이 달라 실행 단위로는 금액이 안 나오지만, 단계별 합으로는 나온다. -->
            {formatKrw(data.stageTotal.krw)}
          {:else}
            <CostCell cost={data.cost} tokens={run.promptTokens + run.completionTokens} />
          {/if}
        </p>
        <p class={css({ marginTop: '2px', fontSize: '11px', color: 'text.faint' })}>
          {#if data.krwPerCharacter !== null}
            자당 {data.krwPerCharacter.toFixed(2)}원 · {data.characters.toLocaleString()}자
          {:else if data.models.length > 0}
            {data.models.join(', ')}
          {:else}
            모델 정보 없음
          {/if}
        </p>
      </div>
    </div>

    {#if data.stages.length > 0}
      <div class={css({ marginTop: '16px', overflowX: 'auto' })}>
        <table
          class={css({
            width: 'full',
            fontSize: '12px',
            fontVariantNumeric: 'tabular-nums',
            '& td, & th': { paddingX: '10px', paddingY: '6px', textAlign: 'right', whiteSpace: 'nowrap' },
            '& td:first-child, & th:first-child': { textAlign: 'left' },
            '& th': { color: 'text.faint', fontWeight: 'medium', borderBottomWidth: '1px', borderColor: 'border.default' },
            '& td': { borderBottomWidth: '1px', borderColor: 'border.subtle' },
          })}
        >
          <thead>
            <tr>
              <th>단계</th>
              <th>호출</th>
              <th>입력</th>
              <th>캐시 읽기</th>
              <th>캐시 쓰기</th>
              <th>출력</th>
              <th>비용</th>
              <th>모델</th>
            </tr>
          </thead>
          <tbody>
            {#each data.stages as stage (stage.stage)}
              <tr>
                <td class={css({ fontWeight: 'medium' })}>{STAGE_LABELS[stage.stage] ?? stage.stage}</td>
                <td>{stage.calls.toLocaleString()}</td>
                <td>{stage.promptTokens.toLocaleString()}</td>
                <td class={css({ color: stage.cachedTokens > 0 ? 'text.success' : 'text.faint' })}>
                  {stage.cachedTokens > 0 ? stage.cachedTokens.toLocaleString() : '—'}
                </td>
                <td class={css({ color: 'text.faint' })}>
                  {stage.cacheWriteTokens > 0 ? stage.cacheWriteTokens.toLocaleString() : '—'}
                </td>
                <td>{stage.completionTokens.toLocaleString()}</td>
                <td>{stage.cost.kind === 'exact' ? formatKrw(stage.cost.krw) : '—'}</td>
                <td class={css({ color: 'text.faint' })}>{stage.model?.split('/').at(-1) ?? '—'}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}

    <div class={flex({ gap: '8px', marginTop: '16px', align: 'center' })}>
      {#if run.status === 'running'}
        <button class={outlineButtonClass} onclick={requestCancel} type="button">실행 취소</button>
      {/if}
      {#if (run.kind === 'pipeline' || run.kind === 'analysis') && retryableCount > 0}
        <button class={outlineButtonClass} disabled={retrying} onclick={retryFailed} type="button">
          {retrying ? '재실행하는 중…' : `미완료 문서만 재실행 (${retryableCount})`}
        </button>
      {/if}
    </div>
    <p class={css({ marginTop: '8px', height: '16px', fontSize: '12px', color: 'text.danger' })}>{retryError ?? ''}</p>
  </section>

  {#if hasDocs}
    <section class={sectionClass}>
      <div class={flex({ align: 'center', justify: 'space-between', marginBottom: '12px' })}>
        <h2 class={css({ fontSize: '13px', fontWeight: 'bold', color: 'text.subtle' })}>문서 상태 ({docs.length})</h2>
        <div class={flex({ gap: '10px', fontSize: '11px', color: 'text.faint' })}>
          <span>칸이 찰수록 뒤 단계 · 실패한 문서는 눌러서 오류를 봅니다</span>
        </div>
      </div>

      {#if phaseDistribution.length > 0}
        <p class={flex({ wrap: 'wrap', gap: '10px', marginBottom: '10px', fontSize: '12px', color: 'text.subtle' })}>
          {#each phaseDistribution as entry (entry.label)}
            <span class={css({ fontVariantNumeric: 'tabular-nums' })}>{entry.label} {entry.count}</span>
          {/each}
        </p>
      {/if}

      <div class={grid({ columns: 5, gap: '6px' })}>
        {#each docs as doc, i (doc.id)}
          {@const step = phaseIndex(doc.phase)}
          <button
            class={css({
              display: 'flex',
              flexDirection: 'column',
              gap: '5px',
              paddingX: '8px',
              paddingY: '7px',
              borderWidth: '1px',
              borderColor: selectedFailedDocId === doc.id ? 'border.strong' : 'border.default',
              borderRadius: '8px',
              backgroundColor: doc.status === 'failed' ? 'accent.danger.subtle' : 'surface.default',
              textAlign: 'left',
              cursor: doc.status === 'failed' ? 'pointer' : 'default',
              transition: '[border-color 0.15s ease, background-color 0.15s ease]',
              ['&:hover:not(:disabled)']: { borderColor: 'border.strong' },
            })}
            aria-label={`${doc.documentId} · ${STATUS_LABEL[doc.status]}${doc.phase ? ` · ${PHASE_LABEL[doc.phase] ?? doc.phase}` : ''}`}
            disabled={doc.status !== 'failed'}
            onclick={() => (selectedFailedDocId = doc.id)}
            type="button"
            use:tooltip={{ message: doc.documentId, delay: 200 }}
          >
            <span class={flex({ align: 'baseline', gap: '5px' })}>
              <span class={css({ fontSize: '11px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>{i + 1}</span>
              <span
                class={css({
                  fontSize: '12px',
                  fontWeight: 'bold',
                  color:
                    doc.status === 'failed'
                      ? 'text.danger'
                      : doc.status === 'done'
                        ? 'text.success'
                        : doc.status === 'running'
                          ? 'text.default'
                          : 'text.faint',
                })}
              >
                {doc.phase ? (PHASE_LABEL[doc.phase] ?? doc.phase) : STATUS_LABEL[doc.status]}
              </span>
            </span>

            {#if doc.phase}
              <span class={flex({ gap: '2px' })}>
                {#each PHASE_STEPS as phase, s (phase.key)}
                  <span
                    class={css({
                      flex: '1',
                      height: '3px',
                      borderRadius: 'full',
                      backgroundColor:
                        doc.status === 'failed' && s === step
                          ? 'accent.danger.default'
                          : s <= step
                            ? doc.status === 'done'
                              ? 'accent.success.default'
                              : 'accent.brand.default'
                            : 'surface.muted',
                    })}
                  ></span>
                {/each}
              </span>
            {:else}
              <span
                class={css({
                  height: '3px',
                  borderRadius: 'full',
                  backgroundColor:
                    doc.status === 'done'
                      ? 'accent.success.default'
                      : doc.status === 'running'
                        ? 'accent.brand.default'
                        : doc.status === 'failed'
                          ? 'accent.danger.default'
                          : 'surface.muted',
                })}
              ></span>
            {/if}
          </button>
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

      <div class={css({ marginTop: '16px' })}>
        <h3 class={css({ fontSize: '13px', fontWeight: 'bold', color: 'text.subtle', marginBottom: '8px' })}>도구 원장</h3>
        <div class={flex({ wrap: 'wrap', gap: '6px', marginBottom: '10px' })}>
          {#each docs as doc, i (doc.id)}
            <button
              class={css({
                paddingX: '10px',
                paddingY: '4px',
                borderWidth: '1px',
                borderColor: ledgerDocId === doc.documentId ? 'border.strong' : 'border.default',
                borderRadius: '6px',
                backgroundColor: ledgerDocId === doc.documentId ? 'surface.muted' : 'surface.default',
                fontSize: '12px',
                cursor: 'pointer',
              })}
              onclick={() => openLedger(doc.documentId)}
              type="button"
              use:tooltip={{ message: doc.documentId, delay: 200 }}
            >
              문서 {i + 1}
            </button>
          {/each}
        </div>
        {#if ledgerDocId}
          {#if ledgerLoading && !Object.hasOwn(ledgerByDoc, ledgerDocId)}
            <p class={css({ fontSize: '13px', color: 'text.faint' })}>원장을 불러오는 중…</p>
          {:else if ledgerError && !Object.hasOwn(ledgerByDoc, ledgerDocId)}
            <p class={css({ fontSize: '13px', color: 'text.danger' })}>{ledgerError}</p>
          {:else if (ledgerByDoc[ledgerDocId] ?? []).length === 0}
            <p class={css({ fontSize: '13px', color: 'text.faint' })}>이 문서의 도구 원장이 없습니다 (구 파이프라인 실행).</p>
          {:else}
            <div class={flex({ direction: 'column', gap: '12px' })}>
              {#each ledgerByDoc[ledgerDocId] ?? [] as stage (stage.stage)}
                <div class={css({ borderWidth: '1px', borderColor: 'border.default', borderRadius: '8px', padding: '10px' })}>
                  <p class={css({ fontSize: '12px', fontWeight: 'bold', marginBottom: '6px' })}>
                    {STAGE_LABELS[stage.stage] ?? stage.stage}
                    <span class={css({ fontWeight: 'normal', color: 'text.faint' })}>
                      · 도구 {stage.tools.length}{stage.live ? ' · 진행 중 (이벤트는 단계 완료 후)' : ` · 이벤트 ${stage.events.length}`}
                    </span>
                  </p>
                  {#if stage.tools.length > 0}
                    <pre
                      class={css({
                        fontSize: '12px',
                        fontFamily: 'mono',
                        whiteSpace: 'pre-wrap',
                        overflowWrap: 'anywhere',
                        color: 'text.default',
                      })}>{stage.tools.map(toolLine).join('\n')}</pre>
                  {/if}
                  {#if stage.events.length > 0}
                    <pre
                      class={css({
                        fontSize: '12px',
                        fontFamily: 'mono',
                        whiteSpace: 'pre-wrap',
                        overflowWrap: 'anywhere',
                        color: 'text.danger',
                        marginTop: stage.tools.length > 0 ? '6px' : '0',
                      })}>{stage.events.map((e) => `[${e.kind}] ${e.detail}`).join('\n')}</pre>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        {:else}
          <p class={css({ fontSize: '13px', color: 'text.faint' })}>
            문서를 선택하면 스테이지별 read·grep·search 호출과 게이트 이벤트가 표시됩니다.
          </p>
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
          <p class={css({ fontSize: '12px', color: 'text.faint' })}>피드백 0건 문서</p>
          <p class={css({ marginTop: '2px', fontSize: '18px', fontWeight: 'bold' })}>
            {data.summary.feedbackDistribution.zero}
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
                labelMap={previewLabelMap}
                onHover={previewNoop}
                onSelect={previewNoop}
                onUpdateLabels={updatePreviewLabels}
              />
            </div>
          {/each}
        </div>
      </section>
    {/if}
  {/if}
</div>
