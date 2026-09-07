import '../../../../app.css';

import { mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import TagPills from './TagPills.svelte';

let component: Record<string, unknown> | undefined;

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
});

const openInput = async (tags: string[]) => {
  const target = document.createElement('div');
  document.body.append(target);
  component = mount(TagPills, { target, props: { tags } as never });

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
    component = mount(TagPills, { target, props: { tags } as never });

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
    component = mount(TagPills, { target, props: { tags: ['봄'] } as never });

    const chip = chipButton(target, '봄');
    expect(chip).toBeDefined();

    expect(await clickAndTrackTarget(chip as HTMLButtonElement)).toBe(true);
  });

  it('추가 알약을 눌러도 눌린 요소가 문서에 남는다', async () => {
    const target = document.createElement('div');
    document.body.append(target);
    component = mount(TagPills, { target, props: { tags: [] } as never });

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
