declare module '@iframely/embed.js' {
  export const iframely: {
    findIframe: (options: { contentWindow: MessageEventSource | null }) => HTMLIFrameElement | undefined;
    openHref: (href: string) => void;
    load: (element: HTMLElement) => void;
    // Added by our package patch to release per-iframe resources.
    unload: (element: HTMLIFrameElement) => void;
    on: (event: 'message', callback: (widget: { iframe: HTMLIFrameElement } | undefined, message: { method?: string }) => void) => void;
  };
}

declare module '*.svg?component' {
  import type { Component } from 'svelte';
  import type { SVGAttributes } from 'svelte/elements';

  const content: Component<SVGAttributes<SVGSVGElement>>;

  // eslint-disable-next-line import/no-default-export
  export default content;
}
