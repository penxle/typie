export type TabIcon = { icon: string; color: string | null };

export type TypieDesktopBridge = {
  version: string;
  platform: 'darwin' | 'win32';
  openExternal: (url: string) => Promise<void>;
  on: (event: 'focus' | 'preference', callback: () => void) => () => void;
  setTabIcon?: (icon: TabIcon) => void;
  openTab?: (url: string) => void;
};
