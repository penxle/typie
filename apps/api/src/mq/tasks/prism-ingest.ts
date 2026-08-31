import { setTimeout as timeout } from 'node:timers/promises';
import { logger } from '@typie/lib';
import { applyDelta, awaitingUser, effectiveResolver, parked, pendingRequests, sealTurn } from '@typie/prism';
import { DelayedError } from 'bullmq';
import dayjs from 'dayjs';
import { and, asc, eq, inArray, isNotNull, isNull, ne } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import { db, first, firstOrThrow, PrismRuns, PrismSessionEvents, PrismSessions, PrismWorkflowEvents, PrismWorkflows } from '#/db/index.ts';
import { prism, PrismApiError } from '#/external/prism.ts';
import { pumpSse } from '#/external/prism-stream.ts';
import { pubsub } from '#/pubsub.ts';
import { opsAlertOnce } from '#/utils/ops-alert.ts';
import { prismApps } from '#/utils/prism-apps.ts';
import { chargePrismCredit } from '#/utils/prism-credit.ts';
import { projectFrame } from '#/utils/prism-events.ts';
import {
  absentDelay,
  liveFieldKey,
  logKeyOf,
  PARKED_KINDS,
  parseLogKey,
  planEvent,
  shouldStop,
  STALE_BEATS,
  staleBeats,
} from '#/utils/prism-ingest-core.ts';
import { prismNotificationUserActionKey, prismRunNotification } from '#/utils/prism-notification.ts';
import { serveTool } from '#/utils/prism-serve.ts';
import { resolvedToolCallIds, toolResolverOf } from '#/utils/prism-tool-calls.ts';
import { closeRun, linkWorkflowFromEvent, titleSession } from '#/utils/prism-workflows.ts';
import { ensureIngest, LOCK_LOST, shutdown } from '../prism-queue.ts';
import type { EventFrame, ParkedEvent, ParkedOptions, ParkedScope, StreamFrame, TurnLive } from '@typie/prism';
import type { Job } from 'bullmq';
import type { Transaction } from '#/db/index.ts';
import type { ProjectedScope } from '#/utils/prism-events.ts';
import type { DomainOp, IngestTarget } from '#/utils/prism-ingest-core.ts';

const log = logger.getChild('prism-ingest');

const IDLE_MS = 45_000;
const RECONNECT_DELAY_MS = 1000;
const LIVE_TTL_SECONDS = 120;
const LIVE_TTL_REFRESH_MS = 30_000;
const GENERATION_CHECK_MS = 60_000;

type SessionRef = { id: string; prismAgentId: string; userId: string; title: string | null };

type PumpOutcome = 'done' | 'relocate';

class StopPump extends Error {}
class StalePump extends Error {}

const sleep = async (ms: number, signal: AbortSignal): Promise<void> => {
  try {
    await timeout(ms, undefined, { signal });
  } catch (err) {
    if (!(err instanceof Error) || err.name !== 'AbortError') throw err;
  }
};

const liveKey = (sessionId: string) => `prism:live:${sessionId}`;

export const loadParkedEvents = async (target: IngestTarget): Promise<ParkedEvent[]> => {
  const rows =
    target.kind === 'agent'
      ? await db
          .select({
            kind: PrismSessionEvents.kind,
            context: PrismSessionEvents.context,
            data: PrismSessionEvents.data,
            occurredAt: PrismSessionEvents.occurredAt,
          })
          .from(PrismSessionEvents)
          .where(and(eq(PrismSessionEvents.sessionId, target.sessionId), inArray(PrismSessionEvents.kind, [...PARKED_KINDS])))
          .orderBy(asc(PrismSessionEvents.seq))
      : await db
          .select({
            kind: PrismWorkflowEvents.kind,
            context: PrismWorkflowEvents.context,
            data: PrismWorkflowEvents.data,
            occurredAt: PrismWorkflowEvents.occurredAt,
          })
          .from(PrismWorkflowEvents)
          .where(and(eq(PrismWorkflowEvents.workflowId, target.workflowId), inArray(PrismWorkflowEvents.kind, [...PARKED_KINDS])))
          .orderBy(asc(PrismWorkflowEvents.seq));

  return rows.map((row) => ({ kind: row.kind, context: row.context ?? null, data: row.data, occurredAt: row.occurredAt.valueOf() }));
};

