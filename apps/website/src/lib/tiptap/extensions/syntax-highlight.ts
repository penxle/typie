import { Editor, Extension, findChildren } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { bundledLanguages, getSingletonHighlighter, getTokenStyleObject, stringifyTokenStyle } from 'shiki';
import Cookies from 'universal-cookie';
import type { Node } from '@tiptap/pm/model';
import type { Highlighter, TokenStyles } from 'shiki';

const THEME_COOKIE_NAME = 'typie-th';

type Storage = {
  highlighter: Highlighter | null;
};

const key = new PluginKey('syntax_highlight');

const getCurrentTheme = (): 'min-light' | 'min-dark' => {
  if (typeof window === 'undefined') {
    return 'min-light';
  }

  const theme = new Cookies().get(THEME_COOKIE_NAME);
  if (theme === 'dark') return 'min-dark';
  if (theme === 'light') return 'min-light';

  return window.matchMedia?.('(prefers-color-scheme: dark)').matches ? 'min-dark' : 'min-light';
};

export const SyntaxHighlight = Extension.create<unknown, Storage>({
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
          init: () => {
            getSingletonHighlighter({
              themes: ['min-light', 'min-dark'],
              langs: ['html'],
            }).then((highlighter) => {
              this.storage.highlighter = highlighter;

              if (this.editor.isDestroyed) return;

              const { tr } = this.editor.state;
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

        view: () => {
          const cookies = new Cookies();

          const triggerUpdate = () => {
            if (this.editor.isDestroyed) return;

            const { tr } = this.editor.state;
            tr.setMeta(key, true);
            this.editor.view.dispatch(tr);
          };

          const handleCookieChange = (options: { name: string }) => {
            if (options.name === THEME_COOKIE_NAME) {
              triggerUpdate();
            }
          };

          const handleSystemThemeChange = () => {
            const theme = cookies.get(THEME_COOKIE_NAME);
            if (theme === 'auto' || !theme) {
              triggerUpdate();
            }
          };

          cookies.addChangeListener(handleCookieChange);
          const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
          mediaQuery.addEventListener('change', handleSystemThemeChange);

          return {
            destroy: () => {
              cookies.removeChangeListener(handleCookieChange);
              mediaQuery.removeEventListener('change', handleSystemThemeChange);
            },
          };
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
  const currentTheme = getCurrentTheme();

  const children = findChildren(doc, (node) => node.type.spec.code === true);
  for (const child of children) {
    const code = child.node.textContent;
    if (code.length > 100_000) {
      continue;
    }

    const language = child.node.type.name === 'html_block' ? 'html' : child.node.attrs.language;
    if (!languages.has(language) && bundledLanguages[language as never]) {
      highlighter.loadLanguage(language).then(() => {
        const { tr } = editor.state;
        tr.setMeta(key, true);
        editor.view.dispatch(tr);
      });

      continue;
    }

    const result = highlighter.codeToTokens(code, { theme: currentTheme, lang: language });

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
