import '../../../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import PrismComposer from './PrismComposer.svelte';
import { reactiveProps } from './PrismTranscript.test-props.svelte.ts';

let component: Record<string, unknown> | undefined;

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
});

describe('PRISM composer submission lifetime', () => {
  it('이전 generation의 전송이 끝나지 않아도 새 대화에서 제출한다', async () => {
    const firstSend = Promise.withResolvers<boolean>();
    const secondSend = Promise.withResolvers<boolean>();
    const onSend = vi
      .fn()
      .mockImplementationOnce(() => firstSend.promise)
      .mockImplementationOnce(() => secondSend.promise);
    const props = reactiveProps({
      generation: 0,
      running: false,
      sendDisabled: false,
      blocked: false,
      commands: null,
      status: null,
      policy: { current: 'STANDARD' as const, onChange: vi.fn() },
      onSend,
      onStop: vi.fn().mockResolvedValue(undefined),
      text: '',
    });
    const target = document.createElement('div');
    document.body.append(target);
    component = mount(PrismComposer, { target, props: props as never });

    const composer = target.querySelector<HTMLTextAreaElement>('[placeholder="메시지를 입력하세요"]');
    const send = target.querySelector<HTMLButtonElement>('[aria-label="보내기"]');
    expect(composer).not.toBeNull();
    expect(send).not.toBeNull();
    if (!composer || !send) return;

    await userEvent.fill(composer, '첫 메시지');
    await userEvent.click(send);
    await vi.waitFor(() => expect(onSend).toHaveBeenCalledTimes(1));
    expect(send.disabled).toBe(true);

    props.generation = 1;
    await tick();
    await userEvent.fill(composer, '둘째 메시지');
    await vi.waitFor(() => expect(send.disabled).toBe(false));
    await userEvent.click(send);

    expect(onSend).toHaveBeenCalledTimes(2);
    expect(onSend).toHaveBeenLastCalledWith('둘째 메시지');
    await userEvent.fill(composer, '셋째 메시지');
    expect(send.disabled).toBe(true);

    firstSend.resolve(true);
    await firstSend.promise;
    await tick();
    expect(send.disabled).toBe(true);

    secondSend.resolve(true);
    await vi.waitFor(() => expect(send.disabled).toBe(false));
  });

  it('이전 generation의 실패한 메시지를 새 대화에 복원하지 않는다', async () => {
    const firstSend = Promise.withResolvers<boolean>();
    const props = reactiveProps({
      generation: 0,
      running: false,
      sendDisabled: false,
      blocked: false,
      commands: null,
      status: null,
      policy: { current: 'STANDARD' as const, onChange: vi.fn() },
      onSend: vi.fn().mockImplementation(() => firstSend.promise),
      onStop: vi.fn().mockResolvedValue(undefined),
      text: '',
    });
    const target = document.createElement('div');
    document.body.append(target);
    component = mount(PrismComposer, { target, props: props as never });

    const composer = target.querySelector<HTMLTextAreaElement>('[placeholder="메시지를 입력하세요"]');
    const send = target.querySelector<HTMLButtonElement>('[aria-label="보내기"]');
    expect(composer).not.toBeNull();
    expect(send).not.toBeNull();
    if (!composer || !send) return;

    await userEvent.fill(composer, '이전 메시지');
    await userEvent.click(send);
    props.generation = 1;
    await tick();

    firstSend.reject(new Error('failed'));
    await firstSend.promise.catch(() => false);
    await tick();

    expect(composer.value).toBe('');
  });
});