const loadSettledWorkflows = async (sessionId: string): Promise<Set<string>> => {
  const rows = await db
    .select({ prismWorkflowId: PrismWorkflows.prismWorkflowId })
    .from(PrismWorkflows)
    .where(and(eq(PrismWorkflows.sessionId, sessionId), ne(PrismWorkflows.state, 'RUNNING')));

  return new Set(rows.map((row) => row.prismWorkflowId));
};

// 서버가 선해소한 user 도구는 tool.resolved가 스트림으로 닿기 전까지 로컬 로그에서 대기로 보인다 — 원장이 해소를 아는 요청은 대기에서 뺀다
const ledgerResolved = async (sessionId: string, events: ParkedEvent[], scope: ParkedScope): Promise<ReadonlySet<string>> =>
  resolvedToolCallIds(
    sessionId,
    pendingRequests(events, scope).map((request) => request.toolCallId),
  );

export const agentParked = async (sessionId: string, events: ParkedEvent[], resolverOf?: ParkedOptions['resolverOf']): Promise<boolean> =>
  parked(events, 'agent', { resolverOf }) &&
  parked(events, 'agent', {
    settledWorkflows: await loadSettledWorkflows(sessionId),
    resolverOf,
    resolved: await ledgerResolved(sessionId, events, 'agent'),
  });

const workflowParked = async (sessionId: string, events: ParkedEvent[], resolverOf: ParkedOptions['resolverOf']): Promise<boolean> =>
  parked(events, 'workflow', { resolverOf }) &&
  parked(events, 'workflow', { resolverOf, resolved: await ledgerResolved(sessionId, events, 'workflow') });

