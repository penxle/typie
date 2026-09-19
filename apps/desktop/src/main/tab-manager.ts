import { randomUUID } from 'node:crypto';
import { prepareDocumentDeparture, requestDocumentSave } from './document-save';
import { createTabView } from './tab-view';
import { rendererUrl } from './window-manager';
import { nextZoomLevel } from './zoom';
import type { DesktopZoomAction, DocumentDepartureReason, TabIcon } from '@typie/lib/desktop';
import type { WebContents, WebContentsView } from 'electron';
import type { NavigationPolicy } from './navigation-policy';
import type { TabSession } from './store';
import type { WindowManager } from './window-manager';

export type TabState = { id: string; title: string; url: string; icon: TabIcon | null; saving: boolean };
export type TabsStatePayload = { tabs: TabState[]; activeId: string | null };

type Tab = TabState & { view: WebContentsView; documentLoaded: boolean };

const RECENTLY_CLOSED_LIMIT = 10;

const closeWebContents = async (contents: WebContents): Promise<void> => {
  if (contents.isDestroyed()) return;
  await new Promise<void>((resolve) => {
    contents.once('destroyed', resolve);
    contents.close();
  });
};

export class TabManager {
  #tabs: Tab[] = [];
  #activeId: string | null = null;
  #recentlyClosed: string[] = [];
  #restoring = false;
  #related: { id: string; count: number } | null = null;
  #onState?: (state: TabsStatePayload) => void;
  #windowManager: WindowManager;
  #policy: NavigationPolicy;
  #zoomLevel: number;
  #departure: Promise<unknown> = Promise.resolve();
  #preparingAll = false;
  #closing?: { ids: Set<string>; changes: EventTarget; promise: Promise<boolean> };

  constructor(windowManager: WindowManager, policy: NavigationPolicy, zoomLevel: number) {
    this.#windowManager = windowManager;
    this.#policy = policy;
    this.#zoomLevel = zoomLevel;
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

  async #close(id: string) {
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
    await closeWebContents(tab.view.webContents);
    this.#publish();
  }

