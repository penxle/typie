/* eslint-disable @typescript-eslint/consistent-type-definitions */

import 'unplugin-icons/types/svelte';

declare module 'svelte/elements' {
  export interface HTMLTextareaAttributes {
    autocorrect?: 'on' | 'off';
  }
}

declare global {
  namespace App {
    // interface Locals {}
    // interface PageData {}
    // interface Platform {}

    interface Error {
      message: string;
      code?: string;
      eventId?: string;
    }

    interface PageState {
      shallowRoute?: string;
    }
  }
}
