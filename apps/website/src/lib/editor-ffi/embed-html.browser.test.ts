import { createRawSnippet, mount, tick, unmount } from 'svelte';
import { fromStore, writable } from 'svelte/store';
import { afterEach, describe, expect, it, vi } from 'vitest';
import EmbedHtml from './components/EmbedHtml.svelte';

const cleanups = new Set<() => Promise<void>>();

afterEach(async () => {
  for (const cleanup of cleanups) await cleanup();
  vi.restoreAllMocks();
  document.body.replaceChildren();
});

async function mountHtml(html: string, layoutReady = true) {
  const scrollRoot = document.createElement('div');
  scrollRoot.style.cssText = 'height: 200px; width: 400px; overflow: auto';
  const spacer = document.createElement('div');
  spacer.style.height = '2000px';
  const box = document.createElement('div');
  const host = document.createElement('div');
  host.style.display = 'contents';
  box.append(host);
  scrollRoot.append(spacer, box);
  document.body.append(scrollRoot);
  const ready = fromStore(writable(layoutReady));
  const component = mount(EmbedHtml, {
    target: host,
    props: {
      html,
      scrollRoot,
      get layoutReady() {
        return ready.current;
      },
      fallback: createRawSnippet(() => ({ render: () => '<div data-embed-fallback>Unavailable embed</div>' })),
    },
  });
  const destroy = async () => {
    cleanups.delete(destroy);
    await unmount(component);
  };
  cleanups.add(destroy);
  await tick();
  return { scrollRoot, box, host, ready, destroy };
}

async function waitForIframely() {
  const { iframely } = await import('@iframely/embed.js');
  const unrelated = document.createElement('iframe');
  unrelated.srcdoc = '';
  document.body.append(unrelated);
  // Importing the SDK does not await our adapter. Its configured lookup must
  // exclude other browsing contexts before we exercise message handling.
  await expect.poll(() => iframely.findIframe({ contentWindow: unrelated.contentWindow })).toBeUndefined();
  unrelated.remove();
  return iframely;
}

function sendMessage(frame: HTMLIFrameElement | null, message: Record<string, unknown>) {
  expect(frame?.contentWindow).toBeTruthy();
  window.dispatchEvent(
    new MessageEvent('message', {
      source: frame?.contentWindow,
      origin: 'https://iframely.net',
      data: JSON.stringify(message),
    }),
  );
}

