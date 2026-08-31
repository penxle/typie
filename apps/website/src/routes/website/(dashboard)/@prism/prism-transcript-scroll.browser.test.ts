import '../../../../app.css';

import { emptyTranscript } from '@typie/prism';
import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import PrismTranscript from './PrismTranscript.svelte';
import { reactiveProps } from './PrismTranscript.test-props.svelte.ts';
import type { Transcript, TranscriptMessage } from '@typie/prism';
import type { PrismPendingMessage } from './prism-chat.svelte.ts';

vi.mock('$env/dynamic/public', () => ({ env: {} }));

const userMessage = (index: number): TranscriptMessage => ({
  role: 'user',
  key: `user-${index}`,
  text: `message ${index} `.repeat(12),
  at: index,
  runSeq: null,
});

const transcriptWith = (messages: TranscriptMessage[]): Transcript => ({ ...emptyTranscript(), messages });
const bottomGap = (element: HTMLElement) => element.scrollHeight - element.scrollTop - element.clientHeight;
const nextFrame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
const waitForBottomGap = async (element: HTMLElement, maximum: number) => {
  for (let frame = 0; frame < 120 && bottomGap(element) > maximum; frame++) await nextFrame();
  expect(bottomGap(element)).toBeLessThanOrEqual(maximum);
};

let component: Record<string, unknown> | undefined;

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
});

describe('PRISM transcript bottom follow', () => {
  it('chases the actual bottom whenever the go-to-bottom button is absent', async () => {
    const messages = Array.from({ length: 30 }, (_, index) => userMessage(index));
    const props = reactiveProps({
      transcript: transcriptWith(messages),
      answers: [],
      loading: false,
      pending: null as PrismPendingMessage | null,
      sessionId: 'session-1' as string | null,
      failedIds: new Set<string>(),
      reconnecting: false,
      policy: 'STANDARD' as const,
      onResolve: vi.fn().mockResolvedValue(undefined),
      onRetry: vi.fn(),
      onReact: vi.fn().mockResolvedValue(true),
    });
    const target = document.createElement('div');
    Object.assign(target.style, { display: 'flex', flexDirection: 'column', height: '320px', width: '360px' });
    document.body.append(target);
    component = mount(PrismTranscript, { target, props });

    await tick();
    const scroller = target.firstElementChild?.firstElementChild;
    expect(scroller).toBeInstanceOf(HTMLElement);
    if (!(scroller instanceof HTMLElement)) return;
    await waitForBottomGap(scroller, 1);
    expect(scroller.scrollHeight).toBeGreaterThan(scroller.clientHeight);

    scroller.dispatchEvent(new Event('scroll'));
    scroller.scrollTop = scroller.scrollHeight - scroller.clientHeight - 40;
    scroller.dispatchEvent(new Event('scroll'));
    await tick();
    expect(target.querySelector('[aria-label="아래로"]')).not.toBeNull();

    scroller.scrollTop = scroller.scrollHeight - scroller.clientHeight - 7;
    scroller.dispatchEvent(new Event('scroll'));
    await tick();
    await vi.waitFor(() => expect(target.querySelector('[aria-label="아래로"]')).toBeNull());
    await waitForBottomGap(scroller, 1);

    const previousScrollHeight = scroller.scrollHeight;
    props.transcript = transcriptWith([...messages, userMessage(messages.length)]);
    await tick();
    await vi.waitFor(() => expect(scroller.scrollHeight).toBeGreaterThan(previousScrollHeight));
    await waitForBottomGap(scroller, 7);
    expect(target.querySelector('[aria-label="아래로"]')).toBeNull();

    scroller.scrollTop = scroller.scrollHeight - scroller.clientHeight - 40;
    scroller.dispatchEvent(new Event('scroll'));
    await tick();
    const button = target.querySelector('[aria-label="아래로"]');
    expect(button).toBeInstanceOf(HTMLButtonElement);
    if (!(button instanceof HTMLButtonElement)) return;
    button.click();
    await waitForBottomGap(scroller, 1);
    await vi.waitFor(() => expect(target.querySelector('[aria-label="아래로"]')).toBeNull());
  });
});
