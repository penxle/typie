import { getContext, setContext } from 'svelte';
import { LocalStore, SessionStore } from '../state';

export type AppPreference = {
  postsExpanded: 'open' | 'closed' | false;
  panelExpanded: boolean;
  focusDuration: number;
  restDuration: number;
  currentPage?: string;
};

type AppState = {
  ancestors: string[];
  current?: string;

  postsOpen: boolean;
  commandPaletteOpen: boolean;
  shareOpen: string | false;
  statsOpen: boolean;

  progress: {
    totalCharacterCount: number;
    totalBlobSize: number;
  };
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

export const setupAppContext = () => {
  const appState = $state<AppState>({
    ancestors: [],
    postsOpen: false,
    commandPaletteOpen: false,
    shareOpen: false,
    statsOpen: false,

    progress: {
      totalCharacterCount: 0,
      totalBlobSize: 0,
    },
  });

  const context: AppContext = {
    preference: new LocalStore<AppPreference>('typie:pref', {
      postsExpanded: false,
      panelExpanded: true,
      focusDuration: 30,
      restDuration: 10,
    }),
    state: appState,
    timerState: new SessionStore<AppTimerState>('typie:timer', {
      status: 'init',
      currentTime: 0,
      paused: false,
      keepFocus: false,
    }),
  };

  setContext(key, context);

  return context;
};
