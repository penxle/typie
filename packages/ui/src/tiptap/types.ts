declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Storage {
    contexts: Map<unknown, unknown>;
    webviewFeatures?: string[];
  }
}