const applyAgentOp = async (tx: Transaction, session: SessionRef, op: DomainOp): Promise<(() => Promise<void>) | null> => {
  switch (op.op) {
    case 'run-started': {
      await tx
        .insert(PrismRuns)
        .values({ sessionId: session.id, runSeq: op.runSeq, startedAt: dayjs(op.at) })
        .onConflictDoUpdate({ target: [PrismRuns.sessionId, PrismRuns.runSeq], set: { startedAt: dayjs(op.at) } });
      return null;
    }

    case 'run-terminal': {
      const where = and(eq(PrismRuns.sessionId, session.id), eq(PrismRuns.runSeq, op.runSeq));
      const updated = await tx
        .update(PrismRuns)
        .set({ state: op.state, finishedAt: dayjs(op.at) })
        .where(and(where, eq(PrismRuns.state, 'RUNNING')))
        .returning({ id: PrismRuns.id, state: PrismRuns.state, startedAt: PrismRuns.startedAt, finishedAt: PrismRuns.finishedAt })
        .then(first);
      const run =
        updated ??
        (await tx
          .select({ id: PrismRuns.id, state: PrismRuns.state, startedAt: PrismRuns.startedAt, finishedAt: PrismRuns.finishedAt })
          .from(PrismRuns)
          .where(where)
          .then(first)) ??
        (await tx
          .insert(PrismRuns)
          .values({ sessionId: session.id, runSeq: op.runSeq, state: op.state, startedAt: dayjs(op.at), finishedAt: dayjs(op.at) })
          .onConflictDoNothing({ target: [PrismRuns.sessionId, PrismRuns.runSeq] })
          .returning({ id: PrismRuns.id, state: PrismRuns.state, startedAt: PrismRuns.startedAt, finishedAt: PrismRuns.finishedAt })
          .then(first));

      await closeRun(tx, session.id, op.runSeq);

      let unknownCharge: 'unknown-price' | 'run-missing' | null = null;
      if (op.state !== 'FAILED' && op.charge !== undefined) {
        if (op.charge === null) unknownCharge = 'unknown-price';
        else if (run === undefined) unknownCharge = 'run-missing';
        else await chargePrismCredit(tx, { userId: session.userId, kind: 'CHAT_CHARGE', key: run.id, amount: op.charge });
      }

      return async () => {
        // 단가 미지는 전 유저 공통 조건이라 전역 1일 1회, run 미확보는 정산 1건의 유실이라 건마다 — 한 키로 접으면 후자가 전자에 묻힌다
        if (unknownCharge) {
          const dedupeKey = unknownCharge === 'unknown-price' ? 'global' : `run:${session.id}:${op.runSeq}`;
          await opsAlertOnce('prism-credit-charge-unknown', dedupeKey, { cause: unknownCharge, sessionId: session.id, runSeq: op.runSeq });
        }
        if (run?.finishedAt) {
          const rawUserActionAt = await redis.get(prismNotificationUserActionKey(session.prismAgentId)).catch((err) => {
            log.warn('prism notification action timestamp unavailable: {sessionId} {*}', { sessionId: session.id, error: err });
            return null;
          });
          const notification = prismRunNotification({
            sessionId: session.id,
            runSeq: op.runSeq,
            state: run.state,
            startedAt: run.startedAt.valueOf(),
            lastUserActionAt: rawUserActionAt === null ? undefined : Number(rawUserActionAt),
            finishedAt: run.finishedAt.valueOf(),
          });
          if (notification) pubsub.publish('prism:notification', session.userId, notification);
        }
        pubsub.publish('prism:credit', session.userId, {});
      };
    }

    case 'workflow-link': {
      const row = await linkWorkflowFromEvent(tx, session.id, op.descriptor);
      if (row.sessionId !== session.id) {
        log.warn('prism workflow belongs to another session: {workflowId}', { workflowId: op.descriptor.prismWorkflowId });
        return null;
      }

      return () => ensureIngest({ kind: 'workflow', workflowId: row.id });
    }

    case 'titled': {
      await titleSession(tx, session.id, op.title);
      session.title = op.title;
      return null;
    }

    case 'tool-serve': {
      const { toolCallId, tool, input, agentId, runSeq, at } = op;
      return async () => {
        try {
          await serveTool({ sessionId: session.id, agentId, origin: { kind: 'run', runSeq }, toolCallId, tool, input, at });
        } catch (err) {
          throw new Error(`prism tool serve failed for ${toolCallId}`, { cause: err });
        }
      };
    }

    case 'workflow-settle': {
      return null;
    }
  }
};

const applyWorkflowOp = async (
  tx: Transaction,
  workflow: typeof PrismWorkflows.$inferSelect,
  session: SessionRef,
  op: DomainOp,
): Promise<(() => Promise<void>) | null> => {
  switch (op.op) {
    case 'workflow-settle': {
      if (workflow.state !== 'RUNNING') return null;

      const update = { state: op.state, usage: op.usage, error: op.error, finishedAt: dayjs(op.at) };
      await tx
        .update(PrismWorkflows)
        .set(update)
        .where(and(eq(PrismWorkflows.id, workflow.id), eq(PrismWorkflows.state, 'RUNNING')));
      const settled = await prismApps[workflow.app]?.onWorkflowSettled?.(tx, { ...workflow, ...update }, { ...update, result: op.result });
      workflow.state = op.state;
      return async () => {
        await settled?.();
        pubsub.publish('prism:badge', session.userId, { sessionId: session.id });
        await ensureIngest({ kind: 'agent', sessionId: session.id });
      };
    }

    case 'tool-serve': {
      const { toolCallId, tool, input, agentId, at } = op;
      return async () => {
        try {
          await serveTool({
            sessionId: session.id,
            agentId,
            origin: { kind: 'workflow', startedAt: workflow.startedAt.valueOf() },
            toolCallId,
            tool,
            input,
            at,
          });
        } catch (err) {
          throw new Error(`prism tool serve failed for ${toolCallId}`, { cause: err });
        }
      };
    }

    default: {
      return null;
    }
  }
};

type PumpContext = {
  target: IngestTarget;
  scope: ProjectedScope;
  session: SessionRef;
  workflow: typeof PrismWorkflows.$inferSelect | null;
  signal: AbortSignal;
};

