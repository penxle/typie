import {
  combineTransactionSteps,
  findChildrenInRange,
  findParentNodeClosestToPos,
  getChangedRanges,
  isNodeActive,
  mergeAttributes,
  Node,
} from '@tiptap/core';
import { Plugin } from '@tiptap/pm/state';
import { ySyncPluginKey } from 'y-prosemirror';
import { closest } from '../../utils';
import { defaultValues, values } from '../values';
import type { Mark, NodeType, ResolvedPos } from '@tiptap/pm/model';

const textAligns = values.textAlign.map(({ value }) => value);
type TextAlign = (typeof textAligns)[number];

const lineHeights = values.lineHeight.map(({ value }) => value);
type LineHeight = (typeof lineHeights)[number];

const letterSpacings = values.letterSpacing.map(({ value }) => value);
type LetterSpacing = (typeof letterSpacings)[number];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    paragraph: {
      setParagraph: () => ReturnType;
      setParagraphTextAlign: (textAlign: TextAlign) => ReturnType;
      setParagraphLineHeight: (lineHeight: LineHeight) => ReturnType;
      setParagraphLetterSpacing: (letterSpacing: LetterSpacing) => ReturnType;
    };
  }
}

export const Paragraph = Node.create({
  name: 'paragraph',
  group: 'block',
  content: 'inline*',
  priority: 255,

  addAttributes() {
    return {
      textAlign: {
        default: defaultValues.textAlign,
        parseHTML: (element) => {
          const textAlign = element.style.textAlign;
          if (!(textAligns as string[]).includes(textAlign)) {
            return defaultValues.textAlign;
          }

          return textAlign;
        },
        renderHTML: ({ textAlign }) => ({
          style: `text-align: ${textAlign}`,
        }),
      },

      lineHeight: {
        default: defaultValues.lineHeight,
        parseHTML: (element) => {
          const lineHeight = Number.parseFloat(element.style.lineHeight);
          return closest(lineHeight, lineHeights) ?? defaultValues.lineHeight;
        },
        renderHTML: ({ lineHeight }) => ({
          style: `line-height: ${lineHeight}`,
        }),
      },

      letterSpacing: {
        default: defaultValues.letterSpacing,
        parseHTML: (element) => {
          const letterSpacing = Number.parseFloat(element.style.letterSpacing.replace(/em$/, ''));
          return closest(letterSpacing, letterSpacings) ?? defaultValues.letterSpacing;
        },
        renderHTML: ({ letterSpacing }) => ({
          style: `letter-spacing: ${letterSpacing}em`,
        }),
      },
    };
  },

  parseHTML() {
    return [{ tag: 'p' }];
  },

  renderHTML({ node, HTMLAttributes }) {
    return [
      'p',
      mergeAttributes(HTMLAttributes, {
        class: node.attrs.textAlign === 'left' || node.attrs.textAlign === 'justify' ? 'paragraph-indent' : undefined,
      }),
      !this.editor?.isEditable && node.content.size === 0 ? ['br', { class: 'ProseMirror-trailingBreak' }] : 0,
    ];
  },

  addCommands() {
    return {
      setParagraph:
        () =>
        ({ commands }) => {
          return commands.setNode(this.name);
        },

      setParagraphTextAlign:
        (textAlign) =>
        ({ state, tr, dispatch }) => {
          if (!textAligns.includes(textAlign)) {
            return false;
          }

          if (isNodeActive(state, 'blockquote') || isNodeActive(state, 'callout') || isNodeActive(state, 'list_item')) {
            return false;
          }

          const children = findChildrenInRange(state.doc, state.selection, (node) => node.type === this.type);
          if (children.length === 0) {
            return false;
          }

          for (const { pos, node } of children) {
            const pos$ = tr.doc.resolve(pos);
            const parent = findParentNodeClosestToPos(
              pos$,
              (node) => node.type.name === 'blockquote' || node.type.name === 'callout' || node.type.name === 'list_item',
            );

            if (parent) {
              continue;
            }

            tr.setNodeMarkup(pos, undefined, { ...node.attrs, textAlign });
          }

          dispatch?.(tr);

          return true;
        },

      setParagraphLineHeight:
        (lineHeight) =>
        ({ state, commands }) => {
          if (!lineHeights.includes(lineHeight)) {
            return false;
          }

          const children = findChildrenInRange(state.doc, state.selection, (node) => node.type === this.type);
          if (children.length === 0) {
            return false;
          }

          return commands.updateAttributes(this.type, { lineHeight });
        },

      setParagraphLetterSpacing:
        (letterSpacing) =>
        ({ state, commands }) => {
          if (!letterSpacings.includes(letterSpacing)) {
            return false;
          }

          const children = findChildrenInRange(state.doc, state.selection, (node) => node.type === this.type);
          if (children.length === 0) {
            return false;
          }

          return commands.updateAttributes(this.type, { letterSpacing });
        },
    };
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        appendTransaction: (transactions, oldState, newState) => {
          const docChanged = transactions.some((tr) => tr.docChanged) && !oldState.doc.eq(newState.doc);
          const ySync = transactions.find((tr) => tr.getMeta(ySyncPluginKey));
          const paste = transactions.find((tr) => tr.getMeta('paste'));
          const template = transactions.some((tr) => tr.getMeta('template'));

          if (!docChanged || ySync || paste || template) {
            return;
          }

          const { tr } = newState;

          const transform = combineTransactionSteps(oldState.doc, [...transactions]);
          for (const { oldRange, newRange } of getChangedRanges(transform)) {
            const oldNodeIds = new Set<string>();
            let oldAttrs: Record<string, unknown> | null = null;
            let oldMarks: Mark[] | null = null;

            oldState.doc.nodesBetween(oldRange.from, oldRange.to, (node, pos) => {
              if (node.type === this.type) {
                if (node.attrs.nodeId) {
                  oldNodeIds.add(node.attrs.nodeId);
                }

                if (!oldAttrs) {
                  oldAttrs = { ...node.attrs };
                  delete oldAttrs.nodeId;
                }

                if (!oldMarks) {
                  const pos$ = oldState.doc.resolve(pos + 1);
                  const m = pos$.marks();
                  if (m.length > 0) {
                    oldMarks = [...m];
                  }
                }
              }
            });

            newState.doc.nodesBetween(newRange.from, newRange.to, (node, pos) => {
              if (node.type === this.type) {
                const isNewNode = !node.attrs.nodeId || (oldNodeIds.size > 0 && !oldNodeIds.has(node.attrs.nodeId));
                if (isNewNode) {
                  if (oldAttrs) {
                    tr.setNodeMarkup(pos, undefined, { ...node.attrs, ...oldAttrs });
                  }

                  if (oldMarks) {
                    for (const mark of oldMarks) {
                      tr.addMark(pos + 1, pos + node.content.size + 1, mark);
                      tr.addStoredMark(mark);
                    }
                  }
                }
              }
            });
          }

          if (tr.docChanged) {
            return tr;
          }
        },
      }),

      new Plugin({
        appendTransaction: (_, __, newState) => {
          const { selection, storedMarks, tr } = newState;
          const { $anchor, empty } = selection;

          if (
            !empty ||
            $anchor.parent.type !== this.type ||
            $anchor.parentOffset !== 0 ||
            $anchor.parent.childCount !== 0 ||
            storedMarks !== null
          ) {
            return;
          }

          const textNode = getTextNodeToCopyMarks(this.type, $anchor);
          if (textNode) {
            const marks = textNode.marks.filter((mark) => mark.type.spec.inclusive !== false);
            tr.ensureMarks(marks);
            return tr;
          }
        },
      }),
    ];
  },
});

const getTextNodeToCopyMarks = (type: NodeType, $pos: ResolvedPos) => {
  const currentNode = $pos.parent;

  for (let depth = $pos.depth - 1; depth > 0; depth--) {
    const node = $pos.node(depth);
    if (node.childCount === 0) {
      continue;
    }

    for (let idx = $pos.index(depth) - 1; idx >= 0; idx--) {
      let child = node.child(idx);
      if (child.type.name === 'list_item') {
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        child = child.firstChild!;
      }

      if (child.type === type && child.childCount > 0 && child.attrs.textAlign === currentNode.attrs.textAlign) {
        for (let i = child.childCount - 1; i >= 0; i--) {
          const n = child.child(i);
          if (n.isText) {
            return n;
          }
        }
      }
    }
  }

  return null;
};
