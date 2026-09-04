export type TabIcon = { icon: string; color: string | null };

export type DesktopZoomAction = 'in' | 'out' | 'reset';

export type DesktopBridgeListeners = {
  focus: () => void;
  preference: () => void;
  'zoom-shortcut': (action: DesktopZoomAction) => boolean;
};

export type TypieDesktopBridge = {
  version: string;
  platform: 'darwin' | 'win32';
  openExternal: (url: string) => Promise<void>;
  on: <Event extends keyof DesktopBridgeListeners>(event: Event, callback: DesktopBridgeListeners[Event]) => () => void;
  setTabIcon?: (icon: TabIcon) => void;
  openTab?: (url: string) => void;
};
