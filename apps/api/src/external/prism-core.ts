import { AgentStateSchema, WorkflowStateSchema } from '@typie/prism';
import { z } from 'zod';
import { readUntilSync } from './prism-stream.ts';
import type { AgentState, EventFrame, RunSummary, WorkflowState } from '@typie/prism';

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

export const newAgentId = (): string => `chat-${crypto.randomUUID()}`;

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
const workflowPath = (workflowId: string) => `/workflows/${encodeURIComponent(workflowId)}`;

const WrittenSchema = z.object({ written: z.array(z.object({ path: z.string(), bytes: z.number(), sha256: z.string() })) });

export type PrismCommand = { name: string; description: string; argumentHint: string | null };

const CatalogSchema = z.object({
  agents: z.record(
    z.string(),
    z.object({ commands: z.record(z.string(), z.object({ description: z.string(), argumentHint: z.string().nullable() })).optional() }),
  ),
});

export const createPrismClient = (http: PrismHttp) => {
  const openEvents = async (path: string, cursor: number, signal: AbortSignal): Promise<ReadableStream<Uint8Array>> => {
    const res = await expectOk(await http.request(path, { signal, stream: true, headers: { 'last-event-id': String(cursor) } }));
    if (!res.body) throw new PrismApiError('internal', res.status);
    return res.body;
  };
  const openAgentEvents = (agentId: string, cursor: number, signal: AbortSignal): Promise<ReadableStream<Uint8Array>> =>
    openEvents(`${agentPath(agentId)}/events`, cursor, signal);
  const openWorkflowEvents = (workflowId: string, cursor: number, signal: AbortSignal): Promise<ReadableStream<Uint8Array>> =>
    openEvents(`${workflowPath(workflowId)}/events`, cursor, signal);
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
    async writeAgentFiles(agentId: string, files: { path: string; content: string }[]): Promise<z.output<typeof WrittenSchema>> {
      const res = await expectOk(await http.request(`${agentPath(agentId)}/files`, { method: 'PUT', body: { files } }));
      return jsonOf(res, WrittenSchema);
    },
    async resolveTool(agentId: string, toolCallId: string, result: unknown): Promise<void> {
      await expectOk(
        await http.request(`${agentPath(agentId)}/tools/${encodeURIComponent(toolCallId)}/result`, { method: 'POST', body: { result } }),
      );
    },
    async getWorkflow(workflowId: string): Promise<WorkflowState> {
      return jsonOf(await expectOk(await http.request(workflowPath(workflowId))), WorkflowStateSchema);
    },
    async cancelWorkflow(workflowId: string): Promise<void> {
      await expectOk(await http.request(`${workflowPath(workflowId)}/cancel`, { method: 'POST' }));
    },
    async getCatalog(): Promise<{ commands: PrismCommand[] | null }> {
      const catalog = await jsonOf(await expectOk(await http.request(`/apps/${PRISM_CONVERSATION.app}/catalog`)), CatalogSchema);
      const commands = catalog.agents[PRISM_CONVERSATION.agent]?.commands;
      if (commands === undefined) return { commands: null };
      return {
        commands: Object.entries(commands).map(([name, def]) => ({ name, description: def.description, argumentHint: def.argumentHint })),
      };
    },
    openAgentEvents,
    openWorkflowEvents,
    async readAgentEventsUntilSync(agentId: string, cursor: number, signal: AbortSignal): Promise<{ events: EventFrame[]; sync: number }> {
      return readUntilSync(await openAgentEvents(agentId, cursor, signal), signal);
    },
    async readWorkflowEventsUntilSync(
      workflowId: string,
      cursor: number,
      signal: AbortSignal,
    ): Promise<{ events: EventFrame[]; sync: number }> {
      return readUntilSync(await openWorkflowEvents(workflowId, cursor, signal), signal);
    },
  };
};
