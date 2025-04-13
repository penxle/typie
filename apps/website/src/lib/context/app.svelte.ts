import { getContext, setContext } from 'svelte';
import { LocalStore, SessionStore } from '../state';

type AppPreference = {
  sidebarExpanded: boolean;
  toolbarHidden: boolean;
  focusDuration: number;
  restDuration: number;
};

type AppState = {
  sidebarTriggered: boolean;
  commandPaletteOpen: boolean;
  toolbarActive: boolean;
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
    sidebarTriggered: false,
    commandPaletteOpen: false,
    toolbarActive: false,
  });

  const context: AppContext = {
    preference: new LocalStore('typie:pref', {
      sidebarExpanded: true,
      toolbarHidden: false,
      focusDuration: 30,
      restDuration: 10,
    }),
    state: appState,
    timerState: new SessionStore('typie:timer', {
      status: 'init',
      currentTime: 0,
      paused: false,
      keepFocus: false,
    }),
  };

  setContext(key, context);
};
