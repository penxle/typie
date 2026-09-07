import { getContext, setContext } from 'svelte';
import { SvelteSet } from 'svelte/reactivity';

export type ChromePost = {
  title: string;
  url: string;
  collection: { id: string; name: string } | null;
};

const KEY = Symbol('usersite-chrome');

export class UsersiteChrome {
  #stuck = new SvelteSet<HTMLElement>();
  #lastY = 0;

  post = $state<ChromePost | null>(null);
  identityEls = $state<HTMLElement[]>([]);
  titleEl = $state<HTMLElement | null>(null);
  headerHeight = $state(52);
  scrollY = $state(0);
  retreat = $state(false);
  hold = $state(false);
  tick = $state(0);

  get merged(): boolean {
    return this.#stuck.size > 0;
  }

  setStuck(node: HTMLElement, value: boolean) {
    if (value) this.#stuck.add(node);
    else this.#stuck.delete(node);
  }

  sync() {
    const y = window.scrollY;
    if (y < 8 || this.hold) this.retreat = false;
    else if (y - this.#lastY > 4 && y > this.headerHeight + 60) this.retreat = true;
    else if (this.#lastY - y > 4) this.retreat = false;
    this.#lastY = y;
    this.scrollY = y;
    this.tick += 1;
  }

  reset() {
    this.#lastY = window.scrollY;
    this.scrollY = window.scrollY;
    this.retreat = false;
    this.tick += 1;
  }
}

export const setUsersiteChrome = (chrome: UsersiteChrome) => setContext(KEY, chrome);
export const getUsersiteChrome = () => getContext<UsersiteChrome>(KEY);
