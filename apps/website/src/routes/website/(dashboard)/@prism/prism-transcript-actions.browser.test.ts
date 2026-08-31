import '../../../../app.css';

import { emptyTranscript } from '@typie/prism';
import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import PrismTranscript from './PrismTranscript.svelte';
import { reactiveProps } from './PrismTranscript.test-props.svelte.ts';
import type { Transcript, TranscriptMessage, TurnLive } from '@typie/prism';
import type { PrismAnswer, PrismPendingMessage } from './prism-chat.svelte.ts';

vi.mock('$env/dynamic/public', () => ({ env: {} }));

const user = (key: string, runSeq: number | null, text: string, at: number): TranscriptMessage => ({
  role: 'user',
  key,
  runSeq,
  text,
  at,
});

const assistant = (key: string, text: string, at: number): TranscriptMessage => ({
  role: 'assistant',
  key,
  text,
  toolCalls: [],
  at,
  streamed: false,
});

const awaitingRequest = (at: number): TranscriptMessage => ({
  role: 'tool-request',
  key: 'awaiting',
  seq: 1,
  tool: 'unknown-tool',
  toolCallId: 'call-1',
  agentId: 'agent-1',
  data: null,
  status: 'pending',
  at,
});

let component: Record<string, unknown> | undefined;

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
});

