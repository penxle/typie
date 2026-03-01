/* eslint-disable @typescript-eslint/consistent-type-definitions */
/* eslint-disable @typescript-eslint/no-explicit-any */

import '../.mearie/graphql.d.ts';

type WebView = {
  platform: 'android' | 'ios';
  emitEvent: (name: string, data?: unknown) => void;
  addEventListener: (name: string, listener: (data: any) => void) => void;
  removeEventListener: (name: string, listener: (data: any) => void) => void;
  setProcedure: (name: string, handler: (data: any) => unknown | Promise<unknown>) => void;
};

type NodeView = {
  handle?: (event: CustomEvent) => void;
  [key: string]: any;
};

declare global {
  var fbq: ((type: 'track', name: string, data?: Record<string, unknown>) => void) | undefined;

  interface Window {
    __webview__?: WebView;
  }

  interface Node {
    __nodeview__?: NodeView;
  }
}

// eslint-disable-next-line unicorn/require-module-specifiers
export {};
