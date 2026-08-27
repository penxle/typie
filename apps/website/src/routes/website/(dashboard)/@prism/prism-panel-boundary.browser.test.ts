import '../../../../app.css';

import * as Sentry from '@sentry/sveltekit';
import { getAppContext } from '@typie/ui/context';
import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import PrismPanelBoundaryTestHost, { prismPanelBoundaryHarness } from './PrismPanelBoundaryTestHost.svelte';

vi.mock('@sentry/sveltekit', () => ({ captureException: vi.fn() }));
vi.mock('@typie/ui/context', () => ({ getAppContext: vi.fn() }));

const app = {
  preference: {
    current: {
      prismPanelOpen: true,
      prismPanelWidth: 432,
      zenModeEnabled: false,
    },
  },
  state: { prismAccess: true },
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
  app.state.prismAccess = true;
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
    Object.assign(target.style, { display: 'flex', height: '640px', position: 'relative', width: '900px' });
    document.body.append(target);
    component = mount(PrismPanelBoundaryTestHost, { target });

    await vi.waitFor(() => expect(target.querySelector('[role="alert"]')).not.toBeNull());
    await tick();

    const shell = target.querySelector<HTMLElement>('[data-prism-panel-shell]');
    const spacer = target.querySelector<HTMLElement>('[data-prism-panel-spacer]');
    const retry = target.querySelector<HTMLButtonElement>('button:not([aria-label])');
    expect(shell?.style.width).toBe('432px');
    expect(shell?.style.transform).toBe('scale(1)');
    expect(spacer?.style.width).toBe('432px');
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
});
