import { getMarkAttributes, Mark } from '@tiptap/core';
import { Plugin } from '@tiptap/pm/state';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    textStyle: {
      isTextStyleAllowed: () => ReturnType;
      setTextStyle: (attributes: Record<string, unknown>) => ReturnType;
      removeEmptyTextStyle: () => ReturnType;
    };
  }
}

export const TextStyle = Mark.create({
  name: 'text_style',
  priority: 101,

  parseHTML() {
    return [{ tag: 'span', getAttrs: (node) => node.hasAttribute('style') && null }];
  },

  renderHTML({ HTMLAttributes }) {
    return ['span', HTMLAttributes, 0];
  },

  addCommands() {
    return {
      isTextStyleAllowed:
        () =>
        ({ can }) => {
          return can().isMarkAllowed(this.type);
        },

      setTextStyle:
        (attributes) =>
        ({ can, chain }) => {
          if (!can().isTextStyleAllowed()) {
            return false;
          }

          return chain().setMark(this.type, attributes).removeEmptyTextStyle().run();
        },

      removeEmptyTextStyle:
        () =>
        ({ state, commands }) => {
          const attributes = getMarkAttributes(state, this.type);
          const hasStyles = Object.entries(attributes).some(([, value]) => !!value);

          if (hasStyles) {
            return true;
          }

          return commands.unsetMark(this.type);
        },
    };
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        appendTransaction: (transactions, _, newState) => {
          if (!transactions.some((tr) => tr.docChanged)) {
            return null;
          }

          const { tr, doc } = newState;
          let modified = false;

          doc.descendants((node, pos) => {
            if (!node.isText) {
              return true;
            }

            const marks = node.marks.filter((mark) => mark.type === this.type);
            if (marks.length === 0) {
              return true;
            }

            for (const mark of marks) {
              const hasStyles = Object.entries(mark.attrs).some(([, value]) => !!value);
              if (!hasStyles) {
                tr.removeMark(pos, pos + node.nodeSize, mark.type);
                modified = true;
              }
            }

            return true;
          });

          return modified ? tr : null;
        },
      }),
    ];
  },
});
