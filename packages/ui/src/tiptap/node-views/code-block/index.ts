import { createNodeView } from '../../lib';
import Component from './Component.svelte';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    codeBlock: {
      setCodeBlock: () => ReturnType;
    };
  }
}

export const CodeBlock = createNodeView(Component, {
  name: 'code_block',
  group: 'block',
  content: 'text*',
  marks: '',
  code: true,

  parseHTML() {
    return [{ tag: 'pre' }];
  },

  addAttributes() {
    return {
      language: {
        default: 'text',
      },
    };
  },

  addCommands() {
    return {
      setCodeBlock:
        () =>
        ({ can, chain }) => {
          if (!can().isNodeAllowed(this.name)) {
            return false;
          }

          return chain()
            .first(({ commands }) => [() => commands.insertNodeWithSelection(this.name), () => commands.insertNode(this.type.create())])
            .run();
        },
    };
  },
});
