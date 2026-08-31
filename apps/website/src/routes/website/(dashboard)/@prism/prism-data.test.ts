import { describe, expect, it, vi } from 'vitest';
import { toRunItem } from './prism-data.ts';

vi.mock('$env/dynamic/public', () => ({ env: {} }));

describe('toRunItem', () => {
  it('PrismUserMessage — userText 별칭을 text로 옮긴다', () => {
    expect(toRunItem({ __typename: 'PrismUserMessage', key: 'u1', userText: '안녕', at: '2026-08-24T00:00:00.000Z' })).toEqual({
      kind: 'user',
      key: 'u1',
      text: '안녕',
      at: '2026-08-24T00:00:00.000Z',
    });
  });

  it('PrismAssistantMessage — assistantText 별칭과 null 텍스트, 도구 호출 참조', () => {
    expect(
      toRunItem({
        __typename: 'PrismAssistantMessage',
        key: 'a1',
        assistantText: null,
        streamed: true,
        toolCalls: [{ id: 'tc1', name: 'read' }],
        at: '2026-08-24T00:00:01.000Z',
      }),
    ).toEqual({
      kind: 'assistant',
      key: 'a1',
      text: null,
      toolCalls: [{ id: 'tc1', name: 'read' }],
      at: '2026-08-24T00:00:01.000Z',
      streamed: true,
    });
  });

  it('PrismToolCall — ok 부재는 null', () => {
    expect(
      toRunItem({ __typename: 'PrismToolCall', key: 't1', name: 'read', phase: 'EXECUTED', ok: null, at: '2026-08-24T00:00:02.000Z' }),
    ).toEqual({ kind: 'tool', key: 't1', name: 'read', phase: 'EXECUTED', ok: null, at: '2026-08-24T00:00:02.000Z' });
  });

  it('PrismToolRequest — requestStatus 별칭과 nullable 5종', () => {
    expect(
      toRunItem({
        __typename: 'PrismToolRequest',
        key: 'r1',
        seq: 7,
        tool: 'ask-user',
        toolCallId: 'tc2',
        agentId: 'chat-1',
        workflowId: null,
        data: null,
        requestStatus: 'PENDING',
        result: null,
        resolvedBy: null,
        settledAt: null,
        at: '2026-08-24T00:00:03.000Z',
      }),
    ).toEqual({
      kind: 'toolRequest',
      key: 'r1',
      seq: 7,
      tool: 'ask-user',
      toolCallId: 'tc2',
      agentId: 'chat-1',
      workflowId: null,
      data: null,
      status: 'PENDING',
      result: null,
      resolvedBy: null,
      settledAt: null,
      at: '2026-08-24T00:00:03.000Z',
    });
  });

  it('PrismWorkflowRef — workflowStatus 별칭과 중첩 기록의 nullable 경계', () => {
    expect(
      toRunItem({
        __typename: 'PrismWorkflowRef',
        key: 'w1',
        prismWorkflowId: 'wf_1',
        app: 'review',
        name: 'high',
        workflowStatus: 'RUNNING',
        startedAt: '2026-08-24T00:00:04.000Z',
        finishedAt: null,
        cursor: 3,
        invocation: null,
        transcript: {
          steps: [{ name: 'plan', seq: 1, startedAt: '2026-08-24T00:00:05.000Z', completedAt: null }],
          turns: [{ seq: 2, step: null, text: '초안', at: '2026-08-24T00:00:06.000Z' }],
          tools: [{ seq: 3, step: null, tool: 'read', ok: true, path: null, query: null, at: '2026-08-24T00:00:07.000Z' }],
        },
      }),
    ).toEqual({
      kind: 'workflow',
      key: 'w1',
      prismWorkflowId: 'wf_1',
      app: 'review',
      name: 'high',
      status: 'RUNNING',
      startedAt: '2026-08-24T00:00:04.000Z',
      finishedAt: null,
      cursor: 3,
      invocation: null,
      transcript: {
        steps: [{ name: 'plan', seq: 1, startedAt: '2026-08-24T00:00:05.000Z', completedAt: null }],
        turns: [{ seq: 2, step: null, text: '초안', at: '2026-08-24T00:00:06.000Z' }],
        tools: [{ seq: 3, step: null, tool: 'read', ok: true, path: null, query: null, at: '2026-08-24T00:00:07.000Z' }],
      },
    });
  });

  it('PrismRunFailure — 키와 시각만 옮긴다', () => {
    expect(toRunItem({ __typename: 'PrismRunFailure', key: 'f1', at: '2026-08-24T00:00:08.000Z' })).toEqual({
      kind: 'runFailure',
      key: 'f1',
      at: '2026-08-24T00:00:08.000Z',
    });
  });
});
