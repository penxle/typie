import { combineTransactionSteps, findChildrenInRange, getChangedRanges, getMarksBetween, Mark, mergeAttributes } from '@tiptap/core';
import { Plugin } from '@tiptap/pm/state';
import { find } from 'linkifyjs';
import { css } from '$styled-system/css';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    link: {
      setLink: (href: string) => ReturnType;
      updateLink: (href: string) => ReturnType;
      unsetLink: () => ReturnType;
    };
  }
}

export const Link = Mark.create({
  name: 'link',
  priority: 110,
  inclusive: false,

  addAttributes() {
    return {
      href: {},
    };
  },

  parseHTML() {
    return [{ tag: 'a[href]' }];
  },

  renderHTML({ HTMLAttributes }) {
    return [
      'a',
      mergeAttributes(HTMLAttributes, {
        class: css({ color: 'text.faint', textDecorationLine: 'underline' }),
        target: '_blank',
        rel: 'noreferrer nofollow',
      }),
      0,
    ];
  },

  addCommands() {
    return {
      setLink:
        (href) =>
        ({ commands, can }) => {
          if (!can().isMarkAllowed(this.name)) {
            return false;
          }

          return commands.setMark(this.name, { href });
        },

      updateLink:
        (href) =>
        ({ chain }) => {
          return chain().extendMarkRange(this.name).updateAttributes(this.name, { href }).run();
        },

      unsetLink:
        () =>
        ({ chain }) => {
          return chain().unsetMark(this.name, { extendEmptyMarkRange: true }).run();
        },
    };
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        appendTransaction: (transactions, oldState, newState) => {
          const docChanged = transactions.some((transaction) => transaction.docChanged) && !oldState.doc.eq(newState.doc);

          if (!docChanged) {
            return;
          }

          const { tr } = newState;
          const transform = combineTransactionSteps(oldState.doc, [...transactions]);
          const changes = getChangedRanges(transform);

          for (const { newRange } of changes) {
            const nodes = findChildrenInRange(tr.doc, newRange, (node) => node.isTextblock);
            for (const node of nodes) {
              const text = tr.doc.textBetween(node.pos, node.pos + node.node.nodeSize);
              const links = find(text, { defaultProtocol: 'https' }).filter((link) => link.isLink);

              for (const link of links) {
                const from = node.pos + link.start + 1;
                const to = node.pos + link.end + 1;

                const marks = getMarksBetween(from, to, tr.doc);
                if (
                  marks.some(
                    (mark) => mark.from === from && mark.to === to && mark.mark.type === this.type && mark.mark.attrs.href === link.href,
                  )
                ) {
                  continue;
                }

                tr.addMark(from, to, this.type.create({ href: link.href }));
              }
            }
          }

          if (tr.steps.length === 0) {
            return;
          }

          return tr;
        },
      }),
    ];
  },
});
