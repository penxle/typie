import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { css } from '@typie/styled-system/css';
import { match } from 'ts-pattern';
import { isBodyEmpty } from '../lib';
import type { EditorState } from '@tiptap/pm/state';

export const Placeholder = Extension.create({
  name: 'placeholder',
  priority: 1000,

  addProseMirrorPlugins() {
    const key = new PluginKey('placeholder');

    const createDecorations = (state: EditorState) => {
      if (!this.editor.isEditable || !this.editor.isFocused || window.__webview__) {
        return DecorationSet.empty;
      }

      const { selection } = state;
      const { $anchor, empty } = selection;

      if (!empty) {
        return DecorationSet.empty;
      }

      if ($anchor.parent.childCount > 0) {
        return DecorationSet.empty;
      }

      let placeholder: string | null = null;
      let from: number | null = null;
      let to: number | null = null;

      if (
        $anchor.depth === 2 &&
        $anchor.parent.type.name === 'paragraph' &&
        ($anchor.parent.attrs.textAlign === 'left' || $anchor.parent.attrs.textAlign === 'justify')
      ) {
        if (!isBodyEmpty(state)) {
          placeholder = window.__webview__ ? '내용을 입력해보세요...' : '내용을 입력하거나 /를 입력해 블록 삽입하기...';
          from = $anchor.pos <= 2 ? 1 : $anchor.before();
          to = $anchor.pos <= 2 ? 3 : $anchor.after();
        }
      } else if ($anchor.depth >= 2) {
        const block = $anchor.node(2);
        if (block) {
          placeholder = match(block.type.name)
            .with('blockquote', () => '인용구')
            .with('callout', () => '콜아웃')
            .with('fold', () => '폴드')
            .with('bullet_list', 'ordered_list', () => '목록')
            .otherwise(() => null);

          if (placeholder) {
            from = $anchor.before();
            to = $anchor.after();
          }
        }
      }

      if (!placeholder) {
        return DecorationSet.empty;
      }

      if (from !== null && to !== null) {
        return DecorationSet.create(state.doc, [createDecoration(from, to, placeholder)]);
      }
    };

    return [
      new Plugin({
        key,
        state: {
          init: (_, state) => {
            return createDecorations(state);
          },
          apply: (tr, decorations, oldState, newState) => {
            if (!tr.docChanged && oldState.selection.eq(newState.selection)) {
              return decorations;
            }

            const oldPos = oldState.selection.$anchor;
            const newPos = newState.selection.$anchor;

            if (
              oldPos.parent !== newPos.parent ||
              oldPos.parent.childCount !== newPos.parent.childCount ||
              !oldState.selection.eq(newState.selection)
            ) {
              return createDecorations(newState);
            }

            return decorations?.map(tr.mapping, tr.doc);
          },
        },
        props: {
          decorations(state) {
            return this.getState(state);
          },
        },
      }),
    ];
  },
});

const createDecoration = (from: number, to: number, placeholder: string) => {
  return Decoration.node(from, to, {
    class: css({
      position: 'relative',
      _before: {
        content: 'attr(data-placeholder)',
        position: 'absolute',
        left: '0',
        color: 'text.disabled',
        pointerEvents: 'none',
      },
    }),
    'data-placeholder': placeholder,
  });
};
