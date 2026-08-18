export type TypieDesktopBridge = {
  version: string;
  platform: 'darwin' | 'win32';
  openExternal: (url: string) => Promise<void>;
  on: (event: 'focus', callback: () => void) => () => void;
};
