import { randomUUID } from 'node:crypto';
import { createTabView } from './tab-view';
import { rendererUrl } from './window-manager';
import type { TabIcon } from '@typie/lib/desktop';
import type { WebContents, WebContentsView } from 'electron';
import type { NavigationPolicy } from './navigation-policy';
import type { TabSession } from './store';
import type { WindowManager } from './window-manager';

export type TabState = { id: string; title: string; url: string; icon: TabIcon | null };
export type TabsStatePayload = { tabs: TabState[]; activeId: string | null };

type Tab = TabState & { view: WebContentsView };

const RECENTLY_CLOSED_LIMIT = 10;

export class TabManager {
  #tabs: Tab[] = [];
  #activeId: string | null = null;
  #recentlyClosed: string[] = [];
  #restoring = false;
  #related: { id: string; count: number } | null = null;
  #onState?: (state: TabsStatePayload) => void;
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
    tab.view.webContents.loadURL(rendererUrl(page, { ...query, theme: this.#windowManager.theme })).catch(() => null);
  }

  #setTitle(id: string, title: string) {
    if (title) this.#update(id, { title });
  }

  #update(id: string, patch: Partial<TabState>) {
    const tab = this.#tabs.find((t) => t.id === id);
    if (!tab) return;
    Object.assign(tab, patch);
    this.#publish();
  }

  #publish() {
    if (this.#restoring) return;
    this.#onState?.({ tabs: this.tabs, activeId: this.#activeId });
  }

  onState(callback: (state: TabsStatePayload) => void) {
    this.#onState = callback;
  }

  get tabs(): TabState[] {
    return this.#tabs.map((tab) => ({ id: tab.id, title: tab.title, url: tab.url, icon: tab.icon }));
  }

  get activeTab(): Tab | undefined {
    return this.#tabs.find((tab) => tab.id === this.#activeId);
  }

  create(url: string, options: { background?: boolean; index?: number } = {}) {
    const id = randomUUID();
    const tab: Tab = {
      id,
      title: '',
      url,
      icon: null,
      view: createTabView({
        onTitle: (title) => this.#setTitle(id, title),
        onNavigate: (nextUrl) => {
          const patch: Partial<TabState> = { title: '', icon: null };
          if (this.#policy.classify(nextUrl) === 'website') patch.url = nextUrl;
          this.#update(id, patch);
        },
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
    tab.view.setBackgroundColor(this.#windowManager.background);
    const index = options.index ?? this.#tabs.length;
    this.#tabs.splice(index, 0, tab);
    if (!options.background || !this.#activeId) this.activate(id);
    else this.#windowManager.presize(tab.view);
    tab.view.webContents.loadURL(url).catch(() => null);
    this.#publish();
    return id;
  }

  openFrom(opener: WebContents, url: string, background: boolean) {
    const index = this.#tabs.findIndex((t) => t.view.webContents === opener);
    if (index === -1) return this.create(url, { background });
    const tab = this.#tabs[index];
    const count = this.#related?.id === tab.id ? this.#related.count : 0;
    const id = this.create(url, { background, index: Math.min(index + 1 + count, this.#tabs.length) });
    this.#related = { id: tab.id, count: count + 1 };
    return id;
  }

  activate(id: string) {
    const tab = this.#tabs.find((t) => t.id === id);
    if (!tab) return;
    const previous = this.activeTab;
    if (previous && previous.id !== id) {
      this.#windowManager.detach(previous.view);
      this.#related = null;
    }
    this.#activeId = id;
    this.#windowManager.attach(tab.view);
    tab.view.webContents.focus();
    this.#publish();
  }

  activateIndex(index: number) {
    const tab = this.#tabs[index];
    if (tab) this.activate(tab.id);
  }

  next() {
    this.#step(1);
  }

  prev() {
    this.#step(-1);
  }

  close(id: string) {
    if (this.#tabs.length <= 1) return;
    const index = this.#tabs.findIndex((t) => t.id === id);
    if (index === -1) return;
    const [tab] = this.#tabs.splice(index, 1);
    this.#related = null;
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

  get canReopen() {
    return this.#recentlyClosed.length > 0;
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
    this.#related = null;
    this.#publish();
  }

  reloadActive() {
    this.activeTab?.view.webContents.reload();
  }

  setBackground(color: string) {
    for (const tab of this.#tabs) tab.view.setBackgroundColor(color);
  }

  setIcon(sender: WebContents, icon: TabIcon) {
    const tab = this.#tabs.find((t) => t.view.webContents === sender);
    if (tab) this.#update(tab.id, { icon });
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
    this.#restoring = true;
    for (const url of session.urls) this.create(url, { background: true });
    this.#restoring = false;
    this.activateIndex(Math.min(session.active, this.#tabs.length - 1));
  }
}
