/* eslint-disable @typescript-eslint/consistent-type-definitions */

import type { DocumentSaveDialogAction, DocumentSaveDialogData, TabIcon } from '@typie/lib/desktop';

declare global {
  type TabState = { id: string; title: string; url: string; icon: TabIcon | null; saving: boolean };
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
    onCloseTabRequest?: (callback: () => void) => () => void;
    onTheme?: (callback: (theme: ThemePayload) => void) => () => void;
    login?: () => Promise<string | undefined>;
    cancelLogin?: () => void;
    retry?: () => void;
    onAuthError?: (callback: (message: string) => void) => () => void;
    onUpdateReady?: (callback: () => void) => () => void;
    onFullscreen?: (callback: (fullscreen: boolean) => void) => () => void;
    restartToUpdate?: () => void;
  };

  interface Window {
    documentSaveDialog: {
      subscribe: (callback: (data: DocumentSaveDialogData) => void) => () => void;
      ready: (id: string, height: number, completed?: boolean) => void;
      respond: (id: string, action: DocumentSaveDialogAction) => void;
      resize: (height: number) => void;
    };
    shell: ShellApi;
  }
}

export {};
