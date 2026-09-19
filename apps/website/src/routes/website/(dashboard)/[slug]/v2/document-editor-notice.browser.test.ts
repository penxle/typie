import '../../../../../app.css';

import { mount, tick, unmount } from 'svelte';
import { SvelteSet } from 'svelte/reactivity';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import DocumentEditorNotice from './DocumentEditorNotice.svelte';
import type { PaneChromeAttachmentHandle } from '../@pane/zen-mode-pane-chrome.svelte';

vi.mock('@typie/ui/state', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@typie/ui/state')>()),
  prefersReducedMotion: { current: true },
}));

describe('document editor notice', () => {
  let mounted: DocumentEditorNotice;
  let target: HTMLElement;
  afterEach(async () => {
    if (mounted) await unmount(mounted);
    target?.remove();
    vi.useRealTimers();
  });

  const create = async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    const flags = new SvelteSet<string>();
    const details = vi.fn();
    const unlock = vi.fn();
    const attachment: PaneChromeAttachmentHandle = {
      hold: vi.fn(),
      release: vi.fn(),
      discoverable: () => false,
      attached: () => false,
      pointer: () => null,
    };
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentEditorNotice, {
      target,
      props: {
        get saveFailed() {
          return flags.has('failed');
        },
        top: '12px',
        attachment,
        onShowSaveDetails: details,
        onUnlock: unlock,
      },
    });
    await tick();
    return { flags, details, unlock };
  };

  it('preserves edit-lock feedback and its unlock action', async () => {
    const { unlock } = await create();
    expect(target.querySelector('[role="alert"]')).toBeNull();
    mounted.showLocked();
    await tick();
    expect(target.querySelector('[role="alert"]')).not.toBeNull();
    target.querySelector('button')?.click();
    await tick();
    expect(unlock).toHaveBeenCalledOnce();
    expect(target.querySelector('[role="alert"]')).toBeNull();
  });

  it('shows one five-second failure notice per unprotected failure, not on every retry', async () => {
    const { flags } = await create();
    flags.add('failed');
    await tick();
    expect(target.querySelector('[role="alert"]')).not.toBeNull();
    await vi.advanceTimersByTimeAsync(4999);
    expect(target.querySelector('[role="alert"]')).not.toBeNull();
    await vi.advanceTimersByTimeAsync(1);
    await tick();
    expect(target.querySelector('[role="alert"]')).toBeNull();
    flags.add('failed');
    await tick();
    await vi.advanceTimersByTimeAsync(10_000);
    expect(target.querySelector('[role="alert"]')).toBeNull();
    flags.delete('failed');
    await tick();
    flags.add('failed');
    await tick();
    expect(target.querySelector('[role="alert"]')).not.toBeNull();
    flags.delete('failed');
    await tick();
    expect(target.querySelector('[role="alert"]')).toBeNull();
  });

  it('stacks independent notices and dismisses only the notice whose action was used', async () => {
    const { flags, details, unlock } = await create();
    mounted.showLocked();
    flags.add('failed');
    await tick();
    mounted.showLocked();
    await tick();
    expect(target.querySelectorAll('[role="alert"]')).toHaveLength(2);
    const rows = [...target.querySelectorAll('[role="alert"]')];
    expect(rows[0].getBoundingClientRect().bottom).toBeLessThan(rows[1].getBoundingClientRect().top);
    await page.screenshot({ path: '../../../../../../.vitest-screenshots/document-editor-notice-stack.png' });
    const detailsButton = [...target.querySelectorAll('button')].find((button) => button.textContent?.includes('저장 상태 확인'));
    if (!detailsButton) throw new Error('Save details action missing');
    detailsButton.click();
    await tick();
    expect(details).toHaveBeenCalledOnce();
    expect(unlock).not.toHaveBeenCalled();
    expect(target.querySelectorAll('[role="alert"]')).toHaveLength(1);
    target.querySelector('button')?.click();
    await tick();
    expect(unlock).toHaveBeenCalledOnce();
    expect(target.querySelector('[role="alert"]')).toBeNull();
  });

  it('expires each notice independently without extending an existing lock notice', async () => {
    const { flags } = await create();
    mounted.showLocked();
    await tick();
    await vi.advanceTimersByTimeAsync(2000);
    flags.add('failed');
    await tick();
    expect(target.querySelectorAll('[role="alert"]')).toHaveLength(2);
    mounted.showLocked();
    await vi.advanceTimersByTimeAsync(3000);
    await tick();
    expect(target.querySelectorAll('[role="alert"]')).toHaveLength(1);
    expect(target.querySelector('button')?.textContent).toContain('저장 상태 확인');
    await vi.advanceTimersByTimeAsync(2000);
    await tick();
    expect(target.querySelector('[role="alert"]')).toBeNull();
  });

  it('keeps the action available while the pointer is over the notice', async () => {
    const { flags } = await create();
    flags.add('failed');
    await tick();
    await page.screenshot({ path: '../../../../../../.vitest-screenshots/document-save-failure-notice.png' });
    const notice = target.querySelector('[role="alert"]');
    if (!notice) throw new Error('Save failure notice missing');
    notice.dispatchEvent(new PointerEvent('pointerenter'));
    await vi.advanceTimersByTimeAsync(6000);
    expect(target.querySelector('[role="alert"]')).not.toBeNull();
    notice.dispatchEvent(new PointerEvent('pointerleave'));
    await vi.advanceTimersByTimeAsync(5000);
    await tick();
    expect(target.querySelector('[role="alert"]')).toBeNull();
  });
});