const runPump = async (ctx: PumpContext): Promise<PumpOutcome> => {
  const { target, scope, session, signal } = ctx;
  const parkedScope = target.kind;
  const live = new Map<string, TurnLive>();
  const parkedEvents = await loadParkedEvents(target);

  const policyRow = await db
    .select({ toolPolicy: PrismSessions.toolPolicy })
    .from(PrismSessions)
    .where(eq(PrismSessions.id, session.id))
    .then(firstOrThrow);

  // 서브는 커밋 뒤에 도는 thunk라 잡이 그 사이 죽으면 커서만 전진한 채 실행도 알림도 사라진다 — 기동 시 파킹된 요청을 해소 주체와 무관하게 다시 태운다
  // (원장이 중복 실행을, once-key가 중복 푸시를, prism의 pending 불일치가 이미 해소된 요청을 막는다)
  for (const request of pendingRequests(parkedEvents, parkedScope)) {
    await serveTool({
      sessionId: session.id,
      agentId: request.agentId,
      origin:
        ctx.workflow === null ? { kind: 'run', runSeq: request.runSeq } : { kind: 'workflow', startedAt: ctx.workflow.startedAt.valueOf() },
      toolCallId: request.toolCallId,
      tool: request.tool,
      input: request.input,
      at: request.at,
    });
  }

  const requireWorkflow = (): typeof PrismWorkflows.$inferSelect => {
    if (ctx.workflow === null) throw new Error('prism workflow context missing');
    return ctx.workflow;
  };

  const cursorRow =
    target.kind === 'agent'
      ? await db
          .select({ cursor: PrismSessions.cursor })
          .from(PrismSessions)
          .where(eq(PrismSessions.id, target.sessionId))
          .then(firstOrThrow)
      : await db
          .select({ cursor: PrismWorkflows.cursor })
          .from(PrismWorkflows)
          .where(eq(PrismWorkflows.id, target.workflowId))
          .then(firstOrThrow);

  const livePrefix = scope.source === 'SESSION' ? 'S|' : `W|${scope.workflowId}|`;

  let cursor = cursorRow.cursor;
  let synced = false;
  let absent = 0;
  let absentSince: number | null = null;
  let liveTtlAt = 0;
  let awaitingWritten: boolean | null = null;

  const writeAwaiting = async (waiting: boolean): Promise<void> => {
    if (waiting === awaitingWritten) return;

    const at = waiting ? dayjs() : null;
    const rows =
      target.kind === 'agent'
        ? await db
            .update(PrismSessions)
            .set({ awaitingUserAt: at })
            .where(
              and(
                eq(PrismSessions.id, target.sessionId),
                waiting ? isNull(PrismSessions.awaitingUserAt) : isNotNull(PrismSessions.awaitingUserAt),
              ),
            )
            .returning({ id: PrismSessions.id })
        : await db
            .update(PrismWorkflows)
            .set({ awaitingUserAt: at })
            .where(
              and(
                eq(PrismWorkflows.id, target.workflowId),
                waiting ? isNull(PrismWorkflows.awaitingUserAt) : isNotNull(PrismWorkflows.awaitingUserAt),
              ),
            )
            .returning({ id: PrismWorkflows.id });

    awaitingWritten = waiting;
    if (rows.length > 0) pubsub.publish('prism:badge', session.userId, { sessionId: session.id });
  };

  const syncAwaiting = async (): Promise<void> => {
    await writeAwaiting(awaitingUser(parkedEvents, parkedScope, (tool) => effectiveResolver(tool, policyRow.toolPolicy)));
  };

  const isOpen = async (): Promise<boolean> => {
    if (target.kind === 'agent') {
      const row = await db
        .select({ openRunSeq: PrismSessions.openRunSeq })
        .from(PrismSessions)
        .where(eq(PrismSessions.id, target.sessionId))
        .then(firstOrThrow);
      return row.openRunSeq !== null;
    }

    return ctx.workflow?.state === 'RUNNING';
  };

  const resolverOf = (tool: string) => effectiveResolver(tool, policyRow.toolPolicy);
  const isParked = async (): Promise<boolean> =>
    target.kind === 'agent'
      ? await agentParked(target.sessionId, parkedEvents, resolverOf)
      : await workflowParked(session.id, parkedEvents, resolverOf);

  const stopNow = async (): Promise<boolean> => synced && shouldStop({ synced, open: await isOpen(), parked: await isParked() });

  const publish = (frame: StreamFrame) => {
    try {
      const projected = projectFrame(frame, scope);
      if (projected !== null) pubsub.publish('prism:session', session.id, projected);
    } catch (err) {
      log.error('prism frame projection failed: {sessionId} (seq {cursor}) {*}', { sessionId: session.id, cursor, error: err });
    }
  };

  const onDelta = async (frame: Extract<StreamFrame, { type: 'delta' }>) => {
    const projected = projectFrame(frame, scope);
    if (projected === null || projected.type !== 'delta') return;

    const field = liveFieldKey(scope, frame.delta.context.agent.id);
    const next = applyDelta(live.get(field) ?? null, projected.delta);
    live.set(field, next);

    const pipeline = redis.pipeline().hset(liveKey(session.id), field, JSON.stringify(next));
    const now = Date.now();
    if (now - liveTtlAt >= LIVE_TTL_REFRESH_MS) {
      pipeline.expire(liveKey(session.id), LIVE_TTL_SECONDS);
      liveTtlAt = now;
    }

    await pipeline.exec();
    pubsub.publish('prism:session', session.id, projected);
  };

  const clearLive = async (fields: string[]) => {
    if (fields.length === 0) return;

    await redis.hdel(liveKey(session.id), ...fields);
    for (const field of fields) live.delete(field);
    liveTtlAt = 0;
  };

  const clearLiveScope = async () => {
    const remote = await redis.hkeys(liveKey(session.id));
    const fields = new Set(remote.filter((field) => field.startsWith(livePrefix)));
    for (const field of live.keys()) fields.add(field);
    await clearLive([...fields]);
  };

  // 누가 해소했는지는 prism 이벤트에 없다 — 원장(prism_tool_calls.resolver)이 유일한 기록이라, 저장·발행 전에 여기서 굳힌다.
  // 카드/실행 줄 판정이 현재 정책이 아니라 이 값에 매이도록【오너 2026-08-25】.
  const withResolver = async (event: EventFrame): Promise<EventFrame> => {
    if (target.kind !== 'agent' || event.kind !== 'tool.resolved' || !event.context?.toolCallId) return event;
    const resolver = await toolResolverOf(target.sessionId, event.context.toolCallId);
    return resolver === null ? event : { ...event, data: { ...event.data, resolvedBy: resolver.toLowerCase() } };
  };

  const onEvent = async (raw: EventFrame) => {
    const event = await withResolver(raw);
    const plan = planEvent(parkedScope, event, cursor);
    if (PARKED_KINDS.has(event.kind))
      parkedEvents.push({ kind: event.kind, context: event.context, data: event.data, occurredAt: event.occurredAt });

    const after: (() => Promise<void>)[] = [];
    await db.transaction(async (tx) => {
      const row = {
        seq: event.seq,
        kind: event.kind,
        occurredAt: dayjs(event.occurredAt),
        loggedAt: dayjs(event.loggedAt),
        context: event.context,
        data: event.data,
      };

      if (target.kind === 'agent') {
        await tx
          .insert(PrismSessionEvents)
          .values({ sessionId: target.sessionId, ...row })
          .onConflictDoNothing({ target: [PrismSessionEvents.sessionId, PrismSessionEvents.seq] });
      } else {
        await tx
          .insert(PrismWorkflowEvents)
          .values({ workflowId: target.workflowId, ...row })
          .onConflictDoNothing({ target: [PrismWorkflowEvents.workflowId, PrismWorkflowEvents.seq] });
      }

      if (!plan.advance) return;

      for (const op of plan.ops) {
        const wake =
          target.kind === 'agent' ? await applyAgentOp(tx, session, op) : await applyWorkflowOp(tx, requireWorkflow(), session, op);
        if (wake !== null) after.push(wake);
      }

      if (target.kind === 'agent') await tx.update(PrismSessions).set({ cursor: event.seq }).where(eq(PrismSessions.id, target.sessionId));
      else await tx.update(PrismWorkflows).set({ cursor: event.seq }).where(eq(PrismWorkflows.id, target.workflowId));
    });

    if (plan.advance) {
      cursor = event.seq;

      const context = event.context;
      if (context !== null && plan.sealTurn) {
        // 이 인스턴스가 못 본 델타(이전 펌프 인스턴스가 쓴 필드)도 봉인과 함께 지운다 — Map만 보면 stale 스냅샷이 남는다
        const fields = new Set([...live].filter(([, turn]) => sealTurn(turn, context) === null).map(([field]) => field));
        if (context.agent !== undefined) fields.add(liveFieldKey(scope, context.agent.id));
        await clearLive([...fields]);
      }

      if (plan.clearLive) await clearLiveScope();
    }

    if (parkedScope === 'agent' && event.kind === 'invocation.retried') {
      log.warn('prism invocation retried (seq {seq})', { seq: event.seq });
    }

    publish({ type: 'event', event });
    for (const wake of after) await wake();
  };

  const readGeneration = async (): Promise<number | null> => {
    if (target.kind === 'agent') {
      const state = await prism.getAgent(session.prismAgentId);
      return state.agent?.activations ?? null;
    }
    const state = await prism.getWorkflow(requireWorkflow().prismWorkflowId);
    return state.workflow.activations ?? null;
  };

  for (;;) {
    if (signal.aborted) return 'relocate';

    let delay = RECONNECT_DELAY_MS;
    try {
      const stream =
        target.kind === 'agent'
          ? await prism.openAgentEvents(session.prismAgentId, cursor, signal)
          : await prism.openWorkflowEvents(requireWorkflow().prismWorkflowId, cursor, signal);
      absent = 0;
      absentSince = null;

      const generation = await readGeneration();
      const stale = new AbortController();
      let staleReason: string | null = null;
      const watchdog = setInterval(() => {
        void readGeneration()
          .then((current) => {
            if (generation === null || current === null || current === generation || stale.signal.aborted) return;
            staleReason = `generation ${generation} -> ${current}`;
            stale.abort();
          })
          .catch(() => null);
      }, GENERATION_CHECK_MS);
      let beats = 0;

      const outcome = await pumpSse({
        stream,
        idleMs: IDLE_MS,
        signal: AbortSignal.any([signal, stale.signal]),
        onFrame: async (frame) => {
          if (frame.type === 'heartbeat') {
            beats = staleBeats(beats, frame.seq, cursor, synced);
            if (beats >= STALE_BEATS) throw new StalePump(`beacon seq ${frame.seq} > cursor ${cursor} for ${beats} beats`);
            return;
          }

          if (frame.type === 'delta') {
            await onDelta(frame);
            return;
          }

          if (frame.type === 'sync') synced = true;
          else await onEvent(frame.event);

          await syncAwaiting();

          if (await stopNow()) throw new StopPump();
        },
      })
        .catch((err: unknown) => {
          if (err instanceof StopPump) return 'stopped' as const;
          if (err instanceof StalePump) {
            staleReason = err.message;
            return 'stale' as const;
          }
          throw err;
        })
        .finally(() => clearInterval(watchdog));

      if (outcome === 'stopped') return 'done';
      if (outcome === 'aborted' && signal.aborted) return 'relocate';
      if (outcome === 'aborted' || outcome === 'stale') {
        log.warn('prism stream stale, reconnecting: {key} {logKey} (seq {cursor}) {reason}', {
          key: target.kind,
          logKey: logKeyOf(target),
          cursor,
          reason: staleReason,
        });
      } else if (outcome === 'closed' && target.kind === 'workflow' && ctx.workflow?.state !== 'RUNNING') {
        return 'done';
      } else {
        log.warn('prism stream ended before stop condition, reconnecting: {key} (seq {cursor})', { key: target.kind, cursor });
      }
    } catch (err) {
      if (signal.aborted) return 'relocate';
      if (!(err instanceof PrismApiError)) throw err;

      if (err.status === 404) {
        if (target.kind === 'agent') {
          const row = await db
            .select({ userId: PrismSessions.userId, openRunSeq: PrismSessions.openRunSeq })
            .from(PrismSessions)
            .where(eq(PrismSessions.id, target.sessionId))
            .then(firstOrThrow);
          if (row.openRunSeq !== null) {
            await closeRun(db, target.sessionId, row.openRunSeq);
            pubsub.publish('prism:credit', row.userId, {});
          }
          await writeAwaiting(false);
          log.warn('prism agent absent, closing: {sessionId}', { sessionId: target.sessionId });
          return 'done';
        }

        absent += 1;
        absentSince ??= Date.now();
        const next = absentDelay(absent, Date.now() - absentSince);
        if (next === null) {
          await writeAwaiting(false);
          log.warn('prism workflow still absent after grace, giving up: {workflowId}', { workflowId: target.workflowId });
          return 'done';
        }

        delay = next;
      } else if (err.status >= 500) {
        log.warn('prism stream reconnect after upstream failure: {code} ({status})', { code: err.code, status: err.status });
      } else {
        throw err;
      }
    }

    await sleep(delay, signal);
  }
};

