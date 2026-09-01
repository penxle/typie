import { LocalStore, SessionStore } from '../state';
import { createStableContext } from './stable-context';

export type AppPreference = {
  sidebarWidth: number;
  sidebarHidden: boolean;
  sidebarNavigationClip: number;
  sidebarRecentDocumentsOpen: boolean;
  sidebarRecentDocumentsSort: 'VIEWED_AT' | 'UPDATED_AT';
  sidebarAllDocumentsOpen: boolean;
  hasOpenedPanelOnce: boolean;

  panelWidth: number;

  prismPanelOpen: boolean;
  prismPanelWidth: number;
  prismNotificationSoundEnabled: boolean;
  prismWelcomeObjectEnabled: boolean;
  prismHdrEnabled: boolean;

  defaultPrimaryToolbar: 'insert' | 'format';

  trashHeight: number;

  focusDuration: number;
  restDuration: number;

  typewriterEnabled: boolean;
  typewriterPosition: number;

  lineHighlightEnabled: boolean;

  recentEditMarksEnabled: boolean;

  autoSurroundEnabled: boolean;

  zenModeEnabled: boolean;

  contextBarPinned: boolean;

  searchMatchWholeWord: boolean;

  exportFormat: 'DOCX' | 'EPUB' | 'HWP' | 'PDF';

  referralWelcomeModalShown: boolean;

  planChangeNoticeShown: boolean;
  changelogSeenId: string;

  initialPage: 'blank' | 'last';

  widgetHidden: boolean;

  currentSiteId?: string;
  trialReminderLastShownDate?: string;
  prismToolPolicy?: 'READ_ONLY' | 'STANDARD' | 'FULL';
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
  changelogOpen: boolean;
  goalOpen: string[];
  userGoalOpen: boolean;
  shortcutsOpen: boolean;
  sidebarPeek: boolean;
  prismBadge: boolean;
  prismViewingSessionId: string | null;

  subscribed: boolean;

  usage: {
    current: { totalCharacterCount: number; totalBlobSize: string };
    limit: { totalCharacterCount: number; totalBlobSize: string };
  };

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

const [getAppContext, setAppContext, tryAppContext] = createStableContext<AppContext>('ui.AppContext');

export { getAppContext, tryAppContext };

export const setupAppContext = (userId: string) => {
  const appState = $state<AppState>({
    ancestors: [],
    trashOpen: false,
    commandPaletteOpen: false,
    notesOpen: false,
    shareOpen: [],
    exportOpen: null,
    statsOpen: false,
    changelogOpen: false,
    goalOpen: [],
    userGoalOpen: false,
    shortcutsOpen: false,
    sidebarPeek: false,
    prismBadge: false,
    prismViewingSessionId: null,

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
      sidebarNavigationClip: 0,
      sidebarRecentDocumentsOpen: true,
      sidebarRecentDocumentsSort: 'VIEWED_AT',
      sidebarAllDocumentsOpen: true,

      hasOpenedPanelOnce: false,
      panelWidth: 250,

      prismPanelOpen: false,
      prismPanelWidth: 420,
      prismNotificationSoundEnabled: true,
      prismWelcomeObjectEnabled: true,
      prismHdrEnabled: true,

      defaultPrimaryToolbar: 'format',

      trashHeight: 300,

      focusDuration: 30,
      restDuration: 10,

      typewriterEnabled: false,
      typewriterPosition: 0.5,

      lineHighlightEnabled: true,

      recentEditMarksEnabled: true,

      autoSurroundEnabled: true,

      zenModeEnabled: false,

      contextBarPinned: true,

      searchMatchWholeWord: false,

      exportFormat: 'PDF',

      referralWelcomeModalShown: false,

      planChangeNoticeShown: false,
      changelogSeenId: '',

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

  setAppContext(context);

  return context;
};
