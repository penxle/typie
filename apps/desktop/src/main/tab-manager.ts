import { randomUUID } from 'node:crypto';
import { nativeTheme } from 'electron';
import { createTabView } from './tab-view';
import { rendererUrl } from './window-manager';
import type { WebContents, WebContentsView } from 'electron';
import type { NavigationPolicy } from './navigation-policy';
import type { TabSession } from './store';
import type { WindowManager } from './window-manager';

export type TabState = { id: string; title: string; url: string; loading: boolean };
export type TabsStatePayload = { tabs: TabState[]; activeId: string | null };

type Tab = TabState & { view: WebContentsView };

const RECENTLY_CLOSED_LIMIT = 10;

export class TabManager {
  #tabs: Tab[] = [];
  #activeId: string | null = null;
  #recentlyClosed: string[] = [];
  #onState?: (state: TabsStatePayload) => void;
  #onEmpty?: () => void;
  #windowManager: WindowManager;
  #policy: NavigationPolicy;

  constructor(windowManager: WindowManager, policy: NavigationPolicy) {
    this.#windowManager = windowManager;
    this.#policy = policy;
  }

  #step(delta: number) {
    if (this.#tabs.length === 0) return;
    const current = this.#tabs.findIndex((t) => t.id === this.#activeId);
    this.activateIndex((current + delta + this.#tabs.length) % this.#tabs.length);
  }

  #showPage(id: string, page: 'offline' | 'crash', query: Record<string, string>) {
    const tab = this.#tabs.find((t) => t.id === id);
    if (!tab) return;
    tab.view.webContents
      .loadURL(rendererUrl(page, { ...query, theme: nativeTheme.shouldUseDarkColors ? 'dark' : 'light' }))
      .catch(() => null);
  }

  #update(id: string, patch: Partial<TabState>) {
    const tab = this.#tabs.find((t) => t.id === id);
    if (!tab) return;
    Object.assign(tab, patch);
    this.#publish();
  }

  #publish() {
    this.#onState?.({ tabs: this.tabs, activeId: this.#activeId });
  }

  onState(callback: (state: TabsStatePayload) => void) {
    this.#onState = callback;
  }

  onEmpty(callback: () => void) {
    this.#onEmpty = callback;
  }

  get tabs(): TabState[] {
    return this.#tabs.map((tab) => ({ id: tab.id, title: tab.title, url: tab.url, loading: tab.loading }));
  }

  get activeTab(): Tab | undefined {
    return this.#tabs.find((tab) => tab.id === this.#activeId);
  }

  create(url: string, options: { background?: boolean; index?: number } = {}) {
    const id = randomUUID();
    const tab: Tab = {
      id,
      title: '타이피',
      url,
      loading: true,
      view: createTabView({
        onTitle: (title) => this.#update(id, { title }),
        onLoading: (loading) => this.#update(id, { loading }),
        onUrl: (nextUrl) => {
          if (this.#policy.classify(nextUrl) === 'website') this.#update(id, { url: nextUrl });
        },
        onFailed: (url, code) => {
          if (this.#policy.classify(url) === 'website') this.#showPage(id, 'offline', { url, code: String(code) });
        },
        onCrashed: (url) => this.#showPage(id, 'crash', { url }),
      }),
    };
    this.#policy.attach(tab.view.webContents);
    const index = options.index ?? this.#tabs.length;
    this.#tabs.splice(index, 0, tab);
    tab.view.webContents.loadURL(url).catch(() => null);
    if (!options.background || !this.#activeId) this.activate(id);
    this.#publish();
    return id;
  }

  activate(id: string) {
    const tab = this.#tabs.find((t) => t.id === id);
    if (!tab) return;
    const previous = this.activeTab;
    if (previous && previous.id !== id) this.#windowManager.detach(previous.view);
    this.#activeId = id;
    this.#windowManager.attach(tab.view);
    tab.view.webContents.focus();
    this.#publish();
  }

  activateIndex(index: number) {
    const tab = this.#tabs[index];
    if (tab) this.activate(tab.id);
  }

  activateLast() {
    this.activateIndex(this.#tabs.length - 1);
  }

  next() {
    this.#step(1);
  }

  prev() {
    this.#step(-1);
  }

  close(id: string) {
    const index = this.#tabs.findIndex((t) => t.id === id);
    if (index === -1) return;
    const [tab] = this.#tabs.splice(index, 1);
    if (this.#activeId === id) {
      this.#windowManager.detach(tab.view);
      this.#activeId = null;
      const neighbor = this.#tabs[index] ?? this.#tabs[index - 1];
      if (neighbor) this.activate(neighbor.id);
    }
    this.#recentlyClosed.push(tab.url);
    if (this.#recentlyClosed.length > RECENTLY_CLOSED_LIMIT) this.#recentlyClosed.shift();
    if (!tab.view.webContents.isDestroyed()) tab.view.webContents.close();
    this.#publish();
    if (this.#tabs.length === 0) this.#onEmpty?.();
  }

  closeActive() {
    if (this.#activeId) this.close(this.#activeId);
  }

  closeAll() {
    for (const tab of this.#tabs) {
      if (tab.id === this.#activeId) this.#windowManager.detach(tab.view);
      if (!tab.view.webContents.isDestroyed()) tab.view.webContents.close();
    }
    this.#tabs = [];
    this.#activeId = null;
    this.#publish();
  }

  reopenLast() {
    const url = this.#recentlyClosed.pop();
    if (url) this.create(url);
  }

  move(id: string, toIndex: number) {
    const from = this.#tabs.findIndex((t) => t.id === id);
    if (from === -1) return;
    const [tab] = this.#tabs.splice(from, 1);
    this.#tabs.splice(Math.max(0, Math.min(toIndex, this.#tabs.length)), 0, tab);
    this.#publish();
  }

  reloadActive() {
    this.activeTab?.view.webContents.reload();
  }

  retry(sender: WebContents) {
    const tab = this.#tabs.find((t) => t.view.webContents === sender);
    if (!tab) return;
    const current = new URL(tab.view.webContents.getURL());
    const target = current.searchParams.get('url') || tab.url;
    tab.view.webContents.loadURL(target).catch(() => null);
  }

  goBack() {
    const wc = this.activeTab?.view.webContents;
    if (wc?.navigationHistory.canGoBack()) wc.navigationHistory.goBack();
  }

  goForward() {
    const wc = this.activeTab?.view.webContents;
    if (wc?.navigationHistory.canGoForward()) wc.navigationHistory.goForward();
  }

  serialize(): TabSession {
    const saved = this.#tabs.filter((t) => this.#policy.classify(t.url) === 'website');
    return {
      urls: saved.map((t) => t.url),
      active: Math.max(
        0,
        saved.findIndex((t) => t.id === this.#activeId),
      ),
    };
  }

  restore(session: TabSession) {
    for (const url of session.urls) this.create(url, { background: true });
    this.activateIndex(Math.min(session.active, this.#tabs.length - 1));
  }
}