const loadContext = async (target: IngestTarget, signal: AbortSignal): Promise<PumpContext | null> => {
  if (target.kind === 'agent') {
    const session = await db
      .select({
        id: PrismSessions.id,
        prismAgentId: PrismSessions.prismAgentId,
        userId: PrismSessions.userId,
        title: PrismSessions.title,
        deletedAt: PrismSessions.deletedAt,
      })
      .from(PrismSessions)
      .where(eq(PrismSessions.id, target.sessionId))
      .then(first);
    if (!session || session.deletedAt !== null) return null;

    const running = await db
      .select()
      .from(PrismWorkflows)
      .where(and(eq(PrismWorkflows.sessionId, session.id), eq(PrismWorkflows.state, 'RUNNING')));
    for (const row of running) await ensureIngest({ kind: 'workflow', workflowId: row.id });

    return { target, scope: { source: 'SESSION' }, session, workflow: null, signal };
  }

  const workflow = await db.select().from(PrismWorkflows).where(eq(PrismWorkflows.id, target.workflowId)).then(first);
  if (!workflow) return null;

  const session = await db
    .select({
      id: PrismSessions.id,
      prismAgentId: PrismSessions.prismAgentId,
      userId: PrismSessions.userId,
      title: PrismSessions.title,
      deletedAt: PrismSessions.deletedAt,
    })
    .from(PrismSessions)
    .where(eq(PrismSessions.id, workflow.sessionId))
    .then(firstOrThrow);
  if (session.deletedAt !== null) return null;

  return { target, scope: { source: 'WORKFLOW', workflowId: workflow.prismWorkflowId }, session, workflow, signal };
};

export const processIngestJob = async (
  job: Job<{ logKey: string }>,
  token: string | undefined,
  signal: AbortSignal | undefined,
): Promise<void> => {
  const target = parseLogKey(job.data.logKey);
  if (target === null) throw new Error(`invalid prism ingest log key: ${job.data.logKey}`);

  const aborted = AbortSignal.any([shutdown.signal, ...(signal ? [signal] : [])]);

  try {
    const ctx = await loadContext(target, aborted);
    if (ctx === null) return;

    const outcome = await runPump(ctx);
    if (outcome === 'relocate') {
      if (signal?.reason === LOCK_LOST) {
        // 락을 잃은 잡은 이미 다른 워커가 이어받았다 — 옮길 것도 없고 옮길 락도 없다
        log.warn('prism ingest yielded after lock loss: {logKey}', { logKey: job.data.logKey });
        return;
      }
      await job.moveToDelayed(Date.now() + 500, token);
      throw new DelayedError();
    }
  } catch (err) {
    if (!(err instanceof DelayedError)) log.error('prism ingest failed: {logKey} {*}', { logKey: job.data.logKey, error: err });

    throw err;
  }
};
