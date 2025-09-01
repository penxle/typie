import { getContext, setContext } from 'svelte';
import { LocalStore, SessionStore } from '../state';
import type { PageLayout } from '../utils';

export type AppPreference = {
  postsExpanded: 'open' | 'closed' | false;
  postsWidth: number;
  panelExpandedByViewId: Record<string, boolean>;
  panelTabByViewId: Record<string, 'info' | 'spellcheck' | 'settings'>;
  panelInfoTabByViewId: Record<string, 'post' | 'anchors'>;

  panelWidth: number;

  toolbarStyle: 'compact' | 'classic';

  noteExpanded: boolean;
  trashHeight: number;

  focusDuration: number;
  restDuration: number;

  announcementViewedIds?: string[];

  typewriterEnabled: boolean;
  typewriterPosition: number;

  lineHighlightEnabled: boolean;

  zenModeEnabled: boolean;

  searchMatchWholeWord: boolean;

  experimental_pdfExportEnabled: boolean;
  experimental_splitViewEnabled: boolean;

  lastPdfPageLayout: PageLayout | null;

  referralWelcomeModalShown: boolean;

  initialPage: 'blank' | 'last';
};

type AppState = {
  ancestors: string[];
  current?: string;

  postsOpen: boolean;
  trashOpen: boolean;
  commandPaletteOpen: boolean;
  shareOpen: string[];
  statsOpen: boolean;
  upgradeOpen: boolean;
  findReplaceOpenByViewId: Record<string, boolean>;

  progress: {
    totalCharacterCount: number;
    totalBlobSize: number;
  };

  newFolderId?: string;
};

type AppTimerState = {
  status: 'focus' | 'rest' | 'init';
  currentTime: number;
  paused: boolean;
  keepFocus: boolean;
};

type AppContext = {
  preference: LocalStore<AppPreference>;
  state: AppState;
  timerState: SessionStore<AppTimerState>;
};

const key: unique symbol = Symbol('AppContext');

export const getAppContext = () => {
  return getContext<AppContext>(key);
};

export const setupAppContext = (userId: string) => {
  const appState = $state<AppState>({
    ancestors: [],
    postsOpen: false,
    trashOpen: false,
    commandPaletteOpen: false,
    shareOpen: [],
    statsOpen: false,
    upgradeOpen: false,
    findReplaceOpenByViewId: {},

    progress: {
      totalCharacterCount: 0,
      totalBlobSize: 0,
    },
  });

  const context: AppContext = {
    preference: new LocalStore<AppPreference>(`typie:pref:${userId}`, {
      postsExpanded: false,
      postsWidth: 240,
      panelExpandedByViewId: {},
      panelTabByViewId: {},
      panelInfoTabByViewId: {},
      panelWidth: 250,

      toolbarStyle: 'compact',

      noteExpanded: false,
      trashHeight: 300,

      focusDuration: 30,
      restDuration: 10,

      typewriterEnabled: false,
      typewriterPosition: 0.5,

      lineHighlightEnabled: true,

      zenModeEnabled: false,

      searchMatchWholeWord: false,

      experimental_pdfExportEnabled: false,
      experimental_splitViewEnabled: false,

      lastPdfPageLayout: null,

      referralWelcomeModalShown: false,

      initialPage: 'last',
    }),
    state: appState,
    timerState: new SessionStore<AppTimerState>(`typie:timer:${userId}`, {
      status: 'init',
      currentTime: 0,
      paused: false,
      keepFocus: false,
    }),
  };

  setContext(key, context);

  return context;
};
