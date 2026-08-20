import { AgentStateSchema } from '@typie/prism';
import { z } from 'zod';
import { readUntilSync } from './prism-stream.ts';
import type { AgentState, EventFrame, RunSummary } from '@typie/prism';

export class PrismApiError extends Error {
  readonly code: string;
  readonly status: number;
  constructor(code: string, status: number) {
    super(`prism-api ${status}: ${code}`);
    this.name = 'PrismApiError';
    this.code = code;
    this.status = status;
  }
}

export const PRISM_CONVERSATION = { app: 'assistant', agent: 'chat' } as const;

export const newAgentId = (): string => `typie-${crypto.randomUUID()}`;

export const sessionTitleFrom = (message: string): string | null => {
  const title = message.replaceAll(/\s+/g, ' ').trim().slice(0, 40);
  return title.length === 0 ? null : title;
};

export const activeRun = (runs: RunSummary[]): RunSummary | null => runs.findLast((run) => run.status === 'running') ?? null;

const ErrorBodySchema = z.object({ error: z.string() });
const RunSeqSchema = z.object({ runSeq: z.number() });

export const mapPrismError = (status: number, body: unknown): PrismApiError => {
  const parsed = ErrorBodySchema.safeParse(body);
  return new PrismApiError(parsed.success ? parsed.data.error : 'internal', status);
};

export type PrismHttp = {
  request(
    path: string,
    init?: { method?: string; body?: unknown; signal?: AbortSignal; stream?: boolean; headers?: Record<string, string> },
  ): Promise<{ status: number; json(): Promise<unknown>; body: ReadableStream<Uint8Array> | null }>;
};

const expectOk = async <T extends { status: number; json(): Promise<unknown> }>(res: T): Promise<T> => {
  if (res.status >= 200 && res.status < 300) return res;
  throw mapPrismError(res.status, await res.json().catch(() => null));
};

const jsonOf = async <S extends z.ZodType>(res: { status: number; json(): Promise<unknown> }, schema: S): Promise<z.output<S>> => {
  const parsed = schema.safeParse(await res.json().catch(() => null));
  if (!parsed.success) throw new PrismApiError('malformed-response', res.status);
  return parsed.data;
};

const agentPath = (agentId: string) => `/agents/${encodeURIComponent(agentId)}`;

export const createPrismClient = (http: PrismHttp) => {
  const openAgentEvents = async (agentId: string, cursor: number, signal: AbortSignal): Promise<ReadableStream<Uint8Array>> => {
    const res = await expectOk(
      await http.request(`${agentPath(agentId)}/events`, { signal, stream: true, headers: { 'last-event-id': String(cursor) } }),
    );
    if (!res.body) throw new PrismApiError('internal', res.status);
    return res.body;
  };
  return {
    async invokeAgent(opts: {
      agentId: string;
      message: string;
      key: string;
      metadata: Record<string, unknown>;
    }): Promise<{ runSeq: number }> {
      const res = await expectOk(
        await http.request('/agents', {
          method: 'POST',
          body: { agentId: opts.agentId, ...PRISM_CONVERSATION, message: opts.message, key: opts.key, metadata: opts.metadata },
        }),
      );
      return jsonOf(res, RunSeqSchema);
    },
    async resumeAgent(agentId: string, opts: { message: string; key: string }): Promise<{ runSeq: number }> {
      const res = await expectOk(await http.request(`${agentPath(agentId)}/resume`, { method: 'POST', body: opts }));
      return jsonOf(res, RunSeqSchema);
    },
    async cancelAgentRun(agentId: string, runSeq: number): Promise<void> {
      await expectOk(await http.request(`${agentPath(agentId)}/cancel`, { method: 'POST', body: { runSeq } }));
    },
    async getAgent(agentId: string): Promise<AgentState> {
      return jsonOf(await expectOk(await http.request(agentPath(agentId))), AgentStateSchema);
    },
    openAgentEvents,
    async readAgentEventsUntilSync(agentId: string, cursor: number, signal: AbortSignal): Promise<{ events: EventFrame[]; sync: number }> {
      return readUntilSync(await openAgentEvents(agentId, cursor, signal), signal);
    },
  };
};
