import { Editor, Extension, findChildren } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { bundledLanguages, getSingletonHighlighter, getTokenStyleObject, stringifyTokenStyle } from 'shiki';
import type { Node } from '@tiptap/pm/model';
import type { Highlighter, TokenStyles } from 'shiki';

type Storage = {
  highlighter: Highlighter | null;
};

const key = new PluginKey('syntax_highlight');

export const SyntaxHighlight = Extension.create<never, Storage>({
  name: 'syntax_highlight',

  addStorage() {
    return {
      highlighter: null,
    };
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key,
        state: {
          init: (_, state) => {
            getSingletonHighlighter({
              themes: ['min-light'],
              langs: ['html'],
            }).then((highlighter) => {
              this.storage.highlighter = highlighter;

              const { tr } = state;
              tr.setMeta(key, true);
              this.editor.view.dispatch(tr);
            });

            return DecorationSet.empty;
          },
          apply: (tr, decorationSet) => {
            if (!this.storage.highlighter) {
              return DecorationSet.empty;
            }

            if (tr.getMeta(key) === true || tr.docChanged) {
              return getDecorations(this.editor, this.storage.highlighter, tr.doc);
            }

            return decorationSet.map(tr.mapping, tr.doc);
          },
        },
        props: {
          decorations: (state) => {
            return key.getState(state);
          },
        },
      }),
    ];
  },
});

const themedTokenToStyle = (token: TokenStyles) => {
  return stringifyTokenStyle(token.htmlStyle || getTokenStyleObject(token));
};

const getDecorations = (editor: Editor, highlighter: Highlighter, doc: Node) => {
  const decorations: Decoration[] = [];
  const languages = new Set(['text', ...highlighter.getLoadedLanguages()]);

  const children = findChildren(doc, (node) => node.type.spec.code === true);
  for (const child of children) {
    const language = child.node.type.name === 'html_block' ? 'html' : child.node.attrs.language;
    if (!languages.has(language) && bundledLanguages[language as never]) {
      highlighter.loadLanguage(language).then(() => {
        const { tr } = editor.state;
        tr.setMeta(key, true);
        editor.view.dispatch(tr);
      });

      continue;
    }

    const result = highlighter.codeToTokens(child.node.textContent, { theme: 'min-light', lang: language });

    for (const token of result.tokens.flat()) {
      const from = child.pos + token.offset + 1;
      const to = from + token.content.length;

      decorations.push(
        Decoration.inline(from, to, {
          style: themedTokenToStyle(token),
        }),
      );
    }
  }

  return DecorationSet.create(doc, decorations);
};
