import path from 'node:path';
import { app, ipcMain, Menu, nativeTheme, screen, session, shell, webContents } from 'electron';
import { AuthService } from './auth-service';
import { showContextMenu } from './context-menu';
import { env } from './env';
import { IPC } from './ipc';
import { buildMenu } from './menu';
import { NavigationPolicy } from './navigation-policy';
import { initSentry } from './sentry';
import { Store } from './store';
import { TabManager } from './tab-manager';
import { Updater } from './updater';
import { WindowManager } from './window-manager';
import type { ContextMenuRequest } from './context-menu';
import type { WindowState } from './window-manager';

type ThemePayload = { theme: 'light' | 'dark'; variantLight: string; variantDark: string };

app.setName('타이피');
app.setPath('userData', path.join(app.getPath('appData'), 'Typie'));
app.userAgentFallback = `${app.userAgentFallback} Typie/${app.getVersion()}`;

initSentry(app.getVersion());

let windowManager: WindowManager | undefined;
let tabManager: TabManager | undefined;
let policy: NavigationPolicy | undefined;
let menu: Menu | undefined;
const auth = new AuthService(env);
const store = new Store();
const updater = new Updater(app.isPackaged);
const themes = new Map<number, ThemePayload>();

let quitting = false;
const singleInstance = app.requestSingleInstanceLock();

if (singleInstance) {
  app.on('second-instance', () => {
    if (!windowManager) return;
    if (windowManager.window.isMinimized()) windowManager.window.restore();
    windowManager.show();
    windowManager.window.focus();
  });
} else {
  app.quit();
}

app.on('before-quit', () => {
  quitting = true;
});

const osTheme = () => (nativeTheme.shouldUseDarkColors ? 'dark' : 'light');

const chromeWebContents = () => {
  const chrome = windowManager?.chrome.webContents;
  return chrome && !chrome.isDestroyed() ? chrome : null;
};

const sanitizeWindowState = (state: WindowState): WindowState => {
  if (!state.bounds) return state;
  const { bounds } = state;
  const visible = screen.getAllDisplays().some((display) => {
    const area = display.workArea;
    return (
      bounds.x < area.x + area.width &&
      bounds.x + bounds.width > area.x &&
      bounds.y < area.y + area.height &&
      bounds.y + bounds.height > area.y
    );
  });
  return visible ? state : { maximized: state.maximized };
};

const showLoginKeepingTabs = () => {
  if (tabManager && tabManager.tabs.length > 0) store.save({ tabs: tabManager.serialize() });
  tabManager?.closeAll();
  windowManager?.showLogin();
};

const showLoggedOut = () => {
  store.save({ tabs: null });
  tabManager?.closeAll();
  windowManager?.showLogin();
};

const showLoggedIn = () => {
  windowManager?.hideLogin();
  if (!tabManager || tabManager.tabs.length > 0) return;
  const saved = store.data.tabs;
  const urls = saved?.urls.filter((url) => policy?.classify(url) === 'website') ?? [];
  if (urls.length > 0) tabManager.restore({ urls, active: saved?.active ?? 0 });
  else tabManager.create(`${env.websiteUrl}/`);
  store.save({ tabs: null });
};

const createWindow = () => {
  store.load();
  windowManager = new WindowManager(sanitizeWindowState(store.data.window), osTheme(), { version: app.getVersion(), env: env.name });
  policy = new NavigationPolicy(env, {
    onLoginRequired: showLoginKeepingTabs,
    onLogout: () => auth.logout().catch(() => null),
    onOpenTab: (url, background) => tabManager?.create(url, { background }),
  });
  tabManager = new TabManager(windowManager, policy);
  const chrome = windowManager.chrome.webContents;

  let lastActiveId: string | null = null;
  tabManager.onState((state) => {
    chromeWebContents()?.send(IPC.tabsState, state);
    for (const id of themes.keys()) {
      if (!webContents.fromId(id)) themes.delete(id);
    }
    if (state.activeId === lastActiveId) return;
    lastActiveId = state.activeId;
    const active = tabManager?.activeTab?.view.webContents;
    const theme = active ? themes.get(active.id) : undefined;
    if (theme) {
      chromeWebContents()?.send(IPC.theme, theme);
      windowManager?.setTheme(theme.theme);
    }
  });
  tabManager.onEmpty(() => windowManager?.window.close());

  if (process.platform !== 'darwin') windowManager.window.setAutoHideMenuBar(true);

  windowManager.window.on('focus', () => tabManager?.activeTab?.view.webContents.send(IPC.bridgeFocus));

  windowManager.window.on('app-command', (_event, command) => {
    if (command === 'browser-backward') tabManager?.goBack();
    else if (command === 'browser-forward') tabManager?.goForward();
  });

  windowManager.window.on('close', (event) => {
    if (windowManager && tabManager) {
      store.save({ window: windowManager.state(), tabs: tabManager.tabs.length > 0 ? tabManager.serialize() : store.data.tabs });
    }
    if (!quitting && process.platform === 'darwin') {
      event.preventDefault();
      windowManager?.window.hide();
    }
  });

  windowManager.window.on('closed', () => {
    tabManager?.closeAll();
    windowManager?.dispose();
    windowManager = undefined;
    tabManager = undefined;
    policy = undefined;
    themes.clear();
  });

  const loggedIn = auth.hasSession();
  chrome.once('did-finish-load', async () => {
    windowManager?.show();
    if (await loggedIn) showLoggedIn();
    else showLoginKeepingTabs();
  });
};

