/* eslint-disable @typescript-eslint/consistent-type-definitions */

declare global {
  type TabState = { id: string; title: string; url: string; loading: boolean };
  type TabsStatePayload = { tabs: TabState[]; activeId: string | null };
  type ThemePayload = { theme: 'light' | 'dark'; variantLight: string; variantDark: string };

  type ShellApi = {
    platform: NodeJS.Platform;
    newTab?: () => void;
    closeTab?: (id: string) => void;
    activateTab?: (id: string) => void;
    moveTab?: (id: string, toIndex: number) => void;
    popupMenu?: () => void;
    onTabsState?: (callback: (state: TabsStatePayload) => void) => () => void;
    onTheme?: (callback: (theme: ThemePayload) => void) => () => void;
    login?: () => Promise<string | undefined>;
    cancelLogin?: () => void;
    retry?: () => void;
    onAuthError?: (callback: (message: string) => void) => () => void;
    onUpdateReady?: (callback: () => void) => () => void;
    restartToUpdate?: () => void;
  };

  interface Window {
    shell: ShellApi;
  }
}

export {};
