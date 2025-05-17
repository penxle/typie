import { combineTransactionSteps, Extension, findChildren, findChildrenInRange, findDuplicates, getChangedRanges } from '@tiptap/core';
import { Fragment, Node as ProseMirrorNode, Slice } from '@tiptap/pm/model';
import { Plugin, Transaction } from '@tiptap/pm/state';
import { nanoid } from 'nanoid';

const generateId = () => nanoid(32);
const types = [
  'body',
  'bullet_list',
  'hard_break',
  'list_item',
  'ordered_list',
  'paragraph',
  'blockquote',
  'callout',
  'code_block',
  'embed',
  'file',
  'fold',
  'horizontal_rule',
  'html-block',
  'image',
  'table',
  'table_row',
  'table_cell',
];

export const NodeId = Extension.create({
  name: 'node_id',
  priority: 10_000,

  addGlobalAttributes() {
    return [
      {
        types,
        attributes: {
          nodeId: {
            default: null,
            parseHTML: (element) => element.dataset.nodeId,
            renderHTML: ({ nodeId }) => {
              return { 'data-node-id': nodeId };
            },
          },
        },
      },
    ];
  },

  onCreate() {
    const { view, state } = this.editor;
    const { tr, doc } = state;

    const children = findChildren(doc, (node) => types.includes(node.type.name) && node.attrs.nodeId === null);

    for (const { node, pos } of children) {
      tr.setNodeMarkup(pos, undefined, {
        ...node.attrs,
        nodeId: generateId(),
      });
    }

    tr.setMeta('addToHistory', false);
    view.dispatch(tr);
  },

  addProseMirrorPlugins() {
    let dragSourceEl: HTMLElement | null = null;
    let pastedFromOutside = false;

    return [
      new Plugin({
        appendTransaction: (transactions, oldState, newState) => {
          const docChanged = transactions.some((tr) => tr.docChanged) && !oldState.doc.eq(newState.doc);
          const ySync = transactions.find((tr) => tr.getMeta('y-sync$'));

          if (!docChanged || ySync) {
            return;
          }

          const transform = combineTransactionSteps(oldState.doc, transactions as Transaction[]);
          const { mapping } = transform;
          const { tr } = newState;

          for (const { newRange } of getChangedRanges(transform)) {
            const nodes = findChildrenInRange(newState.doc, newRange, (node) => types.includes(node.type.name));
            const ids: string[] = nodes.map(({ node }) => node.attrs.nodeId).filter((id) => id !== null);

            for (const [index, { node, pos }] of nodes.entries()) {
              const id = tr.doc.nodeAt(pos)?.attrs.nodeId;
              if (id === null) {
                tr.setNodeMarkup(pos, undefined, {
                  ...node.attrs,
                  nodeId: generateId(),
                });

                continue;
              }

              const nextNode = nodes[index + 1];

              if (nextNode && node.content.size === 0) {
                tr.setNodeMarkup(nextNode.pos, undefined, {
                  ...nextNode.node.attrs,
                  nodeId: id,
                });

                ids[index + 1] = id;

                if (!nextNode.node.attrs.nodeId) {
                  const newId = generateId();

                  tr.setNodeMarkup(pos, undefined, {
                    ...node.attrs,
                    nodeId: newId,
                  });

                  ids[index] = newId;
                }

                continue;
              }

              const duplicateIds = findDuplicates(ids);
              const { deleted } = mapping.invert().mapResult(pos);

              if (deleted && duplicateIds.includes(id)) {
                tr.setNodeMarkup(pos, undefined, {
                  ...node.attrs,
                  nodeId: generateId(),
                });
              }
            }
          }

          if (tr.steps.length === 0) {
            return;
          }

          tr.setStoredMarks(newState.tr.storedMarks);

          return tr;
        },

        view: (view) => {
          const handleDragStart = (event: DragEvent) => {
            dragSourceEl = view.dom.parentElement?.contains(event.target as Node) ? view.dom.parentElement : null;
          };

          window.addEventListener('dragstart', handleDragStart);

          return {
            destroy() {
              window.removeEventListener('dragstart', handleDragStart);
            },
          };
        },

        props: {
          handleDOMEvents: {
            drop: (view, event) => {
              const fromSelf = dragSourceEl === view.dom.parentElement;
              const copying = event.dataTransfer?.effectAllowed === 'copyMove' || event.dataTransfer?.effectAllowed === 'copy';

              if (fromSelf && !copying) {
                dragSourceEl = null;
                pastedFromOutside = true;
              }

              return false;
            },

            paste: () => {
              pastedFromOutside = true;

              return false;
            },
          },

          transformPasted: (slice) => {
            if (!pastedFromOutside) {
              return slice;
            }

            const transformFragment = (fragment: Fragment): Fragment => {
              const nodes: ProseMirrorNode[] = [];

              fragment.forEach((node) => {
                if (node.isText) {
                  nodes.push(node);
                  return;
                }

                if (!types.includes(node.type.name)) {
                  nodes.push(node.copy(transformFragment(node.content)));
                  return;
                }

                const newNode = node.type.create({ ...node.attrs, nodeId: null }, transformFragment(node.content), node.marks);
                nodes.push(newNode);
              });

              return Fragment.from(nodes);
            };

            pastedFromOutside = false;

            return new Slice(transformFragment(slice.content), slice.openStart, slice.openEnd);
          },
        },
      }),
    ];
  },
});
