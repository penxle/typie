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

type Suggestion = { name: string; count: number };
type Suggestions = { mine: Suggestion[]; popular: Suggestion[] };

let component: Record<string, unknown> | undefined;
let getVariables: (() => unknown) | undefined;

const mockSuggestions = (suggestions: Suggestions | undefined) => {
  vi.mocked(createQuery)
    .mockReset()
    .mockImplementation(((_document: unknown, variables: () => unknown) => {
      getVariables = variables;
      return {
        data: suggestions ? { site: { id: 'site-1', tagSuggestions: suggestions } } : undefined,
        loading: false,
        error: undefined,
        refetch: vi.fn(),
      };
    }) as never);
};

beforeEach(() => {
  getVariables = undefined;
  mockSuggestions(undefined);
});

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
});

const openInput = async (tags: string[]) => {
  const target = document.createElement('div');
  document.body.append(target);
  component = mount(TagPills, { target, props: { tags, siteId: 'site-1' } as never });

  const add = target.querySelector<HTMLButtonElement>('[data-primary]');
  expect(add).not.toBeNull();
  if (!add) throw new Error('추가 버튼을 찾지 못했어요');

  await userEvent.click(add);

  const input = await vi.waitFor(() => {
    const el = target.querySelector<HTMLInputElement>('input[aria-label="태그"]');
    expect(el).not.toBeNull();
    return el as HTMLInputElement;
  });

  return { target, input };
};

const contentWidth = (input: HTMLInputElement) => {
  const style = getComputedStyle(input);
  return input.clientWidth - Number.parseFloat(style.paddingLeft) - Number.parseFloat(style.paddingRight);
};

const measureText = (input: HTMLInputElement, text: string) => {
  const style = getComputedStyle(input);
  const probe = document.createElement('span');
  probe.textContent = text;
  probe.style.position = 'absolute';
  probe.style.visibility = 'hidden';
  probe.style.whiteSpace = 'pre';
  probe.style.fontFamily = style.fontFamily;
  probe.style.fontSize = style.fontSize;
  probe.style.fontWeight = style.fontWeight;
  probe.style.fontFeatureSettings = style.fontFeatureSettings;
  probe.style.letterSpacing = style.letterSpacing;
  document.body.append(probe);
  const width = probe.getBoundingClientRect().width;
  probe.remove();
  return width;
};

const chipButton = (target: HTMLElement, name: string) =>
  [...target.querySelectorAll<HTMLButtonElement>('button')].find((el) => el.textContent?.trim() === name);

describe('태그 사이 이동', () => {
  it('입력 칸에서 왼쪽으로 나가면 칩이 포커스를 받고 추가 알약이 돌아온다', async () => {
    const { target, input } = await openInput(['봄', '여름']);

    await userEvent.keyboard('{ArrowLeft}');

    await vi.waitFor(() => expect(document.activeElement).toBe(chipButton(target, '여름')));
    expect(target.contains(input)).toBe(false);
    expect(target.querySelector<HTMLElement>('[data-primary]')?.hidden).toBe(false);
  });

  it('칩에서 오른쪽 끝을 넘어가면 다시 입력 모드가 된다', async () => {
    const { target } = await openInput(['봄', '여름']);

    await userEvent.keyboard('{ArrowLeft}');
    await vi.waitFor(() => expect(document.activeElement).toBe(chipButton(target, '여름')));

    await userEvent.keyboard('{ArrowLeft}');
    await vi.waitFor(() => expect(document.activeElement).toBe(chipButton(target, '봄')));

    await userEvent.keyboard('{ArrowRight}{ArrowRight}');

    await vi.waitFor(() => {
      const el = target.querySelector<HTMLInputElement>('input[aria-label="태그"]');
      expect(el).not.toBeNull();
      expect(document.activeElement).toBe(el);
    });
  });

  it('칩에서 백스페이스로 지우면 왼쪽 칩이 포커스를 이어받는다', async () => {
    const { target } = await openInput(['봄', '여름']);

    await userEvent.keyboard('{ArrowLeft}{Backspace}');

    await vi.waitFor(() => expect(document.activeElement).toBe(chipButton(target, '봄')));
    expect(chipButton(target, '여름')).toBeUndefined();
  });
});

