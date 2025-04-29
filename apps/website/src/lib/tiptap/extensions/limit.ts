import { Extension, getText } from '@tiptap/core';
import { Fragment, Node, Schema, Slice } from '@tiptap/pm/model';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { ySyncPluginKey } from 'y-prosemirror';
import { textSerializers } from '@/pm/serializer';
import type { EditorView } from '@tiptap/pm/view';

const countChars = (item: Node | Fragment | Slice): number => {
  if (item instanceof Slice) {
    return countChars(item.content);
  }

  if (item instanceof Fragment) {
    let sum = 0;
    item.forEach((node) => {
      sum += countChars(node);
    });
    return sum;
  }

  const text = item.isText
    ? (item.text ?? '')
    : getText(item, {
        blockSeparator: '\n',
        textSerializers,
      });

  return [...text.replaceAll(/\s+/g, ' ').trim()].length;
};

const truncateFragment = (schema: Schema, fragment: Fragment, max: number) => {
  const nodes: Node[] = [];
  let used = 0;

  for (let i = 0; i < fragment.childCount; i++) {
    if (used >= max) break;

    const node = fragment.child(i);

    if (node.isText) {
      const chars = [...(node.text ?? '')];
      let partial = '';

      for (const char of chars) {
        if (countChars(schema.text(partial + char, node.marks)) + used > max) {
          break;
        }

        partial += char;
      }

      if (partial.length > 0) {
        const text = schema.text(partial, node.marks);
        nodes.push(text);
        used += countChars(text);
      }
    } else if (node.isLeaf) {
      nodes.push(node);
    } else if (node.content.size > 0) {
      const remaining = max - used;
      if (remaining <= 0) {
        break;
      }

      const content = truncateFragment(schema, node.content, remaining);
      if (content.size > 0) {
        nodes.push(node.copy(content));
        used += countChars(content);
      }
    }
  }

  return Fragment.fromArray(nodes);
};

const handleSlice = (view: EditorView, slice: Slice, maxCount: number) => {
  const { state } = view;
  const { schema, doc, selection } = state;

  const currentCount = countChars(doc);
  const selectionCount = selection.empty ? 0 : countChars(selection.content());

  let leftCount = maxCount - (currentCount - selectionCount);

  if (leftCount <= 0) {
    return true;
  }

  const size = countChars(slice);
  if (size <= leftCount) {
    return false;
  }

  let fragment = slice.content;

  while (true) {
    const currentCount = countChars(fragment);
    if (currentCount > leftCount) {
      fragment = truncateFragment(schema, fragment, leftCount);
    }

    const newSlice = Slice.maxOpen(fragment);
    const { doc } = state.tr.replaceSelection(newSlice);

    const expectedCount = countChars(doc);
    const overCount = expectedCount - maxCount;

    if (overCount <= 0) {
      break;
    }

    leftCount -= overCount;
  }

  if (fragment.size === 0) {
    return true;
  }

  const newSlice = Slice.maxOpen(fragment);
  view.dispatch(state.tr.replaceSelection(newSlice));

  return true;
};

const textLimit = (max: number) => {
  return new Plugin({
    key: new PluginKey('text_limit'),

    filterTransaction: (tr, state) => {
      if (!tr.docChanged) {
        return true;
      }

      if (tr.getMeta(ySyncPluginKey)) {
        return true;
      }

      const oldCount = countChars(state.doc);
      const newCount = countChars(tr.doc);

      if (oldCount < newCount) {
        return newCount <= max;
      }

      return true;
    },

    props: {
      handleTextInput: (view, _, __, text) => {
        const slice = new Slice(Fragment.from(view.state.schema.text(text)), 0, 0);
        return handleSlice(view, slice, max);
      },

      handlePaste: (view, _, slice) => {
        return handleSlice(view, slice, max);
      },

      handleDrop: (view, _, slice, moved) => {
        if (moved) {
          return false;
        }

        return handleSlice(view, slice, max);
      },
    },
  });
};

export const Limit = Extension.create({
  name: 'limit',

  addProseMirrorPlugins() {
    return [textLimit(50)];
  },
});
