import { applyFrame, emptyTranscript } from '@typie/prism';
import { describe, expect, it, vi } from 'vitest';
import { createPrismChat } from './prism-chat.svelte.ts';
import type { ProjectedEventData, ProjectedStreamFrame, Transcript } from '@typie/prism';
import type { PrismChatDeps } from './prism-chat.svelte.ts';

const agent = { id: 'typie-1', name: 'assistant' };

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

const invocation = (seq: number, id: string) => ({ agent, run: 1, turn: 1, attempt: 1, toolCallId: `c${seq}`, invocation: id });

const startWorkflow = (seq: number, workflowId: string, id: string) =>
  ev(seq, 'invocation.started', invocation(seq, id), { target: { kind: 'workflow', id: workflowId, name: 'high', app: 'app_1' } });

const finishWorkflow = (seq: number, id: string) => ev(seq, 'invocation.completed', invocation(seq, id), {});

const sessionLog = [startWorkflow(1, 'workflow_1', 'i1'), startWorkflow(2, 'workflow_2', 'i2'), finishWorkflow(3, 'i2')];

const deps = (over: Partial<PrismChatDeps>): PrismChatDeps => ({
  load: vi.fn().mockResolvedValue(sessionLog.reduce((transcript, frame) => applyFrame(transcript, frame), emptyTranscript())),
  send: vi.fn(),
  cancel: vi.fn(),
  ...over,
});

describe('createPrismChat.load', () => {
  it('transcript를 그대로 싣고 seedCursor를 transcript.cursor로 둔다', async () => {
    const chat = createPrismChat(deps({}));

    await chat.load('PRSS1');

    expect(chat.transcript.messages.map((message) => message.role)).toEqual(['workflow', 'workflow']);
    expect(chat.seedCursor).toBe(3);
    expect(chat.loading).toBe(false);
    expect(chat.error).toBeNull();
  });

  it('같은 세션 재호출은 no-op이고, 실패는 에러 문면을 세운다', async () => {
    const load = vi.fn().mockRejectedValueOnce(new Error('x')).mockResolvedValue(emptyTranscript());
    const chat = createPrismChat(deps({ load }));

    await chat.load('PRSS1');
    expect(chat.error).toBe('대화를 불러오지 못했어요. 잠시 후 다시 시도해 주세요');

    await chat.load('PRSS1');
    expect(chat.error).toBeNull();
    expect(load).toHaveBeenCalledTimes(2);

    await chat.load('PRSS1');
    expect(load).toHaveBeenCalledTimes(2);
  });

  it('늦게 끝난 이전 로드는 무시된다', async () => {
    let resolveFirst!: (transcript: Transcript) => void;
    const load = vi
      .fn()
      .mockImplementationOnce(() => new Promise<Transcript>((resolve) => (resolveFirst = resolve)))
      .mockResolvedValueOnce({ ...emptyTranscript(), cursor: 9 });
    const chat = createPrismChat(deps({ load }));

    const first = chat.load('PRSS1');
    await chat.load('PRSS2');
    resolveFirst({ ...emptyTranscript(), cursor: 1 });
    await first;

    expect(chat.sessionId).toBe('PRSS2');
    expect(chat.seedCursor).toBe(9);
  });

  it('새 대화로 전환하면 아직 스트림에 합류하지 않은 사용자 메시지를 비운다', async () => {
    const chat = createPrismChat(deps({ send: vi.fn().mockResolvedValue({ sessionId: 'PRSS1', runSeq: 1 }) }));

    await chat.send('안녕');
    expect(chat.pending).toBe('안녕');

    await chat.load(null);

    expect(chat.sessionId).toBeNull();
    expect(chat.pending).toBeNull();
  });

  it('새 대화 전환 뒤 끝난 이전 전송은 현재 세션을 되돌리지 않는다', async () => {
    let resolveSend!: (value: { sessionId: string; runSeq: number }) => void;
    const send = vi.fn().mockImplementation(() => new Promise((resolve) => (resolveSend = resolve)));
    const chat = createPrismChat(deps({ send }));

    const sending = chat.send('안녕');
    await chat.load(null);
    resolveSend({ sessionId: 'PRSS1', runSeq: 1 });
    await sending;

    expect(chat.sessionId).toBeNull();
    expect(chat.pending).toBeNull();
  });
});

describe('createPrismChat.receive', () => {
  it('프레임을 리듀서에 적용하고 run.started에서 pending을 지운다', async () => {
    const chat = createPrismChat(deps({ send: vi.fn().mockResolvedValue({ sessionId: 'PRSS1', runSeq: 1 }) }));

    await chat.send('안녕');
    expect(chat.pending).toBe('안녕');

    chat.receive(ev(1, 'run.started', { agent, run: 1 }, { message: '안녕', command: null }));

    expect(chat.pending).toBeNull();
    expect(chat.transcript.messages[0]).toMatchObject({ role: 'user', text: '안녕' });
  });
});
