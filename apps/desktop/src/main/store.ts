import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { app } from 'electron';
import type { WindowState } from './window-manager';

export type TabSession = { urls: string[]; active: number };
export type StoreData = { window: WindowState; tabs: TabSession | null };

const DEFAULT: StoreData = { window: {}, tabs: null };

const isTabSession = (value: unknown): value is TabSession => {
  if (typeof value !== 'object' || value === null) return false;
  const session = value as Partial<TabSession>;
  return Array.isArray(session.urls) && typeof session.active === 'number';
};

export class Store {
  #file = path.join(app.getPath('userData'), 'state.json');
  data: StoreData = DEFAULT;

  load() {
    try {
      const parsed = JSON.parse(readFileSync(this.#file, 'utf8')) as Partial<StoreData>;
      this.data = {
        window: typeof parsed.window === 'object' && parsed.window !== null ? parsed.window : {},
        tabs: isTabSession(parsed.tabs) ? parsed.tabs : null,
      };
    } catch {
      this.data = DEFAULT;
    }
    return this.data;
  }

  save(patch: Partial<StoreData>) {
    this.data = { ...this.data, ...patch };
    mkdirSync(path.dirname(this.#file), { recursive: true });
    writeFileSync(this.#file, JSON.stringify(this.data, null, 2));
  }
}
