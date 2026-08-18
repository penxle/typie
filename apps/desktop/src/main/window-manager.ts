// cspell:ignore unmaximize

import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { BaseWindow, nativeTheme, WebContentsView } from 'electron';
import type { Rectangle } from 'electron';

export const CHROME_HEIGHT = 40;

const isMac = process.platform === 'darwin';

type Theme = 'light' | 'dark';

const themeColors = (theme: Theme) =>
  theme === 'dark' ? { background: '#1a1a1a', symbol: '#c8c8c8' } : { background: '#ffffff', symbol: '#888888' };

const rendererUrl = (page: string, query: Record<string, string> = {}) => {
  const search = new URLSearchParams(query).toString();
  if (process.env.ELECTRON_RENDERER_URL) {
    const suffix = search ? `?${search}` : '';
    return `${process.env.ELECTRON_RENDERER_URL}/${page}/index.html${suffix}`;
  }
  const url = pathToFileURL(path.join(import.meta.dirname, '../renderer', page, 'index.html'));
  url.search = search;
  return url.toString();
};

export const preloadPath = (name: 'chrome' | 'page' | 'tab') => path.join(import.meta.dirname, '../preload', `${name}.cjs`);

export type WindowState = { bounds?: Rectangle; maximized?: boolean };

export class WindowManager {
  #chromeVisible = true;
  #attached = new Set<WebContentsView>();
  #onLayout?: (bounds: Rectangle) => void;
  #login: WebContentsView | null = null;
  #loginAttached = false;

  readonly window: BaseWindow;
  readonly chrome: WebContentsView;

  constructor(state: WindowState, theme: Theme) {
    const { background, symbol } = themeColors(theme);
    this.window = new BaseWindow({
      width: state.bounds?.width ?? 1280,
      height: state.bounds?.height ?? 800,
      x: state.bounds?.x,
      y: state.bounds?.y,
      minWidth: 900,
      minHeight: 600,
      show: false,
      title: '타이피',
      backgroundColor: background,
      titleBarStyle: isMac ? 'hiddenInset' : 'hidden',
      trafficLightPosition: isMac ? { x: 16, y: 12 } : undefined,
      titleBarOverlay: isMac ? undefined : { color: background, symbolColor: symbol, height: CHROME_HEIGHT },
    });

    this.chrome = new WebContentsView({
      webPreferences: { preload: preloadPath('chrome'), sandbox: true, contextIsolation: true },
    });
    this.window.contentView.addChildView(this.chrome);
    this.chrome.webContents.loadURL(rendererUrl('chrome')).catch(() => null);

    this.window.on('resize', () => this.layout());
    this.window.on('maximize', () => this.layout());
    this.window.on('unmaximize', () => this.layout());
    if (state.maximized) this.window.maximize();
    this.layout();
  }

  onLayout(callback: (bounds: Rectangle) => void) {
    this.#onLayout = callback;
  }

  contentBounds(): Rectangle {
    const { width, height } = this.window.getContentBounds();
    const top = this.#chromeVisible ? CHROME_HEIGHT : 0;
    return { x: 0, y: top, width, height: height - top };
  }

  layout() {
    const { width } = this.window.getContentBounds();
    this.chrome.setBounds({ x: 0, y: 0, width, height: this.#chromeVisible ? CHROME_HEIGHT : 0 });
    const content = this.contentBounds();
    for (const view of this.#attached) view.setBounds(content);
    if (this.#login && this.#loginAttached) this.#login.setBounds(content);
    this.#onLayout?.(content);
  }

  setChromeVisible(visible: boolean) {
    this.#chromeVisible = visible;
    this.layout();
  }

  setTheme(theme: Theme) {
    const { background, symbol } = themeColors(theme);
    this.window.setBackgroundColor(background);
    if (process.platform === 'win32') {
      this.window.setTitleBarOverlay({ color: background, symbolColor: symbol, height: CHROME_HEIGHT });
    }
  }

  attach(view: WebContentsView) {
    this.#attached.add(view);
    this.window.contentView.addChildView(view);
    view.setBounds(this.contentBounds());
  }

  detach(view: WebContentsView) {
    this.#attached.delete(view);
    if (this.window.isDestroyed()) return;
    this.window.contentView.removeChildView(view);
  }

  showLogin() {
    if (this.#login) {
      this.#login.webContents.reload();
    } else {
      this.#login = new WebContentsView({
        webPreferences: { preload: preloadPath('page'), sandbox: true, contextIsolation: true },
      });
      this.#login.webContents
        .loadURL(rendererUrl('login', { theme: nativeTheme.shouldUseDarkColors ? 'dark' : 'light' }))
        .catch(() => null);
    }
    this.setChromeVisible(false);
    this.window.contentView.addChildView(this.#login);
    this.#loginAttached = true;
    this.#login.setBounds(this.contentBounds());
  }

  hideLogin() {
    if (!this.#login) return;
    this.window.contentView.removeChildView(this.#login);
    this.#loginAttached = false;
    this.setChromeVisible(true);
  }

  get loginWebContents() {
    return this.#login?.webContents ?? null;
  }

  get loginVisible() {
    return this.#loginAttached;
  }

  dispose() {
    if (!this.#login) return;
    if (!this.#login.webContents.isDestroyed()) this.#login.webContents.close();
    this.#login = null;
    this.#loginAttached = false;
  }

  show() {
    this.window.show();
  }

  state(): WindowState {
    return { bounds: this.window.getNormalBounds(), maximized: this.window.isMaximized() };
  }
}

export { rendererUrl };
