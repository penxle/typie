import { createHash, randomBytes } from 'node:crypto';
import { EventEmitter } from 'node:events';
import http from 'node:http';
import { app, session, shell } from 'electron';
import type { Env } from './env';

const COOKIE_NAME = 'typie-at';
const LOGIN_TTL_MS = 10 * 60 * 1000;
const ONE_YEAR_SECONDS = 365 * 24 * 60 * 60;

type Pending = {
  verifier: string;
  nonce: string;
  redirectUri: string;
  server: http.Server;
  timer: NodeJS.Timeout;
};

const base64url = (buffer: Buffer) => buffer.toString('base64url');

const serializeOAuthState = (state: unknown) => Buffer.from(JSON.stringify(state), 'utf8').toString('base64url');

const deserializeOAuthState = (state: string): Record<string, unknown> => {
  try {
    return JSON.parse(Buffer.from(state, 'base64url').toString('utf8')) as Record<string, unknown>;
  } catch {
    return {};
  }
};

// eslint-disable-next-line unicorn/prefer-event-target
export class AuthService extends EventEmitter<{ authenticated: []; 'logged-out': []; error: [string] }> {
  #pending: Pending | null = null;
  #env: Env;

  constructor(env: Env) {
    super();
    this.#env = env;
  }

  #handleCallback(req: http.IncomingMessage, res: http.ServerResponse) {
    const pending = this.#pending;
    const url = new URL(req.url ?? '/', 'http://127.0.0.1');
    if (!pending || url.pathname !== '/callback') {
      res.writeHead(404).end();
      return;
    }
    const code = url.searchParams.get('code');
    const state = url.searchParams.get('state') ?? '';
    const { nonce } = deserializeOAuthState(state);
    if (!code || nonce !== pending.nonce) {
      console.warn('[auth] ignored callback: state mismatch or expired');
      res.writeHead(400, { 'Content-Type': 'text/plain; charset=utf-8' }).end('잘못된 로그인 요청이에요.');
      return;
    }
    res.writeHead(302, { Location: `${this.#env.authUrl}/desktop` }).end();
    this.#cancelPending();
    this.#exchange(code, pending.verifier, pending.redirectUri).catch((err: unknown) => {
      this.emit('error', err instanceof Error ? err.message : String(err));
    });
  }

  async #exchange(code: string, verifier: string, redirectUri: string) {
    const response = await fetch(`${this.#env.authUrl}/token`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams({
        grant_type: 'authorization_code',
        code,
        code_verifier: verifier,
        redirect_uri: redirectUri,
        client_id: this.#env.oidcClientId,
      }),
    });
    const data = (await response.json()) as { access_token?: string; error?: string; error_description?: string };
    if (!response.ok || !data.access_token) {
      throw new Error(data.error_description ?? data.error ?? `토큰 교환 실패 (${response.status})`);
    }
    await session.defaultSession.cookies.set({
      url: this.#env.websiteUrl,
      name: COOKIE_NAME,
      value: data.access_token,
      httpOnly: true,
      secure: this.#env.secureCookies,
      sameSite: 'lax',
      expirationDate: Math.floor(Date.now() / 1000) + ONE_YEAR_SECONDS,
    });
    app.focus({ steal: true });
    this.emit('authenticated');
  }

  #cancelPending() {
    if (!this.#pending) return;
    clearTimeout(this.#pending.timer);
    this.#pending.server.close();
    this.#pending = null;
  }

  cancelLogin() {
    this.#cancelPending();
  }

  async hasSession() {
    const cookies = await session.defaultSession.cookies.get({ url: this.#env.websiteUrl, name: COOKIE_NAME });
    return cookies.length > 0;
  }

  async startLogin() {
    this.#cancelPending();

    const verifier = base64url(randomBytes(32));
    const challenge = base64url(createHash('sha256').update(verifier).digest());
    const nonce = base64url(randomBytes(16));

    const server = http.createServer((req, res) => this.#handleCallback(req, res));
    try {
      await new Promise<void>((resolve, reject) => {
        server.once('error', reject);
        server.listen(0, '127.0.0.1', resolve);
      });
    } catch {
      this.emit('error', '로그인 리스너를 열 수 없어요.');
      return;
    }
    const address = server.address();
    if (!address || typeof address === 'string') {
      server.close();
      this.emit('error', '로그인 리스너를 열 수 없어요.');
      return;
    }
    const redirectUri = `http://127.0.0.1:${address.port}/callback`;
    const timer = setTimeout(() => this.#cancelPending(), LOGIN_TTL_MS);
    this.#pending = { verifier, nonce, redirectUri, server, timer };

    const url = new URL(`${this.#env.authUrl}/authorize`);
    url.searchParams.set('client_id', this.#env.oidcClientId);
    url.searchParams.set('response_type', 'code');
    url.searchParams.set('redirect_uri', redirectUri);
    url.searchParams.set('code_challenge', challenge);
    url.searchParams.set('code_challenge_method', 'S256');
    url.searchParams.set('prompt', 'consent');
    url.searchParams.set('state', serializeOAuthState({ nonce }));
    await shell.openExternal(url.href);
    return url.href;
  }

  async clearSession() {
    await session.defaultSession.cookies.remove(this.#env.websiteUrl, COOKIE_NAME);
  }

  async logout() {
    await this.clearSession();
    this.emit('logged-out');
  }
}
