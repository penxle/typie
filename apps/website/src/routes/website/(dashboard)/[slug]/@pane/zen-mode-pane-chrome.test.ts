import { afterEach, describe, expect, it, vi } from 'vitest';
import { paneChromeRevealTargets, ZenModePaneChrome } from './zen-mode-pane-chrome.svelte';

afterEach(() => vi.useRealTimers());

describe('paneChromeRevealTargets', () => {
  it('keeps direct header segments independent', () => {
    expect(paneChromeRevealTargets('identity')).toEqual(['identity']);
    expect(paneChromeRevealTargets('actions')).toEqual(['actions']);
  });

  it('unifies the header after middle-gap dwell and promotes toolbar intent to the full group', () => {
    expect(paneChromeRevealTargets('gap')).toEqual(['identity', 'actions']);
    expect(paneChromeRevealTargets('toolbar')).toEqual(['identity', 'actions', 'toolbar']);
  });
});

describe('ZenModePaneChrome lifetime', () => {
  it('holds promoted toolbar chrome and clears every surface after the shared grace and fade', () => {
    vi.useFakeTimers();
    const chrome = new ZenModePaneChrome({ active: () => true, focused: () => true });
    chrome.sync();

    chrome.hold('toolbar', 'focus');
    vi.advanceTimersByTime(100);
    expect(chrome.foreground).toEqual({ identity: true, actions: true, toolbar: true });

    chrome.release('toolbar', 'focus');
    vi.advanceTimersByTime(1500);
    expect(chrome.phase).toBe('fading');
    expect(chrome.shown).toEqual({ identity: true, actions: true, toolbar: true });

    vi.advanceTimersByTime(400);
    expect(chrome.phase).toBe('idle');
    expect(chrome.shown).toEqual({ identity: false, actions: false, toolbar: false });
  });

  it('does not replay the entrance reveal for an active reload revision', () => {
    const chrome = new ZenModePaneChrome({ active: () => true, focused: () => true });
    chrome.sync();
    expect(chrome.shown).toEqual({ identity: false, actions: false, toolbar: false });
  });

  it('replays an actions-segment entry prepared by repeated focus-mode button clicks', () => {
    let active = false;
    const chrome = new ZenModePaneChrome({ active: () => active, focused: () => true });
    chrome.sync();

    for (let count = 0; count < 2; count += 1) {
      chrome.prepareEntryReveal('actions', { clientX: 500, clientY: 18 });
      active = true;
      chrome.sync();
      expect(chrome.shown).toEqual({ identity: false, actions: true, toolbar: false });

      active = false;
      chrome.sync();
    }
  });

  it('ignores the stale focus release from the control that entered focus mode', () => {
    let active = false;
    const chrome = new ZenModePaneChrome({ active: () => active, focused: () => true });
    chrome.sync();

    active = true;
    chrome.sync();
    chrome.release('actions', 'focus');
    chrome.hold('actions', 'hover');

    expect(chrome.shown).toEqual({ identity: false, actions: false, toolbar: false });
  });

  it('drops attachment ownership across a focus-mode lifecycle', () => {
    vi.useFakeTimers();
    let active = true;
    const chrome = new ZenModePaneChrome({ active: () => active, focused: () => true });
    const attachment = chrome.attachmentHandle();
    chrome.sync();
    attachment.hold();

    active = false;
    chrome.sync();
    active = true;
    chrome.sync();
    chrome.handlePointerLeave();
    chrome.hold('identity', 'focus');
    chrome.release('identity', 'focus');
    vi.advanceTimersByTime(1500);

    expect(chrome.phase).toBe('fading');
  });
});
