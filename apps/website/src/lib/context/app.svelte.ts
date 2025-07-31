import { getContext, setContext } from 'svelte';
import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import { LocalStore, SessionStore } from '../state';
import type { TreeEntity } from '../../routes/website/(dashboard)/@tree/@selection/types';

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

  zenModeEnabled: boolean;

  searchMatchWholeWord: boolean;

  experimental_pageEnabled: boolean;
  experimental_pageLayoutId?: string;
};

type AppState = {
  ancestors: string[];
  current?: string;

  postsOpen: boolean;
  commandPaletteOpen: boolean;
  shareOpen: string | false;
  statsOpen: boolean;
  upgradeOpen: boolean;
  findReplaceOpen: boolean;

  progress: {
    totalCharacterCount: number;
    totalBlobSize: number;
  };

  tree: {
    entities: TreeEntity[];
    entityMap: SvelteMap<string, TreeEntity>;
    lastSelectedEntityId?: string;
    selectedEntityIds: SvelteSet<string>;
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
    findReplaceOpen: false,

    progress: {
      totalCharacterCount: 0,
      totalBlobSize: 0,
    },
    tree: {
      entities: [],
      entityMap: new SvelteMap<string, TreeEntity>(),
      lastSelectedEntityId: undefined,
      selectedEntityIds: new SvelteSet<string>(),
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

      zenModeEnabled: false,

      searchMatchWholeWord: false,

      experimental_pageEnabled: false,
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
