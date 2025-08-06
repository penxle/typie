import { createNodeView } from '../../lib';
import Component from './Component.svelte';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    htmlBlock: {
      setHtmlBlock: () => ReturnType;
    };
  }
}

export const HtmlBlock = createNodeView(Component, {
  name: 'html_block',
  group: 'block',
  content: 'text*',
  marks: '',
  code: true,

  parseHTML() {
    return [{ tag: 'pre' }];
  },

  addCommands() {
    return {
      setHtmlBlock:
        () =>
        ({ can, commands }) => {
          if (!can().isNodeAllowed(this.name)) {
            return false;
          }

          return commands.insertNode(this.type.create());
        },
    };
  },
});
