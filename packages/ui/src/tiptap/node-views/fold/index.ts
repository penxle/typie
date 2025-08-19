import { createNodeView } from '../../lib';
import Component from './Component.svelte';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    fold: {
      toggleFold: () => ReturnType;
    };
  }
}

export const Fold = createNodeView(Component, {
  name: 'fold',
  group: 'block',
  content: 'block+',
  defining: true,

  addAttributes() {
    return {
      open: {
        default: true,
        parseHTML: (element) => element.getAttribute('open') === 'true',
        renderHTML: ({ open }) => ({
          open: open ? 'true' : undefined,
        }),
      },
      title: {
        default: '',
        parseHTML: (element) => element.dataset.title,
        renderHTML: ({ title }) => ({
          'data-title': title,
        }),
      },
    };
  },

  addCommands() {
    return {
      toggleFold:
        () =>
        ({ can, editor, commands }) => {
          if (!can().isNodeAllowed(this.name)) {
            return false;
          }

          if (editor.isActive(this.type)) {
            return commands.lift(this.type);
          } else {
            return commands.wrapIn(this.type);
          }
        },
    };
  },
});
