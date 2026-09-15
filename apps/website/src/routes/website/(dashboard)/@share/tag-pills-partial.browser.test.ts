import '../../../../app.css';

import { createQuery } from '@mearie/svelte';
import { mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import TagPills from './TagPills.svelte';

vi.mock(import('@mearie/svelte'), async (importOriginal) => ({
  ...(await importOriginal()),
  createQuery: vi.fn(),
}));

let component: Record<string, unknown> | undefined;

beforeEach(() => {
  vi.mocked(createQuery)
    .mockReset()
    .mockReturnValue({ data: undefined, loading: false, error: undefined, refetch: vi.fn() } as never);
});

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
});

describe('TagPills partial chips', () => {
  it('일부만 가진 태그를 점선 칩으로 그리고 삭제 버튼이 콜백을 부른다', async () => {
    const target = document.createElement('div');
    document.body.append(target);
    const onremovepartial = vi.fn();
    component = mount(TagPills, {
      target,
      props: { tags: ['일기'], partial: [{ tag: '에세이', count: 2 }], onremovepartial, siteId: 'site-1' } as never,
    });

    const chips = [...target.querySelectorAll<HTMLElement>('span')].filter((el) => el.textContent?.trim().startsWith('에세이'));
    expect(chips.length).toBeGreaterThan(0);
    const chip = chips[0];
    expect(getComputedStyle(chip).borderStyle).toBe('dashed');

    const remove = chip.querySelector<HTMLButtonElement>('button[aria-label="태그 삭제"]');
    expect(remove).not.toBeNull();
    if (!remove) throw new Error('삭제 버튼을 찾지 못했어요');

    await userEvent.click(remove);
    expect(onremovepartial).toHaveBeenCalledWith('에세이');
  });

  it('공통 태그가 없고 일부 태그만 있으면 "없음"을 보이지 않는다', () => {
    const target = document.createElement('div');
    document.body.append(target);
    component = mount(TagPills, {
      target,
      props: { tags: [], partial: [{ tag: '에세이', count: 1 }], disabled: true, siteId: 'site-1' } as never,
    });
    expect(target.textContent).not.toContain('없음');
    expect(target.textContent).toContain('에세이');
  });
});