describe('태그 편집', () => {
  const startEditing = async (tags: string[], name: string) => {
    const target = document.createElement('div');
    document.body.append(target);
    component = mount(TagPills, { target, props: { tags, siteId: 'site-1' } as never });

    const chip = chipButton(target, name);
    expect(chip).toBeDefined();
    await userEvent.click(chip as HTMLButtonElement);

    await vi.waitFor(() => expect(target.querySelector('input[aria-label="태그"]')).not.toBeNull());

    return target;
  };

  it('엔터로 끝내면 고친 태그가 선택된 상태로 돌아온다', async () => {
    const target = await startEditing(['봄', '여름'], '봄');

    await userEvent.keyboard('가을{Enter}');

    await vi.waitFor(() => expect(document.activeElement).toBe(chipButton(target, '가을')));
    expect(target.querySelector('input[aria-label="태그"]')).toBeNull();

    await userEvent.keyboard('{ArrowRight}');
    await vi.waitFor(() => expect(document.activeElement).toBe(chipButton(target, '여름')));
  });

  it('ESC로 끝내면 고치던 태그가 그대로 선택된다', async () => {
    const target = await startEditing(['봄', '여름'], '봄');

    await userEvent.keyboard('가을{Escape}');

    await vi.waitFor(() => expect(document.activeElement).toBe(chipButton(target, '봄')));
    expect(target.querySelector('input[aria-label="태그"]')).toBeNull();
  });

  it('선택된 태그에서 엔터를 누르면 다시 고칠 수 있다', async () => {
    const target = await startEditing(['봄'], '봄');

    await userEvent.keyboard('{Escape}');
    await vi.waitFor(() => expect(document.activeElement).toBe(chipButton(target, '봄')));

    await userEvent.keyboard('{Enter}');

    await vi.waitFor(() => {
      const el = target.querySelector<HTMLInputElement>('input[aria-label="태그"]');
      expect(el).not.toBeNull();
      expect(document.activeElement).toBe(el);
      expect(el?.value).toBe('봄');
    });
  });

  const clickAndTrackTarget = async (button: HTMLButtonElement) => {
    let connected: boolean | undefined;
    window.addEventListener('click', (event) => (connected = (event.target as Node).isConnected), { once: true });

    await userEvent.click(button);

    return connected;
  };

  it('칩을 눌러도 눌린 요소가 문서에 남는다', async () => {
    const target = document.createElement('div');
    document.body.append(target);
    component = mount(TagPills, { target, props: { tags: ['봄'], siteId: 'site-1' } as never });

    const chip = chipButton(target, '봄');
    expect(chip).toBeDefined();

    expect(await clickAndTrackTarget(chip as HTMLButtonElement)).toBe(true);
  });

  it('추가 알약을 눌러도 눌린 요소가 문서에 남는다', async () => {
    const target = document.createElement('div');
    document.body.append(target);
    component = mount(TagPills, { target, props: { tags: [], siteId: 'site-1' } as never });

    const add = target.querySelector<HTMLButtonElement>('[data-primary]');
    expect(add).not.toBeNull();

    expect(await clickAndTrackTarget(add as HTMLButtonElement)).toBe(true);
  });

  it('고치는 중에 다른 칩을 누르면 한 번에 그 칩으로 넘어간다', async () => {
    const target = await startEditing(['봄', '여름'], '봄');

    await userEvent.keyboard('가을');

    const other = chipButton(target, '여름');
    expect(other).toBeDefined();
    await userEvent.click(other as HTMLButtonElement);

    await vi.waitFor(() => {
      const el = target.querySelector<HTMLInputElement>('input[aria-label="태그"]');
      expect(el).not.toBeNull();
      expect(el?.value).toBe('여름');
      expect(document.activeElement).toBe(el);
    });

    expect(chipButton(target, '가을')).toBeDefined();
  });

  it('내용을 다 지우고 끝내면 태그가 사라지고 입력이 열린다', async () => {
    const target = await startEditing(['봄'], '봄');

    await userEvent.keyboard('{Backspace}{Enter}');

    await vi.waitFor(() => {
      const el = target.querySelector<HTMLInputElement>('input[aria-label="태그"]');
      expect(el).not.toBeNull();
      expect(document.activeElement).toBe(el);
    });

    expect(chipButton(target, '봄')).toBeUndefined();
  });
});

describe('태그 입력 칸', () => {
  it('플레이스홀더가 잘리지 않는다', async () => {
    const { input } = await openInput([]);

    expect(input.placeholder).toBe('태그 입력');
    expect(contentWidth(input)).toBeGreaterThanOrEqual(measureText(input, input.placeholder));
  });

  it('입력한 글자가 늘어나도 잘리지 않는다', async () => {
    const { input } = await openInput([]);

    await userEvent.type(input, '여름의 끝에서');
    await vi.waitFor(() => expect(input.value).toBe('여름의 끝에서'));

    expect(contentWidth(input)).toBeGreaterThanOrEqual(measureText(input, input.value));
    expect(input.scrollWidth).toBeLessThanOrEqual(input.clientWidth + 1);
  });
});

const suggestions: Suggestions = {
  mine: [
    { name: '봄', count: 3 },
    { name: '여행', count: 2 },
  ],
  popular: [
    { name: '바다', count: 50 },
    { name: '산', count: 40 },
  ],
};

const listbox = (target: HTMLElement) =>
  target.querySelector<HTMLElement>('[role="listbox"]') ?? document.querySelector<HTMLElement>('[role="listbox"]');
const options = (target: HTMLElement) => [...(listbox(target)?.querySelectorAll<HTMLElement>('[role="option"]') ?? [])];

