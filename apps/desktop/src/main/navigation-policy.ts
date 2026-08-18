import { shell } from 'electron';
import type { WebContents } from 'electron';
import type { Env } from './env';

export type NavigationKind = 'website' | 'auth-login' | 'auth-logout' | 'auth-other' | 'external' | 'blocked';

export type NavigationHandlers = {
  onLoginRequired: () => void;
  onLogout: () => void;
  onOpenTab: (url: string, background: boolean) => void;
};

const AUTH_LOGIN_PATHS = new Set(['/authorize', '/login', '/signup']);

export class NavigationPolicy {
  #websiteOrigin: string;
  #authOrigin: string;
  #handlers: NavigationHandlers;

  constructor(env: Env, handlers: NavigationHandlers) {
    this.#websiteOrigin = new URL(env.websiteUrl).origin;
    this.#authOrigin = new URL(env.authUrl).origin;
    this.#handlers = handlers;
  }

  #dispatch(kind: NavigationKind, url: string) {
    switch (kind) {
      case 'auth-login': {
        this.#handlers.onLoginRequired();
        break;
      }
      case 'auth-logout': {
        this.#handlers.onLogout();
        break;
      }
      case 'auth-other':
      case 'external': {
        shell.openExternal(url).catch(() => null);
        break;
      }
      case 'blocked':
      case 'website': {
        break;
      }
    }
  }

  classify(rawUrl: string): NavigationKind {
    let url: URL;
    try {
      url = new URL(rawUrl);
    } catch {
      return 'blocked';
    }
    if (url.protocol !== 'http:' && url.protocol !== 'https:') return 'blocked';
    if (url.origin === this.#websiteOrigin) return 'website';
    if (url.origin === this.#authOrigin) {
      if (AUTH_LOGIN_PATHS.has(url.pathname)) return 'auth-login';
      if (url.pathname === '/logout') return 'auth-logout';
      return 'auth-other';
    }
    return 'external';
  }

  attach(webContents: WebContents) {
    webContents.on('will-navigate', (event, url) => {
      const kind = this.classify(url);
      if (kind === 'website') return;
      event.preventDefault();
      this.#dispatch(kind, url);
    });
    webContents.on('will-redirect', (event, url) => {
      const kind = this.classify(url);
      if (kind === 'website') return;
      event.preventDefault();
      this.#dispatch(kind, url);
    });
    webContents.setWindowOpenHandler(({ url, disposition }) => {
      const kind = this.classify(url);
      if (kind === 'website') {
        this.#handlers.onOpenTab(url, disposition === 'background-tab');
      } else {
        this.#dispatch(kind, url);
      }
      return { action: 'deny' };
    });
  }
}
