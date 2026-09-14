import { captureException } from '@sentry/sveltekit';

const cancellations = new Map<HTMLIFrameElement, () => void>();
let ready: Promise<(typeof import('@iframely/embed.js'))['iframely'] | undefined> | undefined;

export function connectIframely(frames: HTMLIFrameElement[], onCancel: () => void): (() => void) | undefined {
  const hosted = frames.filter(isIframelyIframe);
  if (hosted.length === 0) return;

  for (const frame of hosted) {
    cancellations.set(frame, onCancel);
  }
  const loaded = loadIframely();
  void loaded.then((iframely) => {
    for (const frame of hosted) {
      if (cancellations.get(frame) !== onCancel || !frame.isConnected) continue;
      if (!iframely) {
        onCancel();
        break;
      }
      // SDK initialization runs once, but lazy markup can be inserted later.
      try {
        iframely.load(frame);
      } catch (err) {
        captureException(err);
        onCancel();
        break;
      }
    }
  });

  return () => {
    for (const frame of hosted) cancellations.delete(frame);
    void loaded.then((iframely) => {
      for (const frame of hosted) iframely?.unload(frame);
    });
  };
}

function loadIframely() {
  if (ready) return ready;
  // omit_script=1 removes the library from API responses. It is still needed
  // to apply hosted widgets' size messages to their HTML wrappers.
  ready = import('@iframely/embed.js')
    .then(({ iframely }) => {
      // The library also handles messages from unrelated iframes by default.
      // Limit it to hosted widgets mounted by our embed renderer.
      iframely.findIframe = ({ contentWindow }) =>
        [...cancellations.keys()].find((frame) => frame.isConnected && frame.contentWindow === contentWindow && isIframelyIframe(frame));
      iframely.openHref = (href) => {
        const url = httpUrl(href);
        if (url) window.open(url.href, '_blank', 'noopener');
      };
      iframely.on('message', (widget, message) => {
        // The SDK's later `cancel` event is omitted when a Content ID widget has
        // no original URL. Route by iframe identity instead of URL or DOM shape.
        if (widget && message.method === 'cancelWidget') cancellations.get(widget.iframe)?.();
      });
      return iframely;
    })
    .catch((err: unknown): undefined => {
      ready = undefined;
      captureException(err);
    });
  return ready;
}

export function isIframelyIframe(frame: HTMLIFrameElement): boolean {
  try {
    const { hostname } = new URL(frame.getAttribute('src') || frame.dataset.iframelyUrl || '', document.baseURI);
    return ['iframe.ly', 'iframely.net', 'if-cdn.com'].some((domain) => hostname === domain || hostname.endsWith(`.${domain}`));
  } catch {
    return false;
  }
}

function httpUrl(href: string): URL | undefined {
  try {
    const url = new URL(href);
    return url.protocol === 'https:' || url.protocol === 'http:' ? url : undefined;
  } catch {
    return undefined;
  }
}