describe('태그 제안', () => {
  it('입력이 열리면 빈 질의로 두 섹션이 바로 보인다', async () => {
    mockSuggestions(suggestions);
    const { target } = await openInput([]);

    await vi.waitFor(() => expect(listbox(target)).not.toBeNull());
    expect(listbox(target)?.textContent).toContain('내 태그');
    expect(listbox(target)?.textContent).toContain('인기 태그');
    expect(options(target).map((el) => el.dataset.name)).toEqual(['봄', '여행', '바다', '산']);
    expect(getVariables?.()).toEqual({ siteId: 'site-1', query: '', exclude: [] });
    expect(getComputedStyle(listbox(target) as HTMLElement).animationName).not.toBe('none');
  });

  it('타이핑하면 디바운스 뒤 질의와 exclude가 변수로 나간다', async () => {
    mockSuggestions(suggestions);
    await openInput(['봄']);

    await userEvent.keyboard('여');
    expect(getVariables?.()).toEqual({ siteId: 'site-1', query: '', exclude: ['봄'] });

    await vi.waitFor(() => expect(getVariables?.()).toEqual({ siteId: 'site-1', query: '여', exclude: ['봄'] }), { timeout: 1000 });
  });

  it('아래 화살표로 하이라이트를 옮기고 Enter로 고른다', async () => {
    mockSuggestions(suggestions);
    const { target, input } = await openInput([]);
    await vi.waitFor(() => expect(options(target).length).toBe(4));

    expect(input.getAttribute('aria-activedescendant')).toBeNull();

    await userEvent.keyboard('{ArrowDown}');
    expect(options(target)[0].getAttribute('aria-selected')).toBe('true');
    expect(input.getAttribute('aria-activedescendant')).toBe(options(target)[0].id);

    await userEvent.keyboard('{ArrowDown}{ArrowDown}');
    expect(options(target)[2].getAttribute('aria-selected')).toBe('true');

    await userEvent.keyboard('{Enter}');
    await vi.waitFor(() => expect(chipButton(target, '바다')).not.toBeUndefined());
    expect(document.activeElement).toBe(input);
    expect(input.value).toBe('');
  });

  it('위 화살표는 끝에서 처음으로 순환한다', async () => {
    mockSuggestions(suggestions);
    const { target } = await openInput([]);
    await vi.waitFor(() => expect(options(target).length).toBe(4));

    await userEvent.keyboard('{ArrowUp}');
    expect(options(target)[3].getAttribute('aria-selected')).toBe('true');
  });

  it('하이라이트가 없으면 Enter는 친 글자 그대로 새 태그를 만든다', async () => {
    mockSuggestions(suggestions);
    const { target } = await openInput([]);
    await vi.waitFor(() => expect(options(target).length).toBe(4));

    await userEvent.keyboard('봄날{Enter}');

    await vi.waitFor(() => expect(chipButton(target, '봄날')).not.toBeUndefined());
    expect(chipButton(target, '봄')).toBeUndefined();
  });

  it('제안을 클릭하면 태그가 붙고 입력은 열린 채 포커스를 유지한다', async () => {
    mockSuggestions(suggestions);
    const { target, input } = await openInput([]);
    await vi.waitFor(() => expect(options(target).length).toBe(4));

    await userEvent.click(options(target)[1]);

    await vi.waitFor(() => expect(chipButton(target, '여행')).not.toBeUndefined());
    expect(target.contains(input)).toBe(true);
    expect(document.activeElement).toBe(input);
  });

  it('붙인 태그는 새 응답을 기다리지 않고 목록에서 바로 빠진다', async () => {
    mockSuggestions(suggestions);
    const { target } = await openInput([]);
    await vi.waitFor(() => expect(options(target).length).toBe(4));

    await userEvent.click(options(target)[1]);

    await vi.waitFor(() => expect(chipButton(target, '여행')).not.toBeUndefined());
    expect(options(target).map((el) => el.dataset.name)).toEqual(['봄', '바다', '산']);
  });

  it('두 섹션이 모두 비면 패널을 그리지 않는다', async () => {
    mockSuggestions({ mine: [], popular: [] });
    const { target } = await openInput([]);

    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(listbox(target)).toBeNull();
    expect(target.querySelector('input[aria-label="태그"]')?.getAttribute('aria-expanded')).toBe('false');
  });

  it('칩 편집 모드에서는 제안이 뜨지 않는다', async () => {
    mockSuggestions(suggestions);
    const { target } = await openInput(['봄']);
    await vi.waitFor(() => expect(listbox(target)).not.toBeNull());

    await userEvent.keyboard('{Escape}');
    const chip = chipButton(target, '봄');
    expect(chip).not.toBeUndefined();
    await userEvent.click(chip as HTMLButtonElement);

    await vi.waitFor(() => expect(target.querySelector<HTMLInputElement>('input[aria-label="태그"]')?.value).toBe('봄'));
    expect(listbox(target)).toBeNull();
  });
});
