import { describe, expect, it, vi } from 'vitest';
import { createPrismChat } from './prism-chat.svelte.ts';
import type { ProjectedEventData, ProjectedStreamFrame } from '@typie/prism';
import type { PrismChatDeps } from './prism-chat.svelte.ts';

const agent = { id: 'typie-1', name: 'assistant' };
const child = { id: 'agent_9', name: 'judgment-high' };

const ev = (seq: number, kind: string, context: Record<string, unknown>, data: Record<string, unknown>): ProjectedStreamFrame => ({
  type: 'event',
  event: {
    seq,
    occurredAt: 1000 + seq,
    context: context as never,
    source: 'SESSION',
    ...({ kind, data } as ProjectedEventData),
  },
});

const wf = (
  seq: number,
  kind: string,
  context: Record<string, unknown>,
  data: Record<string, unknown>,
  workflowId: string,
): ProjectedStreamFrame => ({
  type: 'event',
  event: {
    seq,
    occurredAt: 2000 + seq,
    context: context as never,
    source: 'WORKFLOW',
    workflowId,
    ...({ kind, data } as ProjectedEventData),
  },
});

const invocation = (seq: number, id: string) => ({ agent, run: 1, turn: 1, attempt: 1, toolCallId: `c${seq}`, invocation: id });

const startWorkflow = (seq: number, workflowId: string, id: string) =>
  ev(seq, 'invocation.started', invocation(seq, id), { target: { kind: 'workflow', id: workflowId, name: 'high', app: 'app_1' } });

const finishWorkflow = (seq: number, id: string) => ev(seq, 'invocation.completed', invocation(seq, id), {});

const stepStarted = (seq: number, workflowId: string) => wf(seq, 'step.started', { agent: child, step: 'classify-0' }, {}, workflowId);

const sessionLog = [startWorkflow(1, 'workflow_1', 'i1'), startWorkflow(2, 'workflow_2', 'i2'), finishWorkflow(3, 'i2')];

const deps = (over: Partial<PrismChatDeps>): PrismChatDeps => ({
  loadLog: vi.fn().mockResolvedValue(sessionLog),
  loadWorkflowLog: vi.fn().mockResolvedValue([]),
  send: vi.fn(),
  cancel: vi.fn(),
  ...over,
});

const workflowOf = (chat: ReturnType<typeof createPrismChat>, workflowId: string) => {
  const message = chat.transcript.messages.find((entry) => entry.role === 'workflow' && entry.workflowId === workflowId);
  if (message === undefined || message.role !== 'workflow') throw new Error(`workflow ${workflowId} not seeded`);
  return message;
};

describe('createPrismChat.load', () => {
  it('세션 로그 뒤에 진행 중 워크플로의 로그만 읽어 같은 transcript에 적용한 뒤 loading을 푼다', async () => {
    const loadWorkflowLog = vi.fn().mockImplementation((workflowId: string) => Promise.resolve([stepStarted(1, workflowId)]));
    const chat = createPrismChat(deps({ loadWorkflowLog }));

    await chat.load('session_1');

    expect(loadWorkflowLog).toHaveBeenCalledTimes(1);
    expect(loadWorkflowLog).toHaveBeenCalledWith('workflow_1');
    expect(workflowOf(chat, 'workflow_1').trace.steps).toHaveLength(1);
    expect(workflowOf(chat, 'workflow_1').cursor).toBe(1);
    expect(workflowOf(chat, 'workflow_2').trace.steps).toHaveLength(0);
    expect(chat.loading).toBe(false);
    expect(chat.error).toBeNull();
  });

  it('워크플로 로그 읽기가 실패해도 세션은 열리고 그 워크플로는 커서 0으로 남는다', async () => {
    const chat = createPrismChat(deps({ loadWorkflowLog: vi.fn().mockRejectedValue(new Error('down')) }));

    await chat.load('session_1');

    expect(chat.error).toBeNull();
    expect(chat.loading).toBe(false);
    expect(workflowOf(chat, 'workflow_1').cursor).toBe(0);
  });

  it('워크플로 로그를 기다리는 동안 세션이 바뀌면 그 결과는 버린다', async () => {
    let release!: (frames: ProjectedStreamFrame[]) => void;
    const pending = new Promise<ProjectedStreamFrame[]>((resolve) => (release = resolve));
    const chat = createPrismChat(deps({ loadWorkflowLog: vi.fn().mockReturnValue(pending) }));

    const first = chat.load('session_1');
    await Promise.resolve();
    await Promise.resolve();
    await chat.load(null);
    release([stepStarted(1, 'workflow_1')]);
    await first;

    expect(chat.sessionId).toBeNull();
    expect(chat.transcript.messages).toHaveLength(0);
  });
});