describe('PRISM transcript message actions', () => {
  it('전송 중인 사용자 메시지도 확정 전부터 같은 footer 높이를 차지한다', async () => {
    const at = Date.parse('2026-09-01T00:00:00+09:00');
    const pending = user('pending', null, '방금 보낸 메시지', at) as PrismPendingMessage;
    const awaiting = awaitingRequest(at);
    const initialTranscript: Transcript = { ...emptyTranscript(), run: 'running', messages: [awaiting] };
    const props = reactiveProps({
      transcript: initialTranscript,
      answers: [],
      loading: false,
      pending: pending as PrismPendingMessage | null,
      sessionId: 'PRSS1',
      failedIds: new Set<string>(),
      reconnecting: false,
      policy: 'STANDARD',
      onResolve: vi.fn().mockResolvedValue(undefined),
      onRetry: vi.fn(),
      onReact: vi.fn().mockResolvedValue(true),
    });

    const target = document.createElement('div');
    Object.assign(target.style, { display: 'flex', flexDirection: 'column', height: '320px', width: '420px' });
    document.body.append(target);
    component = mount(PrismTranscript, { target, props: props as never });
    await tick();

    const pendingHost = target.querySelector<HTMLElement>('[data-message-actions-host][data-message-role="user"]');
    expect(pendingHost).not.toBeNull();
    if (!pendingHost) return;
    expect(pendingHost.querySelector('[data-message-actions]')).not.toBeNull();
    const pendingHeight = pendingHost.getBoundingClientRect().height;

    props.pending = null;
    props.transcript = {
      ...emptyTranscript(),
      run: 'running',
      messages: [awaiting, user('u1', 1, pending.text, pending.at)],
    };
    await tick();

    const confirmedHost = target.querySelector<HTMLElement>('[data-message-actions-host][data-message-role="user"]');
    expect(confirmedHost).not.toBeNull();
    expect(Math.abs((confirmedHost?.getBoundingClientRect().height ?? 0) - pendingHeight)).toBeLessThan(1);
  });

  it('완료된 답변도 pacer가 마지막 텍스트를 마친 뒤에 footer를 표시한다', async () => {
    const at = Date.parse('2026-09-01T00:00:00+09:00');
    const text = '마지막 문장까지 차분하게 표시한 다음 메시지 동작을 보여 줍니다. '.repeat(3);
    const awaiting = awaitingRequest(at);
    const question = user('u1', 1, '질문', at);
    const live: TurnLive = {
      context: { agent: { id: 'agent-1', name: 'assistant' }, run: 1, turn: 1, attempt: 1 },
      text,
      textBroken: false,
      thinkingChars: 0,
      toolInput: null,
      last: 'text',
      seeded: false,
    };
    const initialTranscript: Transcript = {
      ...emptyTranscript(),
      messages: [awaiting, question],
      run: 'running',
      turn: 'active',
      live,
    };
    const props = reactiveProps({
      transcript: initialTranscript,
      answers: [
        {
          key: 'a1',
          run: { id: 'PRRN1', runSeq: 1, state: 'COMPLETED', reaction: null, reactionNote: null },
        },
      ] satisfies PrismAnswer[],
      loading: false,
      pending: null as PrismPendingMessage | null,
      sessionId: 'PRSS1',
      failedIds: new Set<string>(),
      reconnecting: false,
      policy: 'STANDARD',
      onResolve: vi.fn().mockResolvedValue(undefined),
      onRetry: vi.fn(),
      onReact: vi.fn().mockResolvedValue(true),
    });

    const target = document.createElement('div');
    Object.assign(target.style, { display: 'flex', flexDirection: 'column', height: '320px', width: '420px' });
    document.body.append(target);
    component = mount(PrismTranscript, { target, props: props as never });
    await vi.waitFor(() => expect(target.querySelector('[data-role="assistant"]')).not.toBeNull());

    props.transcript = {
      ...emptyTranscript(),
      messages: [awaiting, question, assistant('a1', text, at + 1)],
    };
    await tick();

    const answerHost = target.querySelector('[data-message-actions-host][data-message-role="assistant"]');
    expect(answerHost).not.toBeNull();
    expect(answerHost?.querySelector(':scope > [data-message-actions]')).toBeNull();
  });

  it('사용자와 완료 답변의 액션·시간·반응 note를 메시지 위치와 상태에 맞게 표시한다', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText } });
    const previousYear = new Date().getFullYear() - 1;
    const oldMessageAt = new Date(previousYear, 7, 30, 10, 0).getTime();

    const messages = [
      user('u1', 1, '첫 질문', oldMessageAt),
      assistant('a1', '첫 답변', Date.parse('2026-08-31T10:01:00+09:00')),
      user('u2', 2, '둘째 질문', Date.parse('2026-08-31T10:02:00+09:00')),
      assistant('a2', '둘째 답변', Date.parse('2026-08-31T10:03:00+09:00')),
    ];
    const transcript: Transcript = { ...emptyTranscript(), messages };
    const answers: PrismAnswer[] = [
      {
        key: 'a1',
        run: { id: 'PRRN1', runSeq: 1, state: 'COMPLETED', reaction: 'UP', reactionNote: '도움 됐어요' },
      },
      {
        key: 'a2',
        run: { id: 'PRRN2', runSeq: 2, state: 'COMPLETED', reaction: null, reactionNote: null },
      },
    ];
    const noteSubmit = Promise.withResolvers<boolean>();
    const onReact = vi.fn(async (runId: string, reaction: 'UP' | 'DOWN' | null, note: string | null) => {
      if (note !== null) await noteSubmit.promise;
      props.answers = props.answers.map((answer) =>
        answer.run.id === runId ? { ...answer, run: { ...answer.run, reaction, reactionNote: reaction === null ? null : note } } : answer,
      );
      await tick();
      return true;
    });
    const props = reactiveProps({
      transcript,
      answers,
      loading: false,
      pending: null,
      sessionId: 'PRSS1',
      failedIds: new Set<string>(),
      reconnecting: false,
      policy: 'STANDARD',
      onResolve: vi.fn().mockResolvedValue(undefined),
      onRetry: vi.fn(),
      onReact,
    });

    const target = document.createElement('div');
    Object.assign(target.style, { display: 'flex', flexDirection: 'column', height: '640px', width: '420px' });
    document.body.append(target);
    component = mount(PrismTranscript, { target, props: props as never });
    await tick();

    const userHosts = [...target.querySelectorAll<HTMLElement>('[data-message-actions-host][data-message-role="user"]')];
    const assistantHosts = [...target.querySelectorAll<HTMLElement>('[data-message-actions-host][data-message-role="assistant"]')];
    expect(userHosts).toHaveLength(2);
    expect(assistantHosts).toHaveLength(2);
    const firstUserHost = userHosts[0];
    const latestAssistantHost = assistantHosts[1];
    if (!firstUserHost || !latestAssistantHost) return;

    const userActions = userHosts[0]?.querySelector<HTMLElement>('[data-message-actions]');
    const previousActions = assistantHosts[0]?.querySelector<HTMLElement>('[data-message-actions]');
    const latestActions = assistantHosts[1]?.querySelector<HTMLElement>('[data-message-actions]');
    expect(userActions).not.toBeNull();
    expect(previousActions).not.toBeNull();
    expect(latestActions).not.toBeNull();
    if (!userActions || !previousActions || !latestActions) return;

    expect(getComputedStyle(userActions).opacity).toBe('0');
    expect(getComputedStyle(previousActions).opacity).toBe('0');
    await vi.waitFor(() => expect(getComputedStyle(latestActions).opacity).toBe('1'));
    expect(getComputedStyle(latestActions.querySelector('[data-message-time]') as Element).opacity).toBe('0');

    const previousNote = assistantHosts[0]?.querySelector<HTMLElement>('[data-reaction-note-container]');
    expect(previousNote).not.toBeNull();
    expect(previousNote?.getBoundingClientRect().height).toBe(0);
    await userEvent.hover(assistantHosts[0] as HTMLElement);
    await vi.waitFor(() => expect((previousNote?.getBoundingClientRect().height ?? 0) > 0).toBe(true));

    await userEvent.hover(firstUserHost);
    await vi.waitFor(() => expect(getComputedStyle(userActions).opacity).toBe('1'));
    const userTime = userActions.querySelector('time');
    const userCopy = userActions.querySelector<HTMLButtonElement>('[aria-label="복사"]');
    expect(userTime?.nextElementSibling).toBe(userCopy);
    expect(userTime?.textContent).toContain(`${previousYear}년 8월 30일`);
    userCopy?.click();
    await vi.waitFor(() => expect(writeText).toHaveBeenCalledWith('첫 질문'));
    await vi.waitFor(() => expect(userActions.querySelector('[aria-label="복사됨"]')).not.toBeNull());

    const latestTime = latestActions.querySelector('time');
    const latestButtons = latestActions.querySelectorAll('button');
    expect(latestButtons).toHaveLength(3);
    expect(latestButtons[0]?.nextElementSibling).toBe(latestButtons[1]);
    expect(latestButtons[1]?.nextElementSibling).toBe(latestButtons[2]);
    expect(latestButtons[2]?.nextElementSibling).toBe(latestTime);

    await userEvent.hover(latestAssistantHost);
    await vi.waitFor(() => expect(getComputedStyle(latestTime as Element).opacity).toBe('1'));
    const down = latestActions.querySelector<HTMLButtonElement>('[aria-label="아쉬웠어요"]');
    down?.click();
    await tick();
    expect(onReact).toHaveBeenLastCalledWith('PRRN2', 'DOWN', null);

    const noteShell = latestAssistantHost.querySelector<HTMLElement>('[data-reaction-note]');
    expect(noteShell).not.toBeNull();
    expect(noteShell?.dataset.reactionNoteState).toBe('draft');
    const note = latestAssistantHost.querySelector<HTMLTextAreaElement>('[placeholder="몇 자 덧붙이기"]');
    expect(note).not.toBeNull();
    if (!note) return;
    await vi.waitFor(() => expect(document.activeElement).toBe(note));
    await userEvent.fill(note, '조금 더 간결하면 좋아요');
    await userEvent.keyboard('{Enter}');
    const submit = noteShell?.querySelector<HTMLButtonElement>('[aria-label="남기는 중"]');
    await vi.waitFor(() => expect(submit?.getAttribute('aria-busy')).toBe('true'));
    expect(note.readOnly).toBe(true);
    expect(submit?.querySelector('[data-reaction-note-submit-spinner]')).not.toBeNull();
    expect(onReact).toHaveBeenLastCalledWith('PRRN2', 'DOWN', '조금 더 간결하면 좋아요');
    noteSubmit.resolve(true);
    await tick();
    await vi.waitFor(() => expect(latestAssistantHost.querySelector('[data-reaction-note-state="saved"]')).not.toBeNull());
    const savedNote = latestAssistantHost.querySelector<HTMLElement>('[data-reaction-note-state="saved"]');
    const savedText = savedNote?.querySelector('[data-reaction-note-text]');
    const edit = savedNote?.querySelector('[aria-label="반응 메모 수정"]');
    expect(savedText?.textContent).toBe('조금 더 간결하면 좋아요');
    expect(edit).not.toBeNull();
  });
});