  async #navigate(tab: Tab, action: () => void): Promise<void> {
    const wc = tab.view.webContents;
    if (wc.isDestroyed()) return;
    await new Promise<void>((resolve, reject) => {
      const cleanup = () => {
        wc.removeListener('did-stop-loading', done);
        wc.removeListener('did-navigate-in-page', done);
        wc.removeListener('destroyed', done);
      };
      const done = () => {
        cleanup();
        resolve();
      };
      // A redirect intercepted for authentication stops the load without a
      // did-finish-load event. Release this operation so login can prepare next.
      wc.once('did-stop-loading', done);
      wc.once('did-navigate-in-page', done);
      wc.once('destroyed', done);
      try {
        action();
      } catch (err) {
        cleanup();
        reject(err);
      }
    });
  }

  #prepareDeparture(
    reason: DocumentDepartureReason,
    commit: () => void | Promise<void>,
    includes?: (tab: Tab) => boolean,
    targetChanges?: EventTarget,
  ): Promise<boolean> {
    const next = this.#departure.then(async () => {
      this.#preparingAll = !includes;
      try {
        return await prepareDocumentDeparture({
          targets: () =>
            this.#tabs.flatMap((tab, index) =>
              (!includes || includes(tab)) && tab.documentLoaded && this.#policy.classify(tab.url) === 'website'
                ? [{ id: tab.id, title: `탭 ${index + 1}`, webContents: tab.view.webContents }]
                : [],
            ),
          reason,
          window: this.#windowManager.window,
          commit,
          getActiveContents: () => {
            const contents = this.activeTab?.view.webContents;
            return contents && this.#policy.classify(contents.getURL()) === 'website' ? contents : undefined;
          },
          onProgress: (targets, visible) => {
            for (const target of targets) this.#update(target.id, { saving: visible });
          },
          targetChanges,
          theme: this.#windowManager.themePayload,
        });
      } finally {
        this.#preparingAll = false;
      }
    });
    this.#departure = next.catch(() => false);
    return next;
  }

  onState(callback: (state: TabsStatePayload) => void) {
    this.#onState = callback;
  }

  get tabs(): TabState[] {
    return this.#tabs.map((tab) => ({ id: tab.id, title: tab.title, url: tab.url, icon: tab.icon, saving: tab.saving }));
  }

  get activeTab(): Tab | undefined {
    return this.#tabs.find((tab) => tab.id === this.#activeId);
  }

  hasWebContents(contents: WebContents): boolean {
    return this.#tabs.some((tab) => tab.view.webContents === contents);
  }

  navigate(contents: WebContents, url: string): Promise<boolean> | undefined {
    const tab = this.#tabs.find((tab) => tab.view.webContents === contents);
    if (!tab) return;
    return this.prepareDeparture(
      'close',
      async () => {
        // Intercepted authentication redirects abort this load; their queued
        // login operation owns the next transition.
        await contents.loadURL(url).catch(() => null);
      },
      tab.id,
    );
  }

  create(url: string, options: { background?: boolean; index?: number } = {}) {
    if (this.#preparingAll) return;
    const id = randomUUID();
    const tab: Tab = {
      id,
      title: '',
      url,
      icon: null,
      saving: false,
      documentLoaded: false,
      view: createTabView({
        onTitle: (title) => this.#setTitle(id, title),
        onNavigate: (nextUrl) => {
          tab.documentLoaded = true;
          tab.view.webContents.setZoomLevel(this.#zoomLevel);
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
    // Only a new tab without a committed page can skip preparation. Scripts
    // may run before DOM ready, so loading alone is never proof of no edits.
    // Never reset documentLoaded on reload/navigation or renderer failure.
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

  prepareDeparture(reason: DocumentDepartureReason, commit: () => void | Promise<void>, tabId?: string): Promise<boolean> {
    return this.#prepareDeparture(reason, commit, tabId ? (tab) => tab.id === tabId : undefined);
  }

  capture() {
    for (const tab of this.#tabs)
      void requestDocumentSave(
        { id: tab.id, title: tab.title, webContents: tab.view.webContents },
        { operationId: randomUUID(), phase: 'capture' },
      );
  }

  close(id: string) {
    if (this.#closing) {
      if (!this.#closing.ids.has(id)) {
        this.#closing.ids.add(id);
        this.#closing.changes.dispatchEvent(new Event('change'));
      }
      return this.#closing.promise;
    }
    const ids = new Set([id]);
    const changes = new EventTarget();
    const promise = this.#prepareDeparture(
      'close',
      async () => {
        // Once commit starts, a later close must prepare in its own operation.
        this.#closing = undefined;
        await Promise.all([...ids].map((id) => this.#close(id)));
      },
      (tab) => ids.has(tab.id),
      changes,
    ).finally(() => {
      if (this.#closing?.ids === ids) this.#closing = undefined;
    });
    this.#closing = { ids, changes, promise };
    return promise;
  }

  async closeAll() {
    const tabs = this.#tabs;
    this.#tabs = [];
    for (const tab of tabs) {
      if (tab.id === this.#activeId) this.#windowManager.detach(tab.view);
    }
    await Promise.all(tabs.map((tab) => closeWebContents(tab.view.webContents)));
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
    const tab = this.activeTab;
    if (tab) void this.prepareDeparture('reload', () => this.#navigate(tab, () => tab.view.webContents.reload()), tab.id);
  }

  setBackground(color: string) {
    for (const tab of this.#tabs) tab.view.setBackgroundColor(color);
  }

  zoom(action: DesktopZoomAction) {
    const next = nextZoomLevel(this.#zoomLevel, action);
    if (next === this.#zoomLevel) return null;
    this.#zoomLevel = next;
    for (const tab of this.#tabs) tab.view.webContents.setZoomLevel(next);
    return next;
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
    const tab = this.activeTab;
    if (tab?.view.webContents.navigationHistory.canGoBack())
      void this.prepareDeparture('close', () => this.#navigate(tab, () => tab.view.webContents.navigationHistory.goBack()), tab.id);
  }

  goForward() {
    const tab = this.activeTab;
    if (tab?.view.webContents.navigationHistory.canGoForward())
      void this.prepareDeparture('close', () => this.#navigate(tab, () => tab.view.webContents.navigationHistory.goForward()), tab.id);
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