ipcMain.on(IPC.themeChanged, (event, theme: ThemePayload) => {
  themes.set(event.sender.id, theme);
  if (event.sender === tabManager?.activeTab?.view.webContents) {
    chromeWebContents()?.send(IPC.theme, theme);
    windowManager?.setTheme(theme.theme);
  }
});
ipcMain.handle(IPC.authLogin, () => auth.startLogin());
ipcMain.on(IPC.authCancel, () => auth.cancelLogin());
ipcMain.on(IPC.tabsNew, () => tabManager?.create(`${env.websiteUrl}/`));
ipcMain.on(IPC.tabsClose, (_event, id: string) => tabManager?.close(id));
ipcMain.on(IPC.tabsActivate, (_event, id: string) => tabManager?.activate(id));
ipcMain.on(IPC.tabsMove, (_event, id: string, toIndex: number) => tabManager?.move(id, toIndex));
ipcMain.on(IPC.menuPopup, () => {
  if (menu && windowManager) menu.popup({ window: windowManager.window });
});
ipcMain.on(IPC.pageRetry, (event) => tabManager?.retry(event.sender));
ipcMain.on(IPC.updateRestart, () => void updater.confirmRestart(windowManager?.window));
ipcMain.on(IPC.contextMenu, (event, request: ContextMenuRequest) => {
  if (windowManager && event.sender === tabManager?.activeTab?.view.webContents) showContextMenu(windowManager.window, request);
});
ipcMain.handle(IPC.bridgeOpenExternal, (_event, url: string) => {
  const kind = policy?.classify(url);
  if (kind === 'external' || kind === 'website' || kind === 'auth-other') return shell.openExternal(url);
});

const applyMenu = () => {
  menu = buildMenu(
    {
      newTab: () => tabManager?.create(`${env.websiteUrl}/`),
      closeTab: () => tabManager?.closeActive(),
      closeWindow: () => windowManager?.window.close(),
      reopenTab: () => tabManager?.reopenLast(),
      reload: () => tabManager?.reloadActive(),
      goBack: () => tabManager?.goBack(),
      goForward: () => tabManager?.goForward(),
      nextTab: () => tabManager?.next(),
      prevTab: () => tabManager?.prev(),
      activateTab: (index) => tabManager?.activateIndex(index),
      activateLastTab: () => tabManager?.activateLast(),
      checkForUpdates: () => void updater.checkManually(windowManager?.window),
      restartToUpdate: () => void updater.confirmRestart(windowManager?.window),
      openWebsite: () => shell.openExternal(env.websiteUrl),
      toggleDevTools: () => tabManager?.activeTab?.view.webContents.toggleDevTools(),
      crashActiveTab: () => tabManager?.activeTab?.view.webContents.forcefullyCrashRenderer(),
    },
    { devTools: !app.isPackaged, updateReady: updater.ready },
  );
  Menu.setApplicationMenu(menu);
};

updater.on('ready', () => {
  chromeWebContents()?.send(IPC.updateReady);
  applyMenu();
});

auth.on('authenticated', showLoggedIn);
auth.on('logged-out', showLoggedOut);
auth.on('error', (message) => {
  const login = windowManager?.loginWebContents;
  if (!login || login.isDestroyed()) return;
  login.send(IPC.authError, message);
});

// eslint-disable-next-line unicorn/prefer-top-level-await
app.whenReady().then(() => {
  if (!singleInstance) return;
  console.log(`[typie] env=${env.name} website=${env.websiteUrl}`);
  applyMenu();
  session.defaultSession.setPermissionRequestHandler((_wc, permission, callback) => {
    callback(['clipboard-read', 'clipboard-sanitized-write', 'notifications', 'fullscreen'].includes(permission));
  });
  createWindow();
  updater.start();
  app.on('activate', () => {
    if (!windowManager) {
      createWindow();
      return;
    }
    windowManager.show();
    if (!windowManager.loginVisible && tabManager?.tabs.length === 0) showLoggedIn();
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});
