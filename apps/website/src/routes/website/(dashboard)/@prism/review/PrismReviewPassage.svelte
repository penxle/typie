<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { ConfirmDecisionSchema, PRISM_REVIEW_TIERS } from '@typie/prism';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Tooltip } from '@typie/ui/components';
  import { tick, untrack } from 'svelte';
  import { fade } from 'svelte/transition';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import { backoffDelay } from '../lib/backoff.ts';
  import { parseMarkdown } from '../lib/markdown.ts';
  import { expand, fadeIn, fadeOut, MOTION, rise, STAGGER } from '../lib/motion.ts';
  import { PacedText } from '../lib/paced-text.svelte.ts';
  import PrismMarkdown from '../PrismMarkdown.svelte';
  import PrismToolCalls from '../PrismToolCalls.svelte';
  import PrismToolRequest from '../PrismToolRequest.svelte';
  import PrismWaitRow from '../PrismWaitRow.svelte';
  import { buildPassage, elapsedLabel, runningLabel, spentLabel, tailLabel } from './passage-view.ts';
  import PrismReviewResult from './PrismReviewResult.svelte';
  import { describeHeader, describeResult, findRound, recheckMode } from './round-view.ts';
  import { stagesFor, stepRound, stepStage } from './stages.ts';
  import { TIER_OPTIONS } from './tiers.ts';
  import type { PrismReviewTierName } from '@typie/prism';
  import type { WorkflowStatus } from '../lib/conversation.ts';
  import type { WorkflowBlockProps } from '../workflows/index.ts';
  import type { PassageGroup, PassageView, StageView } from './passage-view.ts';
  import type { StageKey } from './stages.ts';

  let { message, sessionId, transcript, requests, failedIds, reconnecting, resolve, onRetry, loadTrace }: WorkflowBlockProps = $props();

  const query = createQuery(
    graphql(`
      query DashboardLayout_PrismReviewPassage_Query($sessionId: ID!) {
        prismSession(sessionId: $sessionId) {
          id

          reviewRounds {
            id
            state
            tier
            ordinal
            issueCount
            hasDetail
            reaction
            reactionNote

            workflow {
              id
              prismWorkflowId
            }

            document {
              id
              title
            }

            rejection {
              message
            }

            conclusion {
              patternsCount
              prioritiesCount
              strengthsCount
            }
          }
        }
      }
    `),
    () => ({ sessionId }),
  );

  const RECHECK_DELAYS = [2000, 5000, 15_000, 30_000];
  const TAIL_DEBOUNCE_MS = 1000;

  const workflowId = $derived(message.workflowId);
  const rounds = $derived(query.data?.prismSession.reviewRounds ?? null);
  const round = $derived(rounds === null ? null : findRound(rounds, workflowId));
  const namedTier = $derived<PrismReviewTierName | null>(
    (PRISM_REVIEW_TIERS as readonly string[]).includes(message.name) ? (message.name as PrismReviewTierName) : null,
  );
  const tier = $derived<PrismReviewTierName | null>(round === null ? namedTier : (round.tier.toLowerCase() as PrismReviewTierName));
  const status = $derived<WorkflowStatus>(
    message.status === 'completed'
      ? round?.state === 'CANCELED'
        ? 'canceled'
        : round?.state === 'FAILED'
          ? 'failed'
          : 'completed'
      : message.status,
  );
  const mode = $derived(recheckMode(round, rounds !== null, status));

  let attempts = $state(0);
  let exhausted = $state(false);
  let timer = $state<ReturnType<typeof setTimeout> | null>(null);

  const clearTimer = () => {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
  };

  const recheck = () => {
    timer = null;
    cache.invalidate({ __typename: 'PrismSession', id: sessionId, $field: 'reviewRounds' });
  };

  const retry = () => {
    clearTimer();
    attempts = 0;
    exhausted = false;
    query.refetch();
  };

  $effect(() => {
    void sessionId;
    void workflowId;

    untrack(() => {
      attempts = 0;
      exhausted = false;
    });

    return clearTimer;
  });

  $effect(() => {
    if (mode !== 'stale') {
      untrack(() => {
        clearTimer();

        if (mode === 'settled') {
          attempts = 0;
          exhausted = false;
        }
      });

      return;
    }

    if (exhausted || timer !== null) {
      return;
    }

    untrack(() => {
      const next = attempts + 1;
      const delay = backoffDelay(RECHECK_DELAYS, next);

      if (delay === null) {
        exhausted = true;
        return;
      }

      attempts = next;
      timer = setTimeout(recheck, delay);
    });
  });

  let now = $state(Date.now());
  let lastBeat = $state(Date.now());

  $effect(() => {
    if (status !== 'running') {
      return;
    }

    const id = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(id);
  });

  $effect.pre(() => {
    void message.trace;
    lastBeat = Date.now();
  });

  const running = $derived(status === 'running');
  const view = $derived<PassageView>(
    tier === null
      ? { prelude: [], stages: [], current: null, liveRound: null, elapsedMs: 0 }
      : buildPassage({ trace: message.trace, status, tier, requests, now, finishedAt: message.finishedAt ?? null }),
  );
  const order = $derived(tier === null ? [] : stagesFor(tier));
  const reached = $derived(view.stages.filter((stage) => stage.status !== 'pending').length);
  const header = $derived(round === null ? null : describeHeader(round));

  const seedTitle = $derived.by(() => {
    for (let i = transcript.messages.length - 1; i >= 0; i -= 1) {
      const entry = transcript.messages[i];
      if (entry.role !== 'tool-request' || entry.tool !== 'confirm-review' || entry.status !== 'resolved') continue;
      if ((entry.settledAt ?? entry.at) > message.startedAt) continue;
      const parsed = ConfirmDecisionSchema.safeParse(entry.result);
      if (parsed.success && parsed.data.decision === 'confirmed') return `「${parsed.data.document.title || '제목 없음'}」`;
    }
    return null;
  });
  const title = $derived(header?.title ?? seedTitle ?? '「…」');
  const tierLabel = $derived(header?.tier ?? (tier === null ? null : (TIER_OPTIONS.find((option) => option.tier === tier)?.label ?? null)));
  const result = $derived(round === null ? null : describeResult(round));

  const reduceMotion = typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  let firstPaint = $state(true);

  $effect(() => {
    void tick().then(() => (firstPaint = false));
  });

  const liveDelay = (index: number) => (firstPaint ? MOTION.quick + index * STAGGER : 0);
  const settledDelay = (index: number) => (firstPaint ? MOTION.quick : 0) + index * STAGGER;

  let live = $state<PacedText | null>(null);
  let drains = $state<{ seq: number; stage: StageKey | null; round: number | null; paced: PacedText }[]>([]);
  let liveKey = '';
  let liveTurns = 0;

  const drainOf = (seq: number) => drains.find((entry) => entry.seq === seq);
  const draining = $derived(drains.some((entry) => !entry.paced.done));

  // 배출이 끝나기 전에 뒤 그룹을 세우면 표시 순서가 역전되고, 자라는 서술이 그 그룹을 밀어낸다.
  const gateGroups = (list: PassageGroup[]): PassageGroup[] => {
    const index = list.findIndex((group) => {
      if (group.kind !== 'narration') return false;
      const drain = drainOf(group.seq);
      return drain !== undefined && !drain.paced.done;
    });

    return index === -1 ? list : list.slice(0, index + 1);
  };

  $effect.pre(() => {
    const turn = message.trace.live;
    const key = turn ? `${turn.context.agent.id}:${turn.context.run}:${turn.context.turn}:${turn.context.attempt}` : '';

    if (key !== liveKey) {
      untrack(() => {
        if (live !== null && key === '') {
          const sealed = message.trace.turns.at(-1);

          if (sealed !== undefined && message.trace.turns.length > liveTurns && live.boundary > 0) {
            live.finalize(sealed.text);
            if (!live.done) {
              const stage = (sealed.step === null ? null : stepStage(sealed.step)) ?? view.current;
              const round = sealed.step === null ? null : stepRound(sealed.step);
              drains = [...drains, { seq: sealed.seq, stage, round, paced: live }];
            }
          }
        }

        liveKey = key;
        liveTurns = message.trace.turns.length;
        live = key === '' ? null : new PacedText({ instant: reduceMotion });

        if (live !== null && turn?.seeded) {
          live.retarget(turn.textBroken ? '' : turn.text);
          live.skip();
        }
      });
    }

    if (turn && live !== null) live.retarget(turn.textBroken ? '' : turn.text);
  });

  $effect(() => {
    if (live === null && drains.length === 0) {
      return;
    }

    let disposed = false;
    let last = performance.now();
    let id = requestAnimationFrame(function loop(time) {
      if (disposed) return;
      const dt = Math.min(time - last, 100);
      last = time;
      live?.advance(dt);

      let settled = false;
      for (const { paced } of drains) {
        paced.advance(dt);
        settled ||= paced.done;
      }
      if (settled) drains = drains.filter(({ paced }) => !paced.done);

      id = requestAnimationFrame(loop);
    });

    return () => {
      disposed = true;
      cancelAnimationFrame(id);
    };
  });

  const HOLD_MS = 5000;
  const remainingHold = (at: number) => Math.max(0, at + HOLD_MS - Date.now());

  let heldStages = $state<{ key: StageKey; at: number }[]>([]);
  const heldStageKeys = $derived(heldStages.map((entry) => entry.key));
  let prevCurrent: StageKey | null = null;

  $effect(() => {
    const currentStage = running ? view.current : null;

    if (currentStage === prevCurrent) {
      return;
    }

    untrack(() => {
      const finished = prevCurrent;
      if (finished !== null && heldStages.every((entry) => entry.key !== finished)) {
        heldStages = [...heldStages, { key: finished, at: Date.now() }];
      }
      prevCurrent = currentStage;
    });
  });

  $effect(() => {
    if (heldStages.length === 0) {
      return;
    }

    const busy = new Set(drains.map((entry) => entry.stage));
    const timers = heldStages
      .filter((entry) => !busy.has(entry.key))
      .map((entry) => setTimeout(() => (heldStages = heldStages.filter((held) => held.key !== entry.key)), remainingHold(entry.at)));

    return () => {
      for (const timer of timers) clearTimeout(timer);
    };
  });

  let heldRounds = $state<{ key: string; stage: StageKey; round: number; at: number }[]>([]);
  const heldRoundKeys = $derived(heldRounds.map((entry) => entry.key));
  let prevLatestRounds: Record<string, { key: string; round: number }> = {};

  $effect(() => {
    if (!running) {
      return;
    }

    const latest: Record<string, { key: string; round: number }> = {};
    for (const stage of view.stages) {
      const box = stage.groups.findLast((group) => group.kind === 'round');
      if (box !== undefined && box.kind === 'round') latest[stage.key] = { key: box.key, round: box.round };
    }

    untrack(() => {
      for (const [stageKey, next] of Object.entries(latest)) {
        const prev = prevLatestRounds[stageKey];
        if (prev !== undefined && prev.key !== next.key && heldRounds.every((entry) => entry.key !== prev.key)) {
          heldRounds = [...heldRounds, { key: prev.key, stage: stageKey as StageKey, round: prev.round, at: Date.now() }];
        }
        prevLatestRounds[stageKey] = next;
      }
    });
  });

  $effect(() => {
    if (heldRounds.length === 0) {
      return;
    }

    const busy = new Set(drains.filter((entry) => entry.round !== null).map((entry) => `${entry.stage}|${entry.round}`));
    const timers = heldRounds
      .filter((entry) => !busy.has(`${entry.stage}|${entry.round}`))
      .map((entry) => setTimeout(() => (heldRounds = heldRounds.filter((held) => held.key !== entry.key)), remainingHold(entry.at)));

    return () => {
      for (const timer of timers) clearTimeout(timer);
    };
  });

  const awaiting = $derived(requests.some((request) => request.status === 'pending'));
  const rawTail = $derived(running && !awaiting ? tailLabel({ live: message.trace.live, reconnecting, lateMs: now - lastBeat }) : null);
  let tail = $state<string | null>(null);

  $effect(() => {
    const next = rawTail;

    if (next === null) {
      tail = null;
      return;
    }

    const id = setTimeout(() => (tail = next), TAIL_DEBOUNCE_MS);
    return () => clearTimeout(id);
  });

  let expanded = $state<Record<string, boolean>>({});
  let traceLoad = $state<'idle' | 'loading' | 'loaded' | 'failed'>('idle');
  let traceAttempts = $state(0);

  const loadProcess = async (background = false) => {
    if (!background) traceLoad = 'loading';

    try {
      await loadTrace();
      traceLoad = 'loaded';
      traceAttempts = 0;
    } catch {
      traceLoad = 'failed';
    }
  };

  $effect(() => {
    if (running || traceLoad !== 'idle' || message.trace.steps.length > 0) {
      return;
    }

    untrack(() => void loadProcess());
  });

  $effect(() => {
    if (traceLoad !== 'failed') {
      return;
    }

    const next = traceAttempts + 1;
    const delay = backoffDelay(RECHECK_DELAYS, next);
    if (delay === null) {
      return;
    }

    const id = setTimeout(() => {
      traceAttempts = next;
      void loadProcess(true);
    }, delay);
    return () => clearTimeout(id);
  });

  const traced = $derived(running || message.trace.steps.length > 0);

  const segmentTip = $derived.by(() => {
    const total = order.length;
    if (running) return `${total}단계 중 ${Math.max(1, reached)}단계 진행 중`;
    if (status === 'completed') return reached === total ? `${total}단계 마침` : `${Math.max(1, reached)}단계에서 마침`;
    return `${Math.max(1, reached)}단계에서 ${status === 'canceled' ? '중단' : '멈춤'}`;
  });

  const elapsedText = $derived.by(() => {
    if (running) return elapsedLabel(view.elapsedMs);
    const minutes = spentLabel(view.elapsedMs);
    if (status === 'completed') return `${minutes} 만에 마침`;
    return status === 'canceled' ? `${minutes} 만에 중단` : `${minutes} 만에 멈춤`;
  });

  const headClass = flex({ alignItems: 'baseline', gap: '8px', paddingTop: '12px', borderTopWidth: '1px', borderColor: 'border.subtle' });
  const titleClass = css({
    minWidth: '0',
    fontSize: '13px',
    fontWeight: 'semibold',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  });
  const metaClass = css({ flexShrink: '0', fontSize: '11px', color: 'text.faint' });
  const rightClass = flex({ alignItems: 'center', gap: '8px', marginLeft: 'auto', flexShrink: '0' });
  const segmentsClass = flex({ alignItems: 'center', gap: '2px' });
  const segmentStyle = css.raw({ width: '9px', height: '4px', borderRadius: '2px', transition: '[background-color 250ms ease]' });
  const runningSegmentStyle = css.raw({ animation: 'pulse 2.4s ease-in-out infinite', _motionReduce: { animation: 'none' } });
  const stageRowStyle = flex.raw({ alignItems: 'center', gap: '8px', marginTop: '14px', fontSize: '[12.5px]' });
  const stageNameStyle = css.raw({ fontWeight: 'semibold', flexShrink: '0' });
  const summaryClass = css({
    flexGrow: '1',
    minWidth: '0',
    fontSize: '12px',
    color: 'text.faint',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  });
  const timeClass = css({ flexShrink: '0', marginLeft: 'auto', fontSize: '11px', color: 'text.disabled' });
  const chevronStyle = css.raw({
    flexShrink: '0',
    color: 'text.disabled',
    transition: '[transform 150ms cubic-bezier(0.23, 1, 0.32, 1)]',
    _motionReduce: { transition: '[none]' },
  });
  const chevronOpenStyle = css.raw({ transform: 'rotate(180deg)' });
  const glyphSlotClass = css({ position: 'relative', size: '14px', flexShrink: '0' });
  const glyphBaseStyle = css.raw({
    position: 'absolute',
    top: '0',
    right: '0',
    bottom: '0',
    left: '0',
    margin: 'auto',
    size: '14px',
    borderRadius: 'full',
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
  });
  const ringClass = css(glyphBaseStyle, {
    borderWidth: '2px',
    borderColor: 'border.default',
    borderTopColor: 'text.subtle',
    animation: 'spin 1s linear infinite',
    _motionReduce: { animation: 'none' },
  });
  const expandClass = css({ display: '[flow-root]' });
  const narrationClass = css({ marginTop: '10px', fontSize: '14px', lineHeight: '[1.6]' });
  const roundBoxClass = css({
    marginTop: '10px',
    paddingX: '10px',
    paddingY: '8px',
    borderWidth: '1px',
    borderColor: 'border.subtle',
    borderRadius: '8px',
  });
  const roundHeadStyle = flex.raw({
    alignItems: 'center',
    gap: '6px',
    fontSize: '[11.5px]',
    fontWeight: 'semibold',
    color: 'text.subtle',
  });
  const retryRowClass = flex({ alignItems: 'center', gap: '6px', marginTop: '10px', fontSize: '12px', color: 'text.subtle' });
  const skeletonStyle = css.raw({
    marginTop: '10px',
    height: '12px',
    borderRadius: '4px',
    backgroundColor: 'surface.muted',
    animation: 'pulse 1.6s ease-in-out infinite',
  });
  const systemClass = css({ marginTop: '10px', fontSize: '[12.5px]', color: 'text.faint' });
