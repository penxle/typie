import { getContext, setContext } from 'svelte';
import { LocalStore, SessionStore } from '../state';

type AppPreference = {
  sidebarExpanded: boolean;
  panelExpanded: boolean;
  focusDuration: number;
  restDuration: number;
};

type AppState = {
  commandPaletteOpen: boolean;
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
    commandPaletteOpen: false,
  });

  const context: AppContext = {
    preference: new LocalStore('typie:pref', {
      sidebarExpanded: true,
      panelExpanded: false,
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

  return context;
};
