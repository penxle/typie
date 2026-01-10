import { findParentNodeClosestToPos } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Dialog } from '../../../notification';
import { createNodeView } from '../../lib';
import Component from './Component.svelte';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    paywall: {
      togglePaywall: () => ReturnType;
      deletePaywall: (pos: number, nodeSize: number) => ReturnType;
    };
  }
}

export const Paywall = createNodeView(Component, {
  name: 'paywall',
  group: 'block',
  content: 'block+',
  isolating: true,
  defining: true,

  addAttributes() {
    return {
      price: {
        default: 0,
        parseHTML: (element) => {
          const price = element.dataset.price;
          return price ? Number.parseInt(price, 10) : 0;
        },
        renderHTML: ({ price }) => ({
          'data-price': price,
        }),
      },
    };
  },

  addCommands() {
    return {
      togglePaywall:
        () =>
        ({ can, editor, commands, state }) => {
          if (!can().isNodeAllowed(this.name)) {
            return false;
          }

          if (editor.isActive(this.type)) {
            return commands.lift(this.type);
          }

          const { $from } = state.selection;
          const parentPaywall = findParentNodeClosestToPos($from, (node) => node.type.name === this.name);
          if (parentPaywall) {
            return false;
          }

          return commands.wrapIn(this.type);
        },

      deletePaywall:
        (pos, nodeSize) =>
        ({ tr, dispatch }) => {
          if (dispatch) {
            tr.setMeta('allowPaywallDelete', true);
            tr.delete(pos, pos + nodeSize);
            dispatch(tr);
          }
          return true;
        },
    };
  },

  addProseMirrorPlugins() {
    const paywallType = this.type;
    const editor = this.editor;

    return [
      new Plugin({
        key: new PluginKey('paywallDelete'),
        filterTransaction: (tr, state) => {
          if (!editor.isEditable) {
            return true;
          }

          if (tr.getMeta('allowPaywallDelete')) {
            return true;
          }

          if (!tr.docChanged) {
            return true;
          }

          const oldPaywalls: { nodeId: string; pos: number; nodeSize: number }[] = [];
          state.doc.descendants((node, pos) => {
            if (node.type.name === paywallType.name && node.attrs.nodeId) {
              oldPaywalls.push({ nodeId: node.attrs.nodeId as string, pos, nodeSize: node.nodeSize });
            }
            return true;
          });

          const newPaywallIds = new Set<string>();
          tr.doc.descendants((node) => {
            if (node.type.name === paywallType.name) {
              newPaywallIds.add(node.attrs.nodeId as string);
            }
            return true;
          });

          const deletedPaywall = oldPaywalls.find((p) => !newPaywallIds.has(p.nodeId));
          if (deletedPaywall) {
            Dialog.confirm({
              title: '유료 블록 삭제',
              message: '유료 블록을 삭제하시겠어요?\n블록 안의 콘텐츠도 함께 삭제돼요.',
              action: 'danger',
              actionLabel: '삭제',
              actionHandler: () => {
                editor.commands.deletePaywall(deletedPaywall.pos, deletedPaywall.nodeSize);
              },
            });
            return false;
          }

          return true;
        },
      }),

      new Plugin({
        key: new PluginKey('paywallNesting'),
        appendTransaction: (transactions, _oldState, newState) => {
          const hasDocChanged = transactions.some((tr) => tr.docChanged);
          if (!hasDocChanged) {
            return null;
          }

          const nestedPaywalls: { from: number; to: number; content: typeof newState.doc.content }[] = [];

          newState.doc.descendants((node, pos) => {
            if (node.type.name === paywallType.name) {
              node.descendants((child, childPos) => {
                if (child.type.name === paywallType.name) {
                  const absolutePos = pos + 1 + childPos;
                  nestedPaywalls.push({
                    from: absolutePos,
                    to: absolutePos + child.nodeSize,
                    content: child.content,
                  });
                }
                return true;
              });
            }
            return true;
          });

          if (nestedPaywalls.length === 0) {
            return null;
          }

          const { tr } = newState;
          tr.setMeta('allowPaywallDelete', true);
          nestedPaywalls.sort((a, b) => b.from - a.from);
          for (const { from, to, content } of nestedPaywalls) {
            tr.replaceWith(from, to, content);
          }

          return tr;
        },
      }),
    ];
  },
});
