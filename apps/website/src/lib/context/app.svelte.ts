import { getContext, setContext } from 'svelte';
import { LocalStore, SessionStore } from '../state';

type AppPreference = {
  postsExpanded: 'open' | 'closed' | false;
  postsWidth: number;
  panelExpanded: boolean;
  noteExpanded: boolean;

  focusDuration: number;
  restDuration: number;

  announcementViewedIds?: string[];

  typewriterEnabled: boolean;
  typewriterPosition: number;

  lineHighlightEnabled: boolean;
};

type AppState = {
  ancestors: string[];
  current?: string;

  postsOpen: boolean;
  commandPaletteOpen: boolean;
  shareOpen: string | false;
  statsOpen: boolean;
  upgradeOpen: boolean;

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
    commandPaletteOpen: false,
    shareOpen: false,
    statsOpen: false,
    upgradeOpen: false,

    progress: {
      totalCharacterCount: 0,
      totalBlobSize: 0,
    },
  });

  const context: AppContext = {
    preference: new LocalStore<AppPreference>(`typie:pref:${userId}`, {
      postsExpanded: false,
      postsWidth: 240,
      panelExpanded: true,
      noteExpanded: false,

      focusDuration: 30,
      restDuration: 10,

      typewriterEnabled: false,
      typewriterPosition: 0.5,

      lineHighlightEnabled: true,
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
