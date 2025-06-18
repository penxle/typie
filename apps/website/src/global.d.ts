/* eslint-disable @typescript-eslint/consistent-type-definitions */
/* eslint-disable @typescript-eslint/no-explicit-any */

type WebView = {
  emitEvent: (name: string, data?: unknown) => void;
  addEventListener: (name: string, listener: (data: any) => void) => void;
  removeEventListener: (name: string, listener: (data: any) => void) => void;
};

type NodeView = {
  handle?: (event: CustomEvent) => void;
  [key: string]: any;
};

declare global {
  import { Window } from 'happy-dom';

  var __happydom__: {
    Window: Window;
  };

  interface Window {
    __webview__?: WebView;
  }

  interface Node {
    __nodeview__?: NodeView;
  }
}

export {};