describe('embed HTML layout and iframe mounting', () => {
  it.each([
    ['responsive player', 'height: 0; padding-bottom: 56.25%', 225],
    ['fixed height player', 'height: 152px', 152],
    ['ratio with extra space', 'height: 0; padding-bottom: 63%; padding-top: 284px', 536],
  ] as const)('preserves Iframely %s sizing without a browsing context', async (_, size, height) => {
    const { scrollRoot, box, host } = await mountHtml(
      `<div style="position: relative; width: 100%; ${size}"><iframe src="about:blank" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0"></iframe></div>`,
    );
    expect(box.getBoundingClientRect().height).toBeCloseTo(height, 1);
    expect(host.querySelector('iframe')).toBeNull();
    scrollRoot.scrollTop = 2000;
    await expect.poll(() => host.querySelector('iframe')?.contentWindow).toBeTruthy();
    expect(box.getBoundingClientRect().height).toBeCloseTo(height, 1);
    const frame = host.querySelector('iframe');
    scrollRoot.scrollTop = 0;
    await new Promise((resolve) => requestAnimationFrame(resolve));
    expect(host.querySelector('iframe')).toBe(frame);
  });

  it('keeps an iframe with its own in-flow height mounted offscreen', async () => {
    const { host } = await mountHtml('<iframe src="about:blank" style="height: 152px; border: 0"></iframe>');
    expect(host.querySelector('iframe')?.contentWindow).toBeTruthy();
  });

  it('waits for layout before deciding which iframes are near the viewport', async () => {
    const { scrollRoot, host, ready } = await mountHtml(
      '<div style="height: 120px; position: relative"><iframe src="about:blank" style="position: absolute"></iframe></div>',
      false,
    );
    scrollRoot.scrollTop = 2000;
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    expect(host.querySelector('iframe')).toBeNull();
    scrollRoot.scrollTop = 0;
    ready.current = true;
    await tick();
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    expect(host.querySelector('iframe')).toBeNull();
    scrollRoot.scrollTop = 2000;
    await expect.poll(() => host.querySelector('iframe')).not.toBeNull();
  });

  it("applies hosted widgets' size messages while offscreen", async () => {
    const { host, box } = await mountHtml(
      '<div style="height: 400px; position: relative"><iframe src="https://iframely.net/example" srcdoc="" sandbox style="position: absolute; width: 100%; height: 100%"></iframe></div>',
    );
    const frame = host.querySelector('iframe');
    expect(frame?.contentWindow).toBeTruthy();
    await waitForIframely();
    sendMessage(frame, { method: 'resize', height: 600 });
    expect(box.getBoundingClientRect().height).toBe(600);
    expect(host.querySelector('iframe')).toBe(frame);

    const open = vi.spyOn(window, 'open').mockReturnValue(null);
    sendMessage(frame, { method: 'open-href', href: 'javascript:alert(1)' });
    expect(open).not.toHaveBeenCalled();

    window.dispatchEvent(
      new MessageEvent('message', {
        source: window,
        data: JSON.stringify({ method: 'resize', height: 900 }),
      }),
    );
    expect(box.getBoundingClientRect().height).toBe(600);

    sendMessage(frame, { method: 'cancelWidget' });
    await expect.poll(() => host.querySelector('[data-embed-fallback]')).not.toBeNull();
    expect(host.querySelector('iframe')).toBeNull();
  });

  it.each([
    ['native lazy iframe', true, ''],
    ['SDK lazy content', false, ''],
    ['SDK lazy content with thumbnail', true, 'data-img'],
  ] as const)('preserves offscreen sizing and loading policy for %s', async (_, nativeLazy, thumbnail) => {
    const { iframely } = await import('@iframely/embed.js');
    const sdk = iframely as typeof iframely & { SUPPORT_IFRAME_LOADING_ATTR: boolean };
    const previousNativeLazy = sdk.SUPPORT_IFRAME_LOADING_ATTR;
    sdk.SUPPORT_IFRAME_LOADING_ATTR = nativeLazy;
    try {
      const { host, box } = await mountHtml(
        `<div style="height: 400px; position: relative"><iframe data-iframely-url="https://iframely.net/example" ${thumbnail} srcdoc="" sandbox style="position: absolute; width: 100%; height: 100%"></iframe></div>`,
      );
      const frame = host.querySelector('iframe');
      await expect.poll(() => frame?.getAttribute('src')).toContain('https://iframely.net/example');
      await waitForIframely();
      expect(box.getBoundingClientRect().height).toBe(400);
      expect(frame?.hasAttribute('data-iframely-url')).toBe(false);
      if (nativeLazy && !thumbnail) {
        expect(frame?.loading).toBe('lazy');
      } else {
        expect(new URL(frame?.src ?? '').searchParams.get('lazy')).toBe('1');
      }
      if (thumbnail) {
        expect(frame?.hasAttribute('data-img-created')).toBe(true);
        expect(frame?.parentElement?.querySelector('div')).not.toBeNull();
      }
      sendMessage(frame, { method: 'resize', height: 600 });
      expect(box.getBoundingClientRect().height).toBe(600);
    } finally {
      sdk.SUPPORT_IFRAME_LOADING_ATTR = previousNativeLazy;
    }
  });

  it('releases SDK observations per iframe and discards empty shared observers', async () => {
    const observe = vi.spyOn(IntersectionObserver.prototype, 'observe');
    const unobserve = vi.spyOn(IntersectionObserver.prototype, 'unobserve');
    const disconnect = vi.spyOn(IntersectionObserver.prototype, 'disconnect');
    const html =
      '<div style="height: 400px; position: relative"><iframe src="https://iframely.net/example" srcdoc="" sandbox style="position: absolute; width: 100%; height: 100%"></iframe></div>';
    const first = await mountHtml(html);
    const second = await mountHtml(html);
    await waitForIframely();
    const firstFrame = first.host.querySelector('iframe');
    const secondFrame = second.host.querySelector('iframe');
    const request = { method: 'send-intersections', options: { margin: 1379 } };
    sendMessage(firstFrame, request);
    sendMessage(secondFrame, request);
    expect(observe.mock.calls).toEqual([[firstFrame], [secondFrame]]);
    const observer = observe.mock.contexts[0];
    expect(observe.mock.contexts[1]).toBe(observer);

    await first.destroy();
    expect(unobserve).toHaveBeenCalledExactlyOnceWith(firstFrame);
    expect(disconnect).not.toHaveBeenCalled();
    sendMessage(secondFrame, { method: 'resize', height: 600 });
    expect(second.box.getBoundingClientRect().height).toBe(600);

    await second.destroy();
    expect(unobserve).toHaveBeenLastCalledWith(secondFrame);
    expect(disconnect.mock.contexts).toEqual([observer]);

    const third = await mountHtml(html);
    sendMessage(third.host.querySelector('iframe'), request);
    expect(observe.mock.contexts[2]).not.toBe(observer);
  });

  it.each(['unmount', 'rendered'] as const)('releases thumbnail waiting work on %s', async (completion) => {
    const timeout = vi.spyOn(window, 'setTimeout');
    const clear = vi.spyOn(window, 'clearTimeout');
    const { host, destroy } = await mountHtml(
      '<div style="height: 400px; position: relative"><iframe data-iframely-url="https://iframely.net/example" data-img srcdoc="" sandbox style="position: absolute; width: 100%; height: 100%"></iframe></div>',
    );
    const frame = host.querySelector('iframe');
    await expect.poll(() => frame?.hasAttribute('data-img-created')).toBe(true);
    await waitForIframely();
    expect(frame).not.toBeNull();
    if (!frame) return;
    const removeListener = vi.spyOn(frame, 'removeEventListener');
    const waitingIndex = timeout.mock.calls.findIndex(([, delay]) => delay === 3000);
    expect(waitingIndex).toBeGreaterThanOrEqual(0);
    const waitingTimer = timeout.mock.results[waitingIndex].value;

    if (completion === 'unmount') {
      await destroy();
    } else {
      sendMessage(frame, { method: 'widgetRendered' });
      expect(frame.parentElement?.querySelector('div')).toBeNull();
    }
    expect(clear).toHaveBeenCalledWith(waitingTimer);
    expect(removeListener).toHaveBeenCalledWith('load', expect.any(Function), false);
    timeout.mockClear();
    frame.dispatchEvent(new Event('load'));
    frame.dispatchEvent(new Event('load'));
    expect(timeout).not.toHaveBeenCalled();
    if (completion === 'rendered') {
      sendMessage(frame, { method: 'end-waiting-widget-render' });
      expect(timeout).not.toHaveBeenCalled();
    }
  });

  it('cancels a pending thumbnail fade on unmount', async () => {
    const timeout = vi.spyOn(window, 'setTimeout');
    const clear = vi.spyOn(window, 'clearTimeout');
    const { host, destroy } = await mountHtml(
      '<div style="height: 400px; position: relative"><iframe data-iframely-url="https://iframely.net/example" data-img srcdoc="" sandbox style="position: absolute; width: 100%; height: 100%"></iframe></div>',
    );
    const frame = host.querySelector('iframe');
    await expect.poll(() => frame?.hasAttribute('data-img-created')).toBe(true);
    // The browser may already have delivered the first load for srcdoc.
    frame?.dispatchEvent(new Event('load'));
    frame?.dispatchEvent(new Event('load'));
    const fadeIndex = timeout.mock.calls.findIndex(([, delay]) => delay === 200);
    expect(fadeIndex).toBeGreaterThanOrEqual(0);
    await destroy();
    expect(clear).toHaveBeenCalledWith(timeout.mock.results[fadeIndex].value);
  });

  it('keeps scripts inert as with the original HTML insertion', async () => {
    const { host } = await mountHtml('<script>document.body.dataset.embedScriptRan = "true"</script><div>Embed</div>');
    await new Promise((resolve) => requestAnimationFrame(resolve));
    expect(host.textContent).toContain('Embed');
    expect(document.body.dataset.embedScriptRan).toBeUndefined();
  });

  it('does not mount a deferred iframe after cleanup', async () => {
    const { scrollRoot, host, destroy } = await mountHtml(
      '<div style="height: 120px; position: relative"><iframe src="about:blank" style="position: absolute"></iframe></div>',
    );
    await destroy();
    scrollRoot.scrollTop = 2000;
    await new Promise((resolve) => requestAnimationFrame(resolve));
    expect(host.querySelector('iframe')).toBeNull();
  });
});