</script>

{#snippet stageGlyph(status: StageView['status'])}
  <span class={glyphSlotClass}>
    {#if status === 'running'}
      <span class={ringClass} in:fade={fadeIn} out:fade={fadeOut}></span>
    {:else if status === 'done'}
      <span class={css(glyphBaseStyle, { backgroundColor: 'text.default', color: 'surface.default' })} in:fade={fadeIn} out:fade={fadeOut}>
        <Icon icon={CheckIcon} size={10} />
      </span>
    {:else if status === 'canceled'}
      <span
        class={css(glyphBaseStyle, { backgroundColor: 'text.faint', color: 'surface.default', fontSize: '8px' })}
        in:fade={fadeIn}
        out:fade={fadeOut}
      >
        ■
      </span>
    {:else if status === 'failed'}
      <span
        class={css(glyphBaseStyle, {
          backgroundColor: 'text.default',
          color: 'surface.default',
          fontSize: '10px',
          fontWeight: 'bold',
        })}
        in:fade={fadeIn}
        out:fade={fadeOut}
      >
        !
      </span>
    {:else}
      <span
        class={css(glyphBaseStyle, { size: '12px', borderWidth: '[1.5px]', borderStyle: 'dashed', borderColor: 'border.default' })}
        in:fade={fadeIn}
        out:fade={fadeOut}
      ></span>
    {/if}
  </span>
{/snippet}

{#snippet liveNarration()}
  {#if live !== null && live.boundary > 0 && !draining}
    <div class={narrationClass}><PrismMarkdown blocks={live.blocks} plain={live.plain} /></div>
  {/if}
{/snippet}

{#snippet groups(list: PassageGroup[], latestSeq: number | null)}
  {#each gateGroups(list) as group (group.key)}
    {#if group.kind === 'narration'}
      {@const groupDrain = drainOf(group.seq)}
      {#if groupDrain !== undefined}
        <div class={narrationClass}><PrismMarkdown blocks={groupDrain.paced.blocks} plain={groupDrain.paced.plain} /></div>
      {:else}
        <div class={narrationClass}><PrismMarkdown blocks={parseMarkdown(group.text)} /></div>
      {/if}
    {:else if group.kind === 'tools'}
      <div class={css({ marginTop: '8px' })} in:rise><PrismToolCalls count={group.count} rows={group.rows} /></div>
    {:else if group.kind === 'question'}
      <div class={css({ marginTop: '12px' })} in:rise={{ block: true }}>
        <PrismToolRequest {failedIds} message={group.request} {onRetry} {resolve} {transcript} />
      </div>
    {:else}
      {@const open = expanded[group.key] ?? (group.seq === latestSeq || heldRoundKeys.includes(group.key))}
      <div class={roundBoxClass} in:rise={{ block: true }}>
        <button
          class={css(roundHeadStyle, { width: 'full' })}
          aria-expanded={open}
          onclick={() => (expanded[group.key] = !open)}
          type="button"
        >
          점검 {group.round}회째
          <span class={timeClass}>{spentLabel(group.elapsedMs)}</span>
          <Icon style={css.raw(chevronStyle, open ? chevronOpenStyle : {})} icon={ChevronDownIcon} size={10} />
        </button>
        {#if open}
          <div class={expandClass} transition:expand>
            {@render groups(group.groups, null)}
          </div>
        {/if}
      </div>
    {/if}
  {/each}
{/snippet}

{#snippet stageBody(stage: StageView)}
  {@const latestRound = stage.groups.findLast((group) => group.kind === 'round')?.seq ?? null}
  {@render groups(stage.groups, stage.status === 'running' ? latestRound : null)}
{/snippet}

{#snippet stageLine(stage: StageView, toggleable: boolean, delay: number = 0)}
  {@const open = expanded[stage.key] ?? false}
  <button
    class={css(stageRowStyle, { width: 'full', textAlign: 'left' })}
    aria-expanded={toggleable ? open : undefined}
    disabled={!toggleable}
    onclick={() => (expanded[stage.key] = !open)}
    type="button"
    in:rise|global={{ delay }}
  >
    {@render stageGlyph(stage.status)}
    <span
      class={css(
        stageNameStyle,
        stage.status === 'pending'
          ? { color: 'text.disabled', fontWeight: 'medium' }
          : stage.status === 'running'
            ? {}
            : { color: 'text.subtle' },
      )}
    >
      {stage.label}
    </span>
    {#if stage.status === 'running'}
      <span class={timeClass}>{stage.elapsedMs === null ? '' : runningLabel(stage.elapsedMs)}</span>
    {:else if stage.status !== 'pending'}
      <span class={summaryClass}>
        {#if stage.status === 'canceled'}여기서 중단했어요{:else if stage.status === 'failed'}문제가 생겨 멈췄어요{:else}{stage.summary ??
            ''}{#if stage.rounds > 0}
            · 점검 {stage.rounds}회{/if}{/if}
      </span>
      <span class={timeClass}>{stage.elapsedMs === null ? '' : spentLabel(stage.elapsedMs)}</span>
      {#if toggleable}
        <Icon style={css.raw(chevronStyle, open ? chevronOpenStyle : {})} icon={ChevronDownIcon} size={10} />
      {/if}
    {/if}
  </button>
  {#if toggleable && open}
    <div class={expandClass} transition:expand>
      {@render stageBody(stage)}
    </div>
  {/if}
{/snippet}

<div>
  <div class={headClass}>
    <span class={titleClass}>{title}</span>
    <span class={metaClass}>
      {#if tierLabel !== null}· {tierLabel}{/if}{#if round !== null && round.ordinal > 1}
        · {round.ordinal}회차{/if}
    </span>
    {#if traced && order.length > 0}
      <div class={rightClass} in:fade={fadeIn}>
        <Tooltip message={segmentTip}>
          <span class={segmentsClass}>
            {#each order as stage, index (stage.key)}
              {@const status = view.stages[index]?.status ?? 'pending'}
              <span
                class={css(segmentStyle, status === 'running' ? runningSegmentStyle : {}, {
                  backgroundColor:
                    status === 'done'
                      ? 'text.default'
                      : status === 'running'
                        ? 'text.faint'
                        : status === 'pending'
                          ? 'border.default'
                          : 'border.strong',
                })}
              ></span>
            {/each}
          </span>
        </Tooltip>
        <span class={metaClass}>{elapsedText}</span>
      </div>
    {/if}
  </div>

  {@render groups(view.prelude, null)}

  {#if running}
    {#each view.stages.filter((stage) => stage.status !== 'pending') as stage, index (stage.key)}
      {@const showBody = stage.status === 'running' || heldStageKeys.includes(stage.key)}
      {@render stageLine(stage, !showBody && stage.groups.length > 0, liveDelay(index))}
      {#if showBody}
        <div class={expandClass} out:expand>
          {@render stageBody(stage)}
        </div>
      {/if}
    {/each}
    {@render liveNarration()}
    {#if !awaiting}
      <PrismWaitRow style={css.raw({ marginTop: '10px' })} label={tail ?? '리뷰가 진행 중이에요'} text={tail} />
    {/if}
  {:else}
    {#if traceLoad === 'loading'}
      <div class={css(skeletonStyle, { width: '[55%]' })}></div>
      <div class={css(skeletonStyle, { width: '[40%]' })}></div>
    {:else if traceLoad === 'failed'}
      <div class={retryRowClass} in:fade={fadeIn}>
        리뷰 과정을 불러오지 못했어요
        <Button onclick={() => void loadProcess()} size="sm" variant="secondary">다시 시도</Button>
      </div>
    {:else}
      {#each view.stages.filter((stage) => stage.status !== 'pending') as stage, index (stage.key)}
        {@const held = heldStageKeys.includes(stage.key)}
        {@render stageLine(stage, !held && stage.groups.length > 0, settledDelay(index))}
        {#if held}
          <div class={expandClass} out:expand>
            {@render stageBody(stage)}
          </div>
        {/if}
      {/each}
    {/if}

    {#if status === 'canceled'}
      <p class={systemClass} in:rise>리뷰를 중단했어요. 다시 받으려면 /리뷰로 새로 시작해 주세요.</p>
    {:else if status === 'failed'}
      <p class={systemClass} in:rise>리뷰를 끝내지 못했어요. 다시 받으려면 /리뷰로 새로 시작해 주세요.</p>
    {:else if (query.error && rounds === null) || exhausted}
      <div class={retryRowClass} in:fade={fadeIn}>
        리뷰 결과를 불러오지 못했어요
        <Button onclick={retry} size="sm" variant="secondary">다시 시도</Button>
      </div>
    {:else if round !== null && result !== null}
      {#if result.kind !== 'rejected'}
        <div class={css({ marginTop: '12px' })} in:rise={{ block: true }}><PrismReviewResult {result} {round} /></div>
      {/if}
    {:else}
      <div class={css(skeletonStyle, { width: '[40%]' })}></div>
    {/if}
  {/if}
</div>
