import '../../../../app.css';

import * as Sentry from '@sentry/sveltekit';
import { getAppContext } from '@typie/ui/context';
import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import PrismPanelBoundaryTestHost, { prismPanelBoundaryHarness } from './PrismPanelBoundaryTestHost.svelte';

vi.mock('@sentry/sveltekit', () => ({ captureException: vi.fn() }));
vi.mock('@typie/ui/context', () => ({ getAppContext: vi.fn(), tryAppContext: vi.fn() }));

const app = {
  preference: {
    current: {
      prismPanelOpen: true,
      prismPanelWidth: 432,
      zenModeEnabled: false,
    },
  },
  state: {},
};

let component: ReturnType<typeof mount> | undefined;
let consoleError: ReturnType<typeof vi.spyOn>;

beforeEach(() => {
  prismPanelBoundaryHarness.reset();
  vi.mocked(Sentry.captureException).mockReset();
  vi.mocked(getAppContext).mockReturnValue(app as never);
  app.preference.current.prismPanelOpen = true;
  app.preference.current.prismPanelWidth = 432;
  app.preference.current.zenModeEnabled = false;
  document.documentElement.dataset.theme = 'light';
  document.documentElement.dataset.variantLight = 'white';
  document.documentElement.dataset.variantDark = 'charcoal';
  consoleError = vi.spyOn(console, 'error').mockImplementation(() => null);
});

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  consoleError.mockRestore();
  document.body.replaceChildren();
});

describe('PRISM panel error boundary', () => {
  test('reports the failure and retries the panel in place', async () => {
    const target = document.createElement('div');
    Object.assign(target.style, { display: 'flex', height: '640px', position: 'relative', width: '432px' });
    document.body.append(target);
    component = mount(PrismPanelBoundaryTestHost, { target });

    await vi.waitFor(() => expect(target.querySelector('[role="alert"]')).not.toBeNull());
    await tick();

    const shell = target.querySelector<HTMLElement>('[data-prism-panel-shell]');
    const spacer = target.querySelector<HTMLElement>('[data-prism-panel-spacer]');
    const reveal = target.querySelector<HTMLElement>('[data-prism-panel-reveal]');
    const edge = target.querySelector<HTMLElement>('[data-prism-panel-edge]');
    const resizeHandle = target.querySelector<HTMLElement>('[data-prism-panel-resize-handle]');
    const retry = target.querySelector<HTMLButtonElement>('button:not([aria-label])');
    if (!shell) throw new Error('Expected the PRISM panel shell to render');
    if (!reveal) throw new Error('Expected the PRISM panel reveal to render');
    if (!edge) throw new Error('Expected the PRISM panel edge to render');
    if (!resizeHandle) throw new Error('Expected the PRISM panel resize handle to render');
    expect(shell).not.toBe(spacer);
    expect(spacer?.style.width).toBe('432px');
    expect(shell.style.width).toBe('432px');
    expect(shell.style.getPropertyValue('--prism-panel-hidden')).toBe('0%');
    expect(reveal.style.clipPath).toBe('inset(0 0 0 calc(var(--prism-panel-hidden) + 1px))');
    expect(edge.style.width).toBe('432px');
    expect(edge.style.transform).toBe('translateX(var(--prism-panel-hidden))');
    expect(getComputedStyle(edge).borderLeftWidth).toBe('1px');
    expect(resizeHandle.style.pointerEvents).toBe('auto');
    expect(getComputedStyle(shell).transitionProperty).toContain('--prism-panel-hidden');
    const edgeRect = edge.getBoundingClientRect();
    const resizeHandleRect = resizeHandle.getBoundingClientRect();
    expect(resizeHandleRect.left + resizeHandleRect.width / 2).toBeCloseTo(edgeRect.left);

    for (const animation of shell.getAnimations()) animation.finish();
    await tick();
    resizeHandle.style.pointerEvents = 'none';
    const shellRect = shell.getBoundingClientRect();
    const borderCenterHit = document.elementFromPoint(shellRect.left + 0.5, shellRect.top + 10);
    const panelInnerHit = document.elementFromPoint(shellRect.left + 1.5, shellRect.top + 10);
    if (!borderCenterHit || !panelInnerHit) throw new Error('Expected the PRISM panel boundary to be inside the test viewport');
    expect(reveal.contains(borderCenterHit)).toBe(false);
    expect(reveal.contains(panelInnerHit)).toBe(true);
    expect(target.textContent).toContain('앗! 문제가 생겼어요');
    expect(target.textContent).toContain('잠시 후 다시 시도해 주세요.');
    expect(retry?.textContent).toContain('다시 시도');
    expect(document.activeElement).toBe(retry);
    expect(consoleError).toHaveBeenCalledWith(prismPanelBoundaryHarness.error);
    expect(Sentry.captureException).toHaveBeenCalledOnce();
    expect(Sentry.captureException).toHaveBeenCalledWith(prismPanelBoundaryHarness.error);

    if (!retry) throw new Error('Expected the PRISM failure panel to render');
    retry.click();
    await vi.waitFor(() => expect(target.querySelector('[data-prism-panel-test-healthy]')).not.toBeNull());
    expect(app.preference.current.prismPanelOpen).toBe(true);
    expect(Sentry.captureException).toHaveBeenCalledOnce();
  });

  test('keeps the closed visual panel laid out behind a synchronized reveal edge', async () => {
    app.preference.current.prismPanelOpen = false;
    const target = document.createElement('div');
    Object.assign(target.style, { display: 'flex', height: '640px', position: 'relative', width: '900px' });
    document.body.append(target);
    component = mount(PrismPanelBoundaryTestHost, { target });

    await vi.waitFor(() => expect(target.querySelector('[role="alert"]')).not.toBeNull());
    await tick();

    const shell = target.querySelector<HTMLElement>('[data-prism-panel-shell]');
    const spacer = target.querySelector<HTMLElement>('[data-prism-panel-spacer]');
    const reveal = target.querySelector<HTMLElement>('[data-prism-panel-reveal]');
    const resizeHandle = target.querySelector<HTMLElement>('[data-prism-panel-resize-handle]');

    if (!shell) throw new Error('Expected the PRISM panel shell to render');
    expect(spacer?.style.width).toBe('0px');
    expect(shell.style.width).toBe('432px');
    expect(shell.style.getPropertyValue('--prism-panel-hidden')).toBe('100%');
    expect(reveal?.style.pointerEvents).toBe('none');
    expect(resizeHandle?.style.pointerEvents).toBe('none');
  });
});
