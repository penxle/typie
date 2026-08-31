import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { reactiveProps } from './PrismPanelIndicator.test-props.svelte.ts';
import PrismPushCard from './PrismPushCard.svelte';

const mocks = vi.hoisted(() => ({
  enable: vi.fn<() => Promise<boolean>>(),
  permission: 'default' as NotificationPermission,
  toastError: vi.fn(),
  toastSuccess: vi.fn(),
}));

vi.mock('$app/environment', () => ({ browser: false }));

vi.mock('@typie/ui/notification', () => ({ Toast: { error: mocks.toastError, success: mocks.toastSuccess } }));

vi.mock('$lib/browser-push', () => ({
  BROWSER_PUSH_STORAGE_KEY: 'typie:browser-push',
  readCurrentBrowserPushIntent: () => null,
}));

vi.mock('$lib/push', () => ({
  getBrowserPushManager: () => ({ enable: mocks.enable }),
  pushPermission: () => mocks.permission,
  pushSupported: async () => true,
}));

let originalAnimateDescriptor: PropertyDescriptor | undefined;

beforeEach(() => {
  mocks.enable.mockReset().mockResolvedValue(true);
  mocks.permission = 'default';
  mocks.toastError.mockReset();
  mocks.toastSuccess.mockReset();
  originalAnimateDescriptor = Object.getOwnPropertyDescriptor(Element.prototype, 'animate');
  Object.defineProperty(Element.prototype, 'animate', {
    configurable: true,
    value: vi.fn(() => {
      const animation = {
        cancel: vi.fn(),
        currentTime: 0,
        effect: null,
        onfinish: null as (() => void) | null,
        playState: 'running',
      };
      queueMicrotask(() => animation.onfinish?.());
      return animation as unknown as Animation;
    }),
  });
});

afterEach(() => {
  if (originalAnimateDescriptor) Object.defineProperty(Element.prototype, 'animate', originalAnimateDescriptor);
  else Reflect.deleteProperty(Element.prototype, 'animate');
  document.body.replaceChildren();
});

describe('PrismPushCard', () => {
  const clickButton = (target: HTMLElement, label: string) => {
    const button = [...target.querySelectorAll('button')].find((candidate) => candidate.textContent?.trim() === label);
    expect(button).toBeDefined();
    button?.click();
  };

  it('remains available after opening the panel clears the unread badge', async () => {
    const props = reactiveProps({ visible: false });
    const target = document.createElement('div');
    const component = mount(PrismPushCard, { target, props });
    try {
      await tick();
      await vi.waitFor(() => expect(target.textContent).not.toContain('알림 받기'));

      props.visible = true;
      await tick();
      props.visible = false;
      await tick();
      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(target.textContent).toContain('알림 받기');
      expect(target.textContent).toContain('확인이 필요하거나 리뷰가 끝나면 브라우저 알림 받기');
    } finally {
      await unmount(component);
    }
  });

  it('explains where to enable notifications after dismissing the offer', async () => {
    const target = document.createElement('div');
    const component = mount(PrismPushCard, { target, props: { visible: true } });
    try {
      await vi.waitFor(() => expect(target.textContent).toContain('나중에'));
      clickButton(target, '나중에');
      await tick();

      expect(mocks.toastSuccess).toHaveBeenCalledWith('설정 > 알림에서 언제든 켤 수 있어요');
    } finally {
      await unmount(component);
    }
  });

  it('reports success after enabling browser notifications', async () => {
    const target = document.createElement('div');
    const component = mount(PrismPushCard, { target, props: { visible: true } });
    try {
      await vi.waitFor(() => expect(target.textContent).toContain('알림 받기'));
      clickButton(target, '알림 받기');

      await vi.waitFor(() => expect(mocks.toastSuccess).toHaveBeenCalledWith('브라우저 알림을 켰어요'));
    } finally {
      await unmount(component);
    }
  });

  it('distinguishes blocked permission from other enable failures', async () => {
    const renderAndEnable = async () => {
      const target = document.createElement('div');
      const component = mount(PrismPushCard, { target, props: { visible: true } });
      await vi.waitFor(() => expect(target.textContent).toContain('알림 받기'));
      clickButton(target, '알림 받기');
      return component;
    };

    mocks.enable.mockImplementationOnce(async () => {
      mocks.permission = 'denied';
      return false;
    });
    const blocked = await renderAndEnable();
    await vi.waitFor(() => expect(mocks.toastError).toHaveBeenCalledWith('브라우저 설정에서 타이피 알림을 허용해 주세요'));
    await unmount(blocked);

    mocks.permission = 'default';
    mocks.toastError.mockReset();
    mocks.enable.mockResolvedValueOnce(false);
    const failed = await renderAndEnable();
    await vi.waitFor(() => expect(mocks.toastError).toHaveBeenCalledWith('브라우저 알림을 켜지 못했어요. 설정에서 다시 시도해 주세요'));
    await unmount(failed);
  });
});
