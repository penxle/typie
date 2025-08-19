import { Extension } from '@tiptap/core';
import { Plugin } from '@tiptap/pm/state';
import { tick } from 'svelte';

declare global {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Window {
    notifyIdle?: () => void;
  }
}

export const NotifyIdle = Extension.create({
  name: 'notifyIdle',

  addProseMirrorPlugins() {
    return [
      new Plugin({
        view() {
          return {
            update() {
              tick().then(() => {
                requestIdleCallback(() => {
                  window.notifyIdle?.();
                });
              });
            },
          };
        },
      }),
    ];
  },
});
