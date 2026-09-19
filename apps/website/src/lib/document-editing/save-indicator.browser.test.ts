import '../../app.css';

import { mount, tick, unmount } from 'svelte';
import { SvelteSet } from 'svelte/reactivity';
import { afterEach, describe, expect, it, vi } from 'vitest';
import DocumentSaveIndicator from './DocumentSaveIndicator.svelte';

const motion = vi.hoisted(() => ({ current: true }));
vi.mock('@typie/ui/state/reduced-motion', () => ({
  prefersReducedMotion: motion,
}));

describe('document save indicator', () => {
  let mounted: Record<string, unknown>;
  let target: HTMLElement;
  afterEach(async () => {
    await unmount(mounted);
    target.remove();
    vi.useRealTimers();
    motion.current = true;
  });

  const create = async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    const flags = new SvelteSet<string>();
    const showDetails = vi.fn();
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentSaveIndicator, {
      target,
      props: {
        get status() {
          if (flags.has('failed')) return 'failed';
          if (flags.has('sync-failed')) return 'sync-failed';
          if (flags.has('idle')) return 'idle';
          return flags.has('pending') ? 'pending' : 'synced';
        },
        get protectedChanges() {
          return !flags.has('unprotected');
        },
        onShowDetails: showDetails,
      },
    });
    await tick();
    return { flags, showDetails };
  };

  it('keeps normal saving quiet for five seconds and restarts the delay for later edits', async () => {
    const { flags } = await create();
    flags.add('unprotected');
    flags.add('pending');
    await tick();
    await vi.advanceTimersByTimeAsync(4999);
    expect(target.querySelector('[aria-label="저장 시도 중"]')).toBeNull();
    await vi.advanceTimersByTimeAsync(1);
    await tick();
    expect(target.querySelector('[aria-label="저장 시도 중"]')).not.toBeNull();
    expect(target.textContent?.trim()).toBe('');
    flags.delete('pending');
    await tick();
    expect(target.querySelector('[aria-label="저장 시도 중"]')).toBeNull();
    expect(target.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
    await vi.advanceTimersByTimeAsync(2000);
    await tick();
    expect(target.querySelector('[role="status"]')).toBeNull();
    flags.add('pending');
    await tick();
    await vi.advanceTimersByTimeAsync(4000);
    expect(target.querySelector('[role="status"]')).toBeNull();
    flags.delete('pending');
    await tick();
    await vi.advanceTimersByTimeAsync(5000);
    expect(target.querySelector('[role="status"]')).toBeNull();
  });

  it('shows a two-second completion check after a visible spinner even without a failure', async () => {
    const { flags } = await create();
    flags.add('unprotected');
    flags.add('pending');
    await tick();
    await vi.advanceTimersByTimeAsync(5000);
    await tick();
    expect(target.querySelector('[aria-label="저장 시도 중"]')).not.toBeNull();
    flags.delete('pending');
    await tick();
    expect(target.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
    await vi.advanceTimersByTimeAsync(1999);
    expect(target.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
    await vi.advanceTimersByTimeAsync(1);
    await tick();
    expect(target.querySelector('[role="status"]')).toBeNull();
  });

  it('shows local preservation as soon as a slow save is captured and confirms only after server acknowledgement', async () => {
    const { flags } = await create();
    flags.add('pending');
    flags.add('unprotected');
    await tick();
    await vi.advanceTimersByTimeAsync(5000);
    await tick();
    expect(target.querySelector('[aria-label="저장 시도 중"]')).not.toBeNull();

    flags.delete('unprotected');
    await tick();
    const locallySaved = () => target.querySelector('[aria-label^="최근 변경사항은 이 기기에만 저장되어 있어요"]');
    expect(locallySaved()).not.toBeNull();
    expect(target.querySelector('[aria-label="저장 시도 중"]')).toBeNull();
    expect(target.querySelector('[aria-label="서버에 저장했어요"]')).toBeNull();

    flags.delete('pending');
    await tick();
    expect(locallySaved()).toBeNull();
    expect(target.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
    await vi.advanceTimersByTimeAsync(2000);
    await tick();
    expect(target.querySelector('[role="status"]')).toBeNull();
  });

  it('shows actionable failure instead of hiding it behind a pending spinner', async () => {
    const { flags, showDetails } = await create();
    flags.add('pending');
    await tick();
    await vi.advanceTimersByTimeAsync(1000);
    flags.add('failed');
    await tick();
    expect(target.querySelector('[aria-label="최근 변경사항 저장 상태 확인"]')).not.toBeNull();
    expect(target.querySelector('[aria-label="저장 시도 중"]')).toBeNull();
    target.querySelector('button')?.click();
    expect(showDetails).toHaveBeenCalledOnce();
    flags.clear();
    await tick();
    expect(target.querySelector('button')).toBeNull();
    expect(target.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
    await vi.advanceTimersByTimeAsync(2000);
    await tick();
    expect(target.querySelector('[role="status"]')).toBeNull();
    flags.add('pending');
    await tick();
    await vi.advanceTimersByTimeAsync(4999);
    flags.clear();
    await tick();
    expect(target.querySelector('[role="status"]')).toBeNull();
  });

  it('shows server-only failure separately and confirms only after server recovery', async () => {
    const { flags, showDetails } = await create();
    expect(target.querySelector('[role="status"]')).toBeNull();
    flags.add('sync-failed');
    await tick();
    const failure = target.querySelector<HTMLButtonElement>('[aria-label="서버 저장 상태 확인"]');
    expect(failure).not.toBeNull();
    expect(failure?.textContent?.trim()).toBe('');
    if (!failure) throw new Error('Save status button missing');
    failure?.click();
    expect(showDetails).toHaveBeenCalledOnce();
    flags.add('pending');
    await tick();
    await vi.advanceTimersByTimeAsync(10_000);
    expect(target.querySelector('[aria-label="서버 저장 상태 확인"]')).toBe(failure);
    expect(target.querySelector('[aria-label="저장 시도 중"]')).toBeNull();
    flags.clear();
    await tick();
    expect(target.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
    flags.add('pending');
    await tick();
    expect(target.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
  });

  it.each(['sync-failed', 'pending'])(
    'keeps completion of %s visible across new edits without delaying later slow-save feedback',
    async (initialStatus) => {
      const { flags } = await create();
      flags.add(initialStatus);
      await tick();
      if (initialStatus === 'pending') await vi.advanceTimersByTimeAsync(5000);
      flags.clear();
      await tick();
      await vi.advanceTimersByTimeAsync(500);
      flags.add('pending');
      await tick();
      expect(target.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
      await vi.advanceTimersByTimeAsync(500);
      flags.add('idle');
      await tick();
      expect(target.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
      await vi.advanceTimersByTimeAsync(500);
      flags.clear();
      await tick();
      expect(target.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
      flags.add('pending');
      await tick();
      await vi.advanceTimersByTimeAsync(499);
      expect(target.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
      await vi.advanceTimersByTimeAsync(1);
      await tick();
      expect(target.querySelector('[role="status"]')).toBeNull();
      await vi.advanceTimersByTimeAsync(4499);
      expect(target.querySelector('[aria-label="저장 시도 중"]')).toBeNull();
      flags.add('unprotected');
      await vi.advanceTimersByTimeAsync(1);
      await tick();
      expect(target.querySelector('[aria-label="저장 시도 중"]')).not.toBeNull();
      flags.clear();
      await tick();
      expect(target.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
      await vi.advanceTimersByTimeAsync(2000);
      await tick();
      expect(target.querySelector('[role="status"]')).toBeNull();
    },
  );

  it.each(['failed', 'sync-failed'])(
    'immediately replaces recovery with %s without a stale check timer hiding the failure',
    async (failure) => {
      const { flags } = await create();
      flags.add('sync-failed');
      await tick();
      flags.clear();
      await tick();
      await vi.advanceTimersByTimeAsync(500);
      flags.add(failure);
      await tick();
      expect(target.querySelector('[aria-label="서버에 저장했어요"]')).toBeNull();
      expect(target.querySelector('button')).not.toBeNull();
      await vi.advanceTimersByTimeAsync(2000);
      await tick();
      expect(target.querySelector('button')).not.toBeNull();
      flags.clear();
      await tick();
      expect(target.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
      await vi.advanceTimersByTimeAsync(1999);
      expect(target.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
      await vi.advanceTimersByTimeAsync(1);
      await tick();
      expect(target.querySelector('[role="status"]')).toBeNull();
    },
  );

  it('crossfades a slow-save spinner into confirmation in the same icon position', async () => {
    motion.current = false;
    const { flags } = await create();
    flags.add('unprotected');
    flags.add('pending');
    await tick();
    await vi.advanceTimersByTimeAsync(5000);
    await tick();
    flags.delete('pending');
    await tick();
    const spinner = target.querySelector('[aria-label="저장 시도 중"]');
    const check = target.querySelector('[aria-label="서버에 저장했어요"]');
    expect(spinner).not.toBeNull();
    expect(check).not.toBeNull();
    expect(spinner?.getBoundingClientRect().toJSON()).toEqual(check?.getBoundingClientRect().toJSON());
    await vi.waitFor(() => expect(target.querySelector('[aria-label="저장 시도 중"]')).toBeNull());
  });

  it('crossfades a failure into confirmation without replacing its entire container', async () => {
    motion.current = false;
    const { flags } = await create();
    flags.add('sync-failed');
    await tick();
    const container = target.firstElementChild;
    const failure = target.querySelector('[aria-label="서버 저장 상태 확인"]');
    expect(failure).not.toBeNull();
    flags.clear();
    await tick();
    expect(target.firstElementChild).toBe(container);
    expect(failure?.isConnected).toBe(true);
    expect(target.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
    await vi.waitFor(() => expect(failure?.isConnected).toBe(false));
  });

  it('appears immediately but fades out before releasing its header space', async () => {
    motion.current = false;
    const { flags } = await create();
    flags.add('sync-failed');
    await tick();
    const container = target.firstElementChild;
    expect(container).not.toBeNull();
    if (!container) throw new Error('Save indicator container missing');
    expect(container.getAnimations({ subtree: true })).toHaveLength(0);
    flags.clear();
    await tick();
    await vi.waitFor(() => expect(target.querySelector('[aria-label="서버 저장 상태 확인"]')).toBeNull());
    await vi.advanceTimersByTimeAsync(2000);
    await tick();
    expect(container.isConnected).toBe(true);
    await vi.waitFor(() => {
      const opacity = container
        .getAnimations()
        .flatMap((animation) => (animation.effect instanceof KeyframeEffect ? animation.effect.getKeyframes() : []))
        .map((frame) => Number(frame.opacity));
      expect(opacity[0]).toBe(1);
      expect(opacity.at(-1)).toBe(0);
    });
    await vi.waitFor(() => expect(container.isConnected).toBe(false));
    expect(target.childElementCount).toBe(0);
  });

  it.each(['sync-failed', 'pending'])('does not confirm %s while input finalization remains', async (initialStatus) => {
    const { flags } = await create();
    flags.add(initialStatus);
    await tick();
    if (initialStatus === 'pending') await vi.advanceTimersByTimeAsync(5000);
    flags.add('idle');
    flags.delete(initialStatus);
    await tick();
    await vi.advanceTimersByTimeAsync(3000);
    expect(target.querySelector('[role="status"]')).toBeNull();
    flags.delete('idle');
    await tick();
    expect(target.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
  });

  it('only promises local protection in the server failure tooltip after pending input is preserved', async () => {
    const { flags } = await create();
    flags.add('sync-failed');
    flags.add('unprotected');
    await tick();
    const failure = target.querySelector<HTMLButtonElement>('[aria-label="서버 저장 상태 확인"]');
    if (!failure) throw new Error('Save status button missing');
    failure.dispatchEvent(new PointerEvent('pointerenter'));
    await vi.advanceTimersByTimeAsync(500);
    await tick();
    const tooltip = () => document.querySelector('[role="tooltip"]')?.textContent ?? '';
    await vi.waitFor(() => expect(tooltip()).toContain('서버에 저장하지 못했어요'));
    expect(tooltip()).not.toContain('이 기기에만 저장되어 있어요');
    flags.delete('unprotected');
    await tick();
    target.querySelector('[aria-label="서버 저장 상태 확인"]')?.dispatchEvent(new PointerEvent('pointerenter'));
    await vi.advanceTimersByTimeAsync(500);
    await vi.waitFor(() => expect(tooltip()).toContain('이 기기에만 저장되어 있어요'));
  });
});
