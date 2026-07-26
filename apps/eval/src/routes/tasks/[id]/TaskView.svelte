<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import { onMount, untrack } from 'svelte';
  import IconArrowRight from '~icons/lucide/arrow-right';
  import IconCheck from '~icons/lucide/check';
  import IconChevronLeft from '~icons/lucide/chevron-left';
  import IconCircleCheck from '~icons/lucide/circle-check';
  import IconCornerUpLeft from '~icons/lucide/corner-up-left';
  import IconInfo from '~icons/lucide/info';
  import IconSave from '~icons/lucide/save';
  import { deserialize, enhance } from '$app/forms';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import { FEEDBACK_LABEL_KEYS } from '$lib/domain/feedback-labels.ts';
  import { computeSegments } from '$lib/domain/highlight.ts';
  import { EMPTY_REVIEW_VERDICT, hasRejection, isFeedbackComplete, isReviewComplete, parseWorkReview } from '$lib/domain/verdicts.ts';
  import AnalysisFeedbackPanel from './AnalysisFeedbackPanel.svelte';
  import FeedbackSetPanel from './FeedbackSetPanel.svelte';
  import FindingRail from './FindingRail.svelte';
  import WorkReviewPanel from './WorkReviewPanel.svelte';
  import type { FeedbackLabelEntry, FeedbackLabelMap } from '$lib/domain/feedback-labels.ts';
  import type { JudgmentResult, PairVerdict } from '$lib/domain/types.ts';
  import type { FeedbackVerdict, FeedbackVerdictMap, ReviewVerdictMap } from '$lib/domain/verdicts.ts';
  import type { PageData } from './$types';
  import type { RailMark } from './FindingRail.svelte';

  // evaluate = 실제 평가, preview = 어드민 점검(조작은 되지만 저장 안 됨),
  // read = 열람 전용(판정 문항과 제출을 아예 걸지 않는다).
  type Props = { data: PageData; mode?: 'evaluate' | 'preview' | 'read' };
  const { data, mode = 'evaluate' }: Props = $props();

  const preview = $derived(mode === 'preview');
  const readOnly = $derived(mode === 'read');

  // elapsed_seconds = 이 태스크에 쓰인 총 활성 시간. 저장된 누적값에서 이어서 세고,
  // 입력 없이 IDLE_LIMIT_MS를 넘긴 구간과 창 이탈~복귀 구간은 세지 않는다.
  const IDLE_LIMIT_MS = 5 * 60 * 1000;
  let activeMs = untrack(() => (data.draft?.elapsedSeconds ?? 0) * 1000);
  let lastActivityAt = Date.now();
  let inactive = false;
  const recordActivity = () => {
    const now = Date.now();
    if (!inactive) {
      const gap = now - lastActivityAt;
      if (gap < IDLE_LIMIT_MS) activeMs += gap;
    }
    inactive = false;
    lastActivityAt = now;
  };
  const suspendActivity = () => {
    recordActivity();
    inactive = true;
  };

  const labels = ['A', 'B', 'C', 'D'];
  // 스크리닝·확정은 후보끼리 견주는 자리라 질(質) 척도를 쓴다. 절대평가는 견줄 상대가 없어
  // 같은 척도를 그대로 쓰면 무엇에 견준 '부실'인지가 평가자마다 갈린다 — 도움 여부를 직접 묻는다.
  const QUALITY_ANCHORS = [
    { score: 1, anchor: '매우 부실' },
    { score: 2, anchor: '부실' },
    { score: 3, anchor: '보통' },
    { score: 4, anchor: '좋음' },
    { score: 5, anchor: '훌륭' },
  ];
  const HELPFULNESS_ANCHORS = [
    { score: 1, anchor: '전혀' },
    { score: 2, anchor: '별로' },
    { score: 3, anchor: '보통' },
    { score: 4, anchor: '도움됨' },
    { score: 5, anchor: '큰 도움' },
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
  let verdictMap = $state<FeedbackVerdictMap>(
    untrack(() => Object.fromEntries(Object.entries(data.verdicts).filter(([feedbackId]) => taskFeedbackIds.has(feedbackId)))),
  );
  let reviewVerdictMap = $state<ReviewVerdictMap>(
    untrack(() => Object.fromEntries(Object.entries(data.reviewVerdicts).filter(([setId]) => data.task.setIds.includes(setId)))),
  );
  let comment = $state(untrack(() => data.draft?.comment ?? ''));
  let hoveredFeedbackId = $state<string | null>(null);
  let focusedFeedbackId = $state<string | null>(null);
  let focusedAnchorKey = $state<string | null>(null);
  let activeSetIndex = $state(0);
  let activeTab = $state<'review' | 'findings'>('review');
  let savedAt = $state<string | null>(null);
  let saving = $state(false);
  let submitting = $state(false);
  let submitError = $state<string | null>(null);
  const busy = $derived(saving || submitting);

  let documentPaneEl = $state<HTMLElement | undefined>();
  let reviewPaneEl = $state<HTMLElement | undefined>();
  let findingsPaneEl = $state<HTMLElement | undefined>();
  const paneScrollTops: Record<'review' | 'findings', number> = { review: 0, findings: 0 };
  let documentViewport = $state<{ start: number; end: number } | null>(null);

  const outlineButtonClass = css({
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    gap: '6px',
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
  // 절대평가는 세트가 한 벌이라 세트 비교용 문구(같은 평가 허용 · N/M 세트 완료)가 성립하지 않는다.
  const multiSet = $derived(data.task.setIds.length > 1);
  const scoreAnchors = $derived(data.isAnalysis ? HELPFULNESS_ANCHORS : QUALITY_ANCHORS);

  const result = $derived.by((): JudgmentResult | null => {
    if (isRanking) {
      if (Object.values(scores).includes(0)) return null;
      return { kind: 'scores', scores: Object.entries(scores).map(([setId, score]) => ({ setId, score })) };
    }
    return verdict ? { kind: 'pair', verdict } : null;
  });

  const activeSet = $derived(data.sets[activeSetIndex]);
  const activeReview = $derived(parseWorkReview(activeSet.review));

  // 피드백 1건이 본문 여러 곳에 붙는다. 하이라이트는 위치 단위로 그리되 키에 피드백 id를 남겨,
  // 카드에 마우스를 올리면 그 피드백의 모든 위치가 함께 켜지도록 한다.
  const anchorRefs = $derived.by(() => {
    const refs: { key: string; feedbackId: string; start: number; end: number }[] = [];
    for (const feedback of activeSet.feedbacks) {
      const positions =
        activeReview === null
          ? [{ matchStart: feedback.matchStart, matchEnd: feedback.matchEnd }]
          : feedback.anchors.map((a) => ({ matchStart: a.matchStart, matchEnd: a.matchEnd }));
      let index = 0;
      for (const { matchStart, matchEnd } of positions) {
        if (matchStart === null || matchEnd === null) continue;
        refs.push({ key: `${feedback.id}:${index}`, feedbackId: feedback.id, start: matchStart, end: matchEnd });
        index++;
      }
    }
    return refs;
  });

  const feedbackIdByAnchorKey = $derived(new Map(anchorRefs.map((ref) => [ref.key, ref.feedbackId])));

  const segments = $derived(
    computeSegments(
      data.document.content,
      anchorRefs.map((ref) => ({ start: ref.start, end: ref.end, feedbackId: ref.key })),
    ),
  );

  const feedbackNumbers = $derived<Record<string, number>>(Object.fromEntries(activeSet.feedbacks.map((f, i) => [f.id, i + 1])));

  const firstSegmentOf = $derived.by(() => {
    const seen: Record<string, number> = {};
    for (const [i, segment] of segments.entries()) {
      for (const key of segment.feedbackIds) {
        seen[key] ??= i;
      }
    }
    return seen;
  });

  // 피드백마다 첫 앵커 위치만 남긴다 — 레일은 분포를 보여주는 것이지 모든 앵커를 그리는 자리가 아니다.
  const firstAnchorStart = $derived.by(() => {
    const starts: Record<string, number> = {};
    for (const ref of anchorRefs) {
      starts[ref.feedbackId] ??= ref.start;
    }
    return starts;
  });

  const railMarks = $derived.by((): RailMark[] => {
    const firstStart = firstAnchorStart;
    const length = Math.max(1, data.document.content.length);
    return activeSet.feedbacks.flatMap((feedback, i) => {
      const start = firstStart[feedback.id];
      if (start === undefined) return [];
      const state = hasRejection(verdictMap[feedback.id]) ? 'fail' : isFeedbackComplete(verdictMap[feedback.id]) ? 'seen' : 'unseen';
      return [{ feedbackId: feedback.id, number: i + 1, position: Math.min(1, start / length), state }];
    });
  });

  // '남음'은 세 항목을 다 채우지 못한 피드백 수다. 하나라도 비면 그 피드백은 집계에 쓸 수 없다.
  const pendingCount = $derived(activeSet.feedbacks.filter((f) => !isFeedbackComplete(verdictMap[f.id])).length);
  // 제출 조건 — 배정된 세트마다 피드백 전부와 총평 두 문항이 채워져야 한다.
  const analysisComplete = $derived.by(() => {
    if (!data.isAnalysis) return true;
    return data.sets.every(
      (set) =>
        set.feedbacks.every((f) => isFeedbackComplete(verdictMap[f.id])) &&
        (set.review === null || isReviewComplete(reviewVerdictMap[set.setId])),
    );
  });

  let submitButtonEl = $state<HTMLButtonElement | undefined>();

  const reducedMotion = () => globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;

  // scrollIntoView는 조상 컨테이너까지 함께 굴린다. 판정 화면은 본문·패널이 각자 스크롤하므로
  // 대상이 든 컨테이너 하나만 직접 굴려야 다른 쪽이 제자리를 잃지 않는다.
  const scrollWithin = (container: HTMLElement, target: HTMLElement) => {
    const containerRect = container.getBoundingClientRect();
    const targetRect = target.getBoundingClientRect();
    const offset = targetRect.top - containerRect.top - containerRect.height / 2 + targetRect.height / 2;
    container.scrollTo({ top: container.scrollTop + offset, behavior: reducedMotion() ? 'auto' : 'smooth' });
  };

  const switchTab = (tab: 'review' | 'findings') => {
    if (tab === activeTab) return;
    const current = activeTab === 'review' ? reviewPaneEl : findingsPaneEl;
    if (current) paneScrollTops[activeTab] = current.scrollTop;
    activeTab = tab;
    requestAnimationFrame(() => {
      const next = tab === 'review' ? reviewPaneEl : findingsPaneEl;
      if (next) next.scrollTop = paneScrollTops[tab];
    });
  };

  // 카드로 이동. 총평 탭에 있었다면 피드백 탭으로 옮기되, 총평의 스크롤 위치는 그대로 남는다.
  const focusFeedback = (feedbackId: string) => {
    focusedFeedbackId = feedbackId;
    if (!data.isAnalysis) {
      const el = document.querySelector<HTMLElement>(`[data-feedback-card="${feedbackId}"]`);
      if (el && findingsPaneEl) scrollWithin(findingsPaneEl, el);
      return;
    }
    switchTab('findings');
    requestAnimationFrame(() => {
      const el = document.querySelector<HTMLElement>(`[data-feedback-card="${feedbackId}"]`);
      if (el && findingsPaneEl) scrollWithin(findingsPaneEl, el);
    });
  };

  const focusAnchor = (feedbackId: string, anchorIndex = 0) => {
    const key = `${feedbackId}:${anchorIndex}`;
    focusedFeedbackId = feedbackId;
    focusedAnchorKey = key;
    const el = document.querySelector<HTMLElement>(`[data-anchor="${key}"]`);
    if (el && documentPaneEl) scrollWithin(documentPaneEl, el);
  };

  // 다른 자리에서 피드백으로 건너뛸 때는 목록과 본문을 함께 옮긴다. 목록만 움직이면 그 지적이
  // 원고 어디를 가리키는지 직접 찾아야 해서 대조가 끊긴다.
  //
  // 본문 하이라이트를 눌러 들어온 경우에는 쓰지 않는다 — 이미 그 자리에 있고, 앵커가 여럿인
  // 피드백에서 0번으로 튀면 방금 누른 위치를 잃는다.
  const revealFeedback = (feedbackId: string) => {
    focusFeedback(feedbackId);
    focusAnchor(feedbackId, 0);
  };

  // 대시보드에서 "#12를 왜 아니오로 봤나"를 되짚어 올 때, 그 피드백과 본문 위치까지 바로 잡아준다.
  onMount(() => {
    const wanted = page.url.searchParams.get('feedback');
    if (!wanted || !taskFeedbackIds.has(wanted)) return;
    revealFeedback(wanted);
  });

  const seekDocument = (fraction: number) => {
    const el = documentPaneEl;
    if (!el) return;
    el.scrollTop = fraction * el.scrollHeight - el.clientHeight / 2;
  };

  const trackDocumentViewport = () => {
    const el = documentPaneEl;
    if (!el || el.scrollHeight <= el.clientHeight) {
      documentViewport = null;
      return;
    }
    documentViewport = { start: el.scrollTop / el.scrollHeight, end: (el.scrollTop + el.clientHeight) / el.scrollHeight };
  };

  const requestSubmit = () => {
    Dialog.confirm({
      title: '평가를 제출할까요?',
      message: '제출한 뒤에는 수정할 수 없고, 다음 평가로 이동합니다.',
      actionLabel: '제출',
      actionHandler: () => {
        submitButtonEl?.click();
      },
    });
  };

  // 자동 저장 — 평가 한 편이 한 시간 가까이 걸리는데 저장이 수동이면 잃을 것이 너무 크다.
  // 마지막 입력에서 잠시 멎었을 때만 보내고, 저장된 내용과 같으면 보내지 않는다.
  const AUTOSAVE_DELAY_MS = 3000;
  let autosaveTimer: ReturnType<typeof setTimeout> | undefined;

  const snapshot = () => JSON.stringify({ result, labelMap, verdictMap, reviewVerdictMap, comment, elapsed: Math.round(activeMs / 1000) });

  // 불러온 상태를 저장된 것으로 친다 — 이 초깃값이 없으면 화면을 열자마자 손대지도 않은 판정이
  // 한 번 저장된다.
  let savedSnapshot = untrack(() => snapshot());

  const formPayload = () => {
    const body = new FormData();
    body.set('result', result ? JSON.stringify(result) : '');
    body.set('feedbackLabels', JSON.stringify(labelMap));
    body.set('verdicts', JSON.stringify(verdictMap));
    body.set('reviewVerdicts', JSON.stringify(reviewVerdictMap));
    body.set('comment', comment);
    body.set('elapsedSeconds', String(Math.round(activeMs / 1000)));
    return body;
  };

  // keepalive는 탭을 닫는 중에도 요청이 끝까지 가게 한다 — 화면을 떠나며 흘리는 입력을 막는다.
  // 본문은 64KB까지만 허용되지만 판정과 사유를 다 합쳐도 그 절반에 못 미친다.
  const autosave = async (keepalive = false) => {
    if (mode !== 'evaluate' || busy) return;
    const current = snapshot();
    if (current === savedSnapshot) return;

    saving = true;
    try {
      const response = await fetch('?/save', { method: 'POST', body: formPayload(), keepalive });
      const outcome = deserialize(await response.text());
      if (outcome.type === 'error' || outcome.type === 'failure') {
        submitError = outcome.type === 'error' ? (outcome.error?.message ?? '알 수 없는 오류') : '저장하지 못했습니다';
        return;
      }
      savedSnapshot = current;
      submitError = null;
      savedAt = new Date().toLocaleTimeString('ko', { hour: '2-digit', minute: '2-digit' });
    } catch (err) {
      submitError = String(err);
    } finally {
      saving = false;
    }
  };

  $effect(() => {
    if (mode !== 'evaluate') return;
    // snapshot()이 상태 전부를 읽어 의존성을 건다. 값이 바뀔 때마다 타이머를 다시 건다.
    const current = snapshot();
    if (current === savedSnapshot) return;
    clearTimeout(autosaveTimer);
    autosaveTimer = setTimeout(() => void autosave(), AUTOSAVE_DELAY_MS);
    return () => clearTimeout(autosaveTimer);
  });

  const requestRelease = () => {
    Dialog.confirm({
      title: '이 글을 반납할까요?',
      message: '입력한 내용은 사라지고, 이 글은 다시 배정되지 않습니다. 다른 평가자에게는 정상적으로 배정됩니다.',
      action: 'danger',
      actionLabel: '반납',
      actionHandler: async () => {
        const response = await fetch('?/release', { method: 'POST', body: new FormData() });
        const result = deserialize(await response.text());
        if (result.type === 'redirect') {
          await goto(result.location);
          return;
        }
        Dialog.alert({ title: '반납 실패', message: '잠시 후 다시 시도해주세요.' });
      },
    });
  };

  const updateLabels = (feedbackId: string, entry: FeedbackLabelEntry | null) => {
    if (entry) {
      labelMap = { ...labelMap, [feedbackId]: entry };
      return;
    }
    labelMap = Object.fromEntries(Object.entries(labelMap).filter(([id]) => id !== feedbackId));
  };

  const updateVerdict = (feedbackId: string, verdict: FeedbackVerdict) => {
    verdictMap = { ...verdictMap, [feedbackId]: verdict };
  };

  // 피드백 사이 이동은 본문과 목록을 함께 옮긴다 — 마흔 건짜리 목록에서 마우스로 짝을 맞추는 일이 없도록.
  const stepFeedback = (delta: number) => {
    const list = activeSet.feedbacks;
    if (list.length === 0) return;
    const current = list.findIndex((f) => f.id === focusedFeedbackId);
    const next = list[(current + delta + list.length) % list.length];
    if (!next) return;
    revealFeedback(next.id);
  };

  const jumpToPending = () => {
    const next = activeSet.feedbacks.find((f) => !isFeedbackComplete(verdictMap[f.id]));
    if (!next) return;
    revealFeedback(next.id);
  };

  const onKeydown = (e: KeyboardEvent) => {
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    const target = e.target as HTMLElement | null;
    if (target && ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)) return;

    if (multiSet) {
      const index = Number(e.key) - 1;
      if (index >= 0 && index < data.sets.length) {
        activeSetIndex = index;
        return;
      }
    }
    if (!data.isAnalysis) return;
    if (e.key === 'j') stepFeedback(1);
    else if (e.key === 'k') stepFeedback(-1);
    else if (e.key === 'u') jumpToPending();
    else if (e.key === 'r') switchTab(activeTab === 'review' ? 'findings' : 'review');
  };

  const readingMinutes = $derived(Math.max(1, Math.round(data.document.characterCount / 500)));
  const scoredCount = $derived(data.task.setIds.filter((setId) => (scores[setId] ?? 0) > 0).length);

  const tabClass = (selected: boolean) =>
    flex({
      align: 'center',
      justify: 'center',
      gap: '6px',
      flex: '1',
      paddingY: '9px',
      borderBottomWidth: '1px',
      borderColor: selected ? 'text.default' : '[transparent]',
      color: selected ? 'text.default' : 'text.faint',
      fontSize: '13px',
      fontWeight: selected ? 'bold' : 'normal',
      cursor: 'pointer',
      transition: '[color 0.15s ease, border-color 0.15s ease]',
      _hover: { color: 'text.default' },
    });
</script>

<svelte:window onblur={suspendActivity} onfocus={recordActivity} onkeydown={onKeydown} />
<svelte:document
  onkeydown={recordActivity}
  onpointerdown={recordActivity}
  onpointermove={recordActivity}
  onscrollcapture={recordActivity}
  ontouchstart={recordActivity}
  onvisibilitychange={() => {
    if (document.visibilityState === 'hidden') {
      suspendActivity();
      void autosave(true);
    } else {
      recordActivity();
    }
  }}
  onwheel={recordActivity}
/>

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
    <!-- 열람 전용은 직접 받은 링크로 들어온다 — 돌아갈 곳이 없으므로 링크를 걸지 않는다. -->
    {#if !readOnly}
      <a
        class={flex({ align: 'center', gap: '2px', fontSize: '13px', color: 'text.subtle', _hover: { color: 'text.default' } })}
        href={preview ? '/admin/tasks' : '/'}
      >
        <Icon icon={IconChevronLeft} size={14} />
        {preview ? '태스크 목록' : '평가 큐'}
      </a>
    {/if}
    {#if preview || readOnly}
      <span
        class={css({
          paddingX: '8px',
          paddingY: '2px',
          borderRadius: 'full',
          fontSize: '12px',
          fontWeight: 'medium',
          backgroundColor: 'accent.warning.subtle',
          color: 'accent.warning.default',
        })}
      >
        {preview ? '관리자 미리보기 — 입력은 저장되지 않습니다' : '열람 전용'}
      </span>
    {:else}
      <div class={flex({ align: 'center', gap: '8px' })}>
        <span class={css({ fontSize: '13px', color: 'text.subtle', fontVariantNumeric: 'tabular-nums' })}>
          내 판정 {data.progress.done} / {data.progress.myTotal} · 라운드 전체 {data.progress.roundDone} / {data.progress.roundRequired}
        </span>
        <div class={css({ width: '120px', height: '4px', borderRadius: 'full', backgroundColor: 'surface.muted', overflow: 'hidden' })}>
          <div
            style:width={`${data.progress.roundRequired === 0 ? 0 : Math.round((data.progress.roundDone / data.progress.roundRequired) * 100)}%`}
            class={css({ height: 'full', backgroundColor: 'accent.brand.default' })}
          ></div>
        </div>
      </div>
    {/if}
    <span
      class={flex({
        align: 'center',
        gap: '4px',
        marginLeft: 'auto',
        fontSize: '13px',
        color: 'text.faint',
        fontVariantNumeric: 'tabular-nums',
      })}
    >
      {data.document.characterCount.toLocaleString()}자 · 약 {readingMinutes}분
      {#if saving}
        · 저장 중…
      {:else if submitError}
        <!-- 실패한 뒤에도 마지막 성공 시각이 남아 있으면 저장된 것으로 읽힌다. -->
        ·
        <span class={css({ color: 'text.danger', fontWeight: 'medium' })}>저장되지 않음</span>
      {:else if savedAt}
        · <Icon icon={IconCheck} size={12} /> 임시 저장됨 {savedAt}
      {/if}
    </span>
    <ThemeToggle />
  </header>

  <div class={grid({ columns: 2, gap: '0', gridTemplateColumns: '[minmax(0, 1fr) 460px]', flex: '1', minHeight: '0' })}>
    <div class={flex({ minHeight: '0', paddingRight: '12px' })}>
      <section
        bind:this={documentPaneEl}
        class={css({
          flex: '1',
          minWidth: '0',
          overflowY: 'auto',
          overflowAnchor: 'none',
          paddingY: '32px',
          paddingX: '20px',
          // 레일이 스크롤바 역할을 대신한다 — 같은 일을 하는 막대를 둘 두지 않는다.
          scrollbarWidth: 'none',
          ['&::-webkit-scrollbar']: { display: 'none' },
        })}
        onscroll={trackDocumentViewport}
      >
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
              {@const owners = segment.feedbackIds.map((key) => feedbackIdByAnchorKey.get(key))}
              {@const active = owners.includes(hoveredFeedbackId ?? '') || owners.includes(focusedFeedbackId ?? '')}
              {@const current = segment.feedbackIds.includes(focusedAnchorKey ?? '')}
              <span
                class={css({
                  position: 'relative',
                  backgroundColor: current ? 'amber.400' : active ? 'amber.300' : 'amber.100',
                  borderBottomWidth: '2px',
                  borderColor: current ? 'amber.600' : 'amber.400',
                  _dark: {
                    backgroundColor: current ? '[#8a7619]' : active ? '[#6e5f16]' : '[#4a4012]',
                    borderColor: current ? '[#c9ad25]' : '[#93801c]',
                  },
                  borderRadius: '2px',
                  color: '[inherit]',
                  cursor: 'pointer',
                  transition: '[background-color 0.15s ease]',
                  scrollMarginBlock: '80px',
                })}
                onclick={() => owners[0] && focusFeedback(owners[0])}
                onkeydown={(e) => {
                  if ((e.key === 'Enter' || e.key === ' ') && owners[0]) {
                    e.preventDefault();
                    focusFeedback(owners[0]);
                  }
                }}
                onmouseenter={() => (hoveredFeedbackId = owners[0] ?? null)}
                onmouseleave={() => (hoveredFeedbackId = null)}
                role="button"
                tabindex="0"
              >
                {#each segment.feedbackIds as key, bi (key)}
                  {@const fid = feedbackIdByAnchorKey.get(key) ?? key}
                  {#if firstSegmentOf[key] === i}
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
                      data-anchor={key}
                      onclick={(e) => {
                        e.stopPropagation();
                        focusFeedback(fid);
                      }}
                      onkeydown={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                          e.preventDefault();
                          e.stopPropagation();
                          focusFeedback(fid);
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
      {#if data.isAnalysis && railMarks.length > 0}
        <FindingRail marks={railMarks} onSeek={seekDocument} onSelect={revealFeedback} viewport={documentViewport} />
      {/if}
    </div>

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
      {#if multiSet}
        <nav class={css({ paddingX: '16px', paddingTop: '12px', flexShrink: '0' })}>
          <div style:grid-template-columns={`repeat(${data.sets.length}, 1fr)`} class={css({ display: 'grid', gap: '6px' })}>
            {#each data.sets as set, i (`${i}-${set.setId}`)}
              <button
                class={css({
                  display: 'inline-flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: '4px',
                  paddingY: '7px',
                  borderRadius: '8px',
                  borderWidth: '1px',
                  borderColor: activeSetIndex === i ? 'border.strong' : 'border.default',
                  backgroundColor: activeSetIndex === i ? 'surface.dark' : 'surface.default',
                  color: activeSetIndex === i ? 'text.bright' : 'text.default',
                  fontSize: '13px',
                  fontWeight: activeSetIndex === i ? 'bold' : 'normal',
                  cursor: 'pointer',
                  transition: '[background-color 0.15s ease, border-color 0.15s ease, color 0.15s ease]',
                })}
                onclick={() => (activeSetIndex = i)}
                type="button"
              >
                세트 {labels[i]}
                <span class={css({ fontSize: '11px', fontWeight: 'normal', opacity: '70' })}>{set.feedbacks.length}건</span>
                {#if isRanking && (scores[set.setId] ?? 0) > 0}
                  <Icon style={css.raw({ color: activeSetIndex === i ? 'text.bright' : 'text.success' })} icon={IconCheck} size={12} />
                {/if}
              </button>
            {/each}
          </div>
        </nav>
      {/if}

      {#if data.isAnalysis && activeReview}
        <!-- 총평과 피드백을 한 스크롤에 두면, 총평이 가리키는 피드백으로 뛴 순간 총평을 잃는다.
             각자 스크롤을 갖게 하고 전환 시 위치를 되돌려 읽던 자리를 지킨다. -->
        <div class={flex({ paddingX: '16px', borderBottomWidth: '1px', borderColor: 'border.default', flexShrink: '0' })}>
          <button class={tabClass(activeTab === 'review')} onclick={() => switchTab('review')} type="button">작품 총평</button>
          <button class={tabClass(activeTab === 'findings')} onclick={() => switchTab('findings')} type="button">
            피드백 {activeSet.feedbacks.length}건
            {#if pendingCount > 0 && !readOnly}
              <span class={css({ fontWeight: 'normal', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
                {pendingCount} 남음
              </span>
            {/if}
          </button>
        </div>

        <div
          bind:this={reviewPaneEl}
          style:display={activeTab === 'review' ? 'block' : 'none'}
          class={css({ flex: '1', overflowY: 'auto', paddingY: '20px', minHeight: '0' })}
        >
          <WorkReviewPanel
            feedbackLabels={activeSet.feedbacks.map((f) => ({ id: f.id, category: f.category }))}
            onSelectFeedback={revealFeedback}
            onUpdate={(next) => (reviewVerdictMap = { ...reviewVerdictMap, [activeSet.setId]: next })}
            {readOnly}
            review={activeReview}
            verdict={reviewVerdictMap[activeSet.setId] ?? EMPTY_REVIEW_VERDICT}
          />
        </div>

        <div
          bind:this={findingsPaneEl}
          style:display={activeTab === 'findings' ? 'block' : 'none'}
          class={css({ flex: '1', overflowY: 'auto', paddingY: '4px', minHeight: '0' })}
        >
          <AnalysisFeedbackPanel
            feedbacks={activeSet.feedbacks}
            focusedId={focusedFeedbackId}
            onHover={(id) => (hoveredFeedbackId = id)}
            onSelect={focusAnchor}
            onUpdate={updateVerdict}
            {readOnly}
            verdicts={verdictMap}
          />
        </div>
      {:else}
        <div
          bind:this={findingsPaneEl}
          class={css({ flex: '1', overflowY: 'auto', padding: '16px', minHeight: '0', backgroundColor: 'surface.subtle' })}
        >
          <FeedbackSetPanel
            feedbacks={activeSet.feedbacks}
            highlightedId={focusedFeedbackId}
            {labelMap}
            onHover={(id) => (hoveredFeedbackId = id)}
            onSelect={focusAnchor}
            onUpdateLabels={updateLabels}
          />
        </div>
      {/if}

      {#if !readOnly}
        <form
          class={css({ padding: '16px', borderTopWidth: '1px', borderColor: 'border.default', flexShrink: '0' })}
          method="post"
          use:enhance={({ action, formData, cancel }) => {
            if (busy) {
              cancel();
              return;
            }
            recordActivity();
            formData.set('elapsedSeconds', String(Math.round(activeMs / 1000)));
            if (action.search.includes('save')) saving = true;
            else submitting = true;
            return async ({ result, update }) => {
              saving = false;
              submitting = false;
              // 실패를 성공으로 칠하지 않는다. update()를 그대로 태우면 오류 화면이 렌더되어
              // 아직 저장되지 않은 입력까지 통째로 날아가므로, 화면은 그대로 두고 알리기만 한다.
              if (result.type === 'error' || result.type === 'failure') {
                submitError =
                  result.type === 'error'
                    ? (result.error?.message ?? '알 수 없는 오류')
                    : ((result.data?.message as string | undefined) ?? '저장하지 못했습니다');
                return;
              }
              submitError = null;
              // 수동 저장으로 이미 보낸 내용을 자동 저장이 다시 보내지 않게 맞춰둔다.
              savedSnapshot = snapshot();
              await update({ reset: false });
              savedAt = new Date().toLocaleTimeString('ko', { hour: '2-digit', minute: '2-digit' });
            };
          }}
        >
          {#if isRanking}
            <fieldset class={flex({ direction: 'column', gap: '6px' })}>
              <legend class={css({ fontSize: '13px', fontWeight: 'bold', marginBottom: '6px' })}>
                {#if data.isAnalysis}
                  이 피드백을 받았다면 도움이 되었을까요?
                {:else}
                  점수
                  <span class={css({ fontWeight: 'normal', color: 'text.faint' })}>(같은 평가 허용)</span>
                {/if}
                {#if multiSet}
                  <span class={css({ fontWeight: 'normal', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
                    · {scoredCount} / {data.task.setIds.length} 세트 완료
                  </span>
                {/if}
              </legend>
              {#each data.task.setIds as setId, i (setId)}
                <div
                  class={`${flex({ align: 'center', gap: '8px', paddingX: '6px', paddingY: '4px', borderRadius: '8px', transition: '[background-color 0.15s ease]' })} ${
                    multiSet && activeSetIndex === i ? css({ backgroundColor: 'surface.muted' }) : ''
                  }`}
                >
                  {#if multiSet}
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
                  {/if}
                  <div class={grid({ columns: 5, gap: '4px', flex: '1' })}>
                    {#each scoreAnchors as { score, anchor } (score)}
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
              <legend class={css({ fontSize: '13px', fontWeight: 'bold', marginBottom: '6px' })}>
                어느 세트의 피드백이 더 나은가요?
                <span class={css({ fontWeight: 'normal', color: 'text.faint' })}>
                  — 두 세트가 비슷하거나 동일해 보여도 오류가 아닙니다. 보이는 그대로 판정해 주세요.
                </span>
              </legend>
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
          <input name="verdicts" type="hidden" value={JSON.stringify(verdictMap)} />
          <input name="reviewVerdicts" type="hidden" value={JSON.stringify(reviewVerdictMap)} />

          {#if preview}
            <p class={flex({ align: 'center', gap: '4px', marginTop: '10px', fontSize: '12px', color: 'text.faint' })}>
              <Icon icon={IconInfo} size={12} />
              미리보기 모드입니다 — 판정을 조작해볼 수 있지만 저장·제출되지 않습니다.
            </p>
          {:else}
            <div class={flex({ wrap: 'wrap', gap: '8px', marginTop: '10px', align: 'center' })}>
              <button class={outlineButtonClass} disabled={busy} formaction="?/save" type="submit">
                <Icon icon={IconSave} size={14} />
                {saving ? '저장 중…' : '임시 저장'}
              </button>
              <button
                class={css({
                  flex: '1',
                  display: 'inline-flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: '6px',
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
                disabled={!result || !analysisComplete || busy}
                onclick={requestSubmit}
                type="button"
              >
                {submitting ? '제출 중…' : '제출하고 다음으로'}
                <Icon icon={IconArrowRight} size={14} />
              </button>
              <button bind:this={submitButtonEl} aria-hidden="true" formaction="?/submit" hidden tabindex="-1" type="submit"></button>
              <button class={outlineButtonClass} disabled={busy} onclick={requestRelease} type="button">
                <Icon icon={IconCornerUpLeft} size={14} />
                반납
              </button>
            </div>
          {/if}

          {#if submitError}
            <!-- 화면은 그대로 두고 알리기만 한다 — 입력이 남아 있어야 다시 눌러 되살릴 수 있다. -->
            <div
              class={css({
                marginTop: '10px',
                paddingX: '12px',
                paddingY: '10px',
                borderWidth: '1px',
                borderColor: 'border.danger',
                borderRadius: '8px',
                backgroundColor: 'accent.danger.subtle',
                fontSize: '12px',
                lineHeight: '[1.6]',
                color: 'text.danger',
              })}
              role="alert"
            >
              <p class={css({ fontWeight: 'bold' })}>저장되지 않았습니다. 화면을 닫지 마세요.</p>
              <p class={css({ marginTop: '2px' })}>입력은 그대로 있습니다. 다시 눌러주세요. 계속 실패하면 관리자에게 알려주세요.</p>
              <p class={css({ marginTop: '4px', color: 'text.faint', wordBreak: 'break-all' })}>{submitError}</p>
            </div>
          {/if}

          <p class={flex({ align: 'center', gap: '4px', marginTop: '8px', minHeight: '16px', fontSize: '12px', color: 'text.faint' })}>
            {#if data.isAnalysis && !analysisComplete}
              <Icon icon={IconInfo} size={12} />
              배정된 글은 피드백 전부에 세 항목을 답해야 제출됩니다.
              {#if pendingCount > 0}
                <kbd class={css({ fontSize: '11px', color: 'text.subtle' })}>U</kbd>
                로 남은 {pendingCount}건으로 이동합니다.
              {:else}
                작품 총평의 두 문항이 남았습니다.
              {/if}
            {:else if data.isAnalysis}
              <kbd class={css({ fontSize: '11px', color: 'text.subtle' })}>J</kbd>
              /
              <kbd class={css({ fontSize: '11px', color: 'text.subtle' })}>K</kbd>
              피드백 이동 ·
              <kbd class={css({ fontSize: '11px', color: 'text.subtle' })}>R</kbd>
              총평·피드백 전환
            {:else if result}
              <Icon style={css.raw({ color: 'text.success' })} icon={IconCircleCheck} size={12} />
              제출하면 다음 평가로 바로 이동합니다.
            {:else}
              <Icon icon={IconInfo} size={12} />
              {isRanking ? '모든 세트에 점수를 매기면' : '판정을 선택하면'} 제출할 수 있습니다.
            {/if}
          </p>
        </form>
      {/if}
    </aside>
  </div>
</div>
