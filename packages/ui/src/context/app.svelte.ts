import { getContext, setContext } from 'svelte';
import { LocalStore, SessionStore } from '../state';

export type AppPreference = {
  sidebarWidth: number;
  sidebarHidden: boolean;
  sidebarTrigger: 'hover' | 'click';
  hasOpenedPanelOnce: boolean;

  panelWidth: number;

  toolbarStyle: 'compact' | 'classic';

  trashHeight: number;

  focusDuration: number;
  restDuration: number;

  announcementViewedIds?: string[];

  typewriterEnabled: boolean;
  typewriterPosition: number;

  lineHighlightEnabled: boolean;

  autoSurroundEnabled: boolean;

  zenModeEnabled: boolean;

  searchMatchWholeWord: boolean;

  exportFormat: 'DOCX' | 'EPUB' | 'HWP' | 'PDF';

  referralWelcomeModalShown: boolean;

  planChangeNoticeShown: boolean;

  initialPage: 'blank' | 'last';

  widgetHidden: boolean;

  currentSiteId?: string;
  trialReminderLastShownDate?: string;
};

type AppState = {
  ancestors: string[];
  current?: string;

  trashOpen: boolean;
  commandPaletteOpen: boolean;
  notesOpen: boolean;
  shareOpen: string[];
  exportOpen: string | null;
  statsOpen: boolean;
  shortcutsOpen: boolean;

  subscribed: boolean;

  usage: {
    current: { totalCharacterCount: number; totalBlobSize: string };
    limit: { totalCharacterCount: number; totalBlobSize: string };
  };

  newFolderId?: string;
  nextCurrentSiteId?: string;

  openMenuCount: number;

  clipboard?: {
    mode: 'copy' | 'cut';
    entityIds: string[];
    sourceSiteId: string;
  };
};

type AppTimerState = {
  status: 'focus' | 'rest' | 'init';
  currentTime: number;
  paused: boolean;
  keepFocus: boolean;
};

export type AppContext = {
  userId: string;
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
    trashOpen: false,
    commandPaletteOpen: false,
    notesOpen: false,
    shareOpen: [],
    exportOpen: null,
    statsOpen: false,
    shortcutsOpen: false,

    subscribed: false,

    usage: {
      current: {
        totalCharacterCount: 0,
        totalBlobSize: '0',
      },
      limit: {
        totalCharacterCount: -1,
        totalBlobSize: '-1',
      },
    },

    openMenuCount: 0,
  });

  const context: AppContext = {
    userId,
    preference: new LocalStore<AppPreference>(`typie:pref:${userId}`, {
      sidebarWidth: 240,
      sidebarHidden: false,
      sidebarTrigger: 'hover',

      hasOpenedPanelOnce: false,
      panelWidth: 250,

      toolbarStyle: 'compact',

      trashHeight: 300,

      focusDuration: 30,
      restDuration: 10,

      typewriterEnabled: false,
      typewriterPosition: 0.5,

      lineHighlightEnabled: true,

      autoSurroundEnabled: true,

      zenModeEnabled: false,

      searchMatchWholeWord: false,

      exportFormat: 'PDF',

      referralWelcomeModalShown: false,

      planChangeNoticeShown: false,

      initialPage: 'last',

      widgetHidden: false,
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
