type WebView = {
  emitEvent: (name: string, data?: unknown) => void;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  addEventListener: (name: string, listener: (data: any) => void) => void;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  removeEventListener: (name: string, listener: (data: any) => void) => void;
};

declare global {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Window {
    __webview__?: WebView;
  }
}

export {};
