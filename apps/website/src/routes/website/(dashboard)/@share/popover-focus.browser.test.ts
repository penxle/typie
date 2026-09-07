import '../../../../app.css';

import { mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import Fixture from './popover-focus.test-fixture.svelte';

let component: Record<string, unknown> | undefined;

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
});

const openPopover = async () => {
  const target = document.createElement('div');
  document.body.append(target);
  component = mount(Fixture, { target });

  const trigger = document.body.querySelector<HTMLButtonElement>('[aria-haspopup="dialog"]');
  expect(trigger).not.toBeNull();
  if (!trigger) throw new Error('트리거를 찾지 못했어요');

  await userEvent.click(trigger);

  return await vi.waitFor(() => {
    const el = document.body.querySelector<HTMLTextAreaElement>('textarea[aria-label="미리보기 문구"]');
    expect(el).not.toBeNull();
    return el as HTMLTextAreaElement;
  });
};

describe('모달 안 팝오버', () => {
  it('열리면 입력에 포커스가 가고 그대로 머문다', async () => {
    const textarea = await openPopover();

    await vi.waitFor(() => expect(document.activeElement).toBe(textarea));

    await userEvent.type(textarea, '여름');
    expect(document.activeElement).toBe(textarea);
    expect(textarea.value).toBe('여름');
  });

  it('입력을 클릭해도 포커스를 빼앗기지 않는다', async () => {
    const textarea = await openPopover();

    await userEvent.click(textarea);
    await userEvent.type(textarea, '가을');

    expect(document.activeElement).toBe(textarea);
    expect(textarea.value).toBe('가을');
  });
});
