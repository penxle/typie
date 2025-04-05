import { getContext, setContext } from 'svelte';
import { LocalStore } from '../state';

type AppPreference = {
  sidebarExpanded: boolean;
};

type AppState = {
  sidebarPopoverVisible: boolean;
};

type AppContext = {
  preference: LocalStore<AppPreference>;
  state: AppState;
};

const key: unique symbol = Symbol('AppContext');

export const getAppContext = () => {
  return getContext<AppContext>(key);
};

export const setupAppContext = () => {
  const appState = $state<AppState>({
    sidebarPopoverVisible: false,
  });

  const context: AppContext = {
    preference: new LocalStore('typie:pref', {
      sidebarExpanded: true,
    }),
    state: appState,
  };

  setContext(key, context);
};
