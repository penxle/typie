import { Extension } from '@tiptap/core';
import { Plugin } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import escape from 'escape-string-regexp';
import { absolutePositionToRelativePosition, relativePositionToAbsolutePosition, ySyncPluginKey } from 'y-prosemirror';
import { css } from '$styled-system/css';

type Match = {
  from: number;
  to: number;
  relativeFrom: unknown;
  relativeTo: unknown;
};

type SearchStorage = {
  text: string;
  currentIndex: number;
  matches: Match[];
};

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    search: {
      search: (text: string) => ReturnType;
      findNext: () => ReturnType;
      findPrevious: () => ReturnType;
      replace: (replacement: string) => ReturnType;
      replaceAll: (replacement: string) => ReturnType;
      clearSearch: () => ReturnType;
    };
  }

  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Storage {
    search: SearchStorage;
  }
}

export const Search = Extension.create<unknown, SearchStorage>({
  name: 'search',

  addStorage() {
    return {
      text: '',
      currentIndex: 0,
      matches: [],
    };
  },

  addCommands() {
    const performSearch = (text: string) => {
      const pattern = new RegExp(escape(text), 'gi');
      const matches: Match[] = [];

      const { doc } = this.editor.state;
      const { binding } = ySyncPluginKey.getState(this.editor.view.state);

      doc.descendants((node, pos) => {
        if (!node.isText || !node.text) return;

        const m = [...node.text.matchAll(pattern)];
        for (const match of m) {
          const from = pos + match.index;
          const to = from + match[0].length;

          matches.push({
            from,
            to,
            relativeFrom: absolutePositionToRelativePosition(from, binding.type, binding.mapping),
            relativeTo: absolutePositionToRelativePosition(to, binding.type, binding.mapping),
          });
        }
      });

      return { matches };
    };

    return {
      search:
        (text: string) =>
        ({ tr, dispatch, chain }) => {
          this.storage.text = text;

          if (!text) {
            this.storage.matches = [];
            this.storage.currentIndex = -1;
            dispatch?.(tr);

            return true;
          }

          const result = performSearch(text);
          if (result.matches.length === 0) {
            this.storage.matches = [];
            this.storage.currentIndex = -1;
            dispatch?.(tr);

            return true;
          }

          this.storage.matches = result.matches;
          this.storage.currentIndex = 0;

          const match = this.storage.matches[this.storage.currentIndex];
          return chain().setTextSelection(match.to).scrollIntoView().run();
        },

      findNext:
        () =>
        ({ chain }) => {
          this.storage.currentIndex = (this.storage.currentIndex + 1) % this.storage.matches.length;

          const match = this.storage.matches[this.storage.currentIndex];
          return chain().setTextSelection(match.to).scrollIntoView().run();
        },

      findPrevious:
        () =>
        ({ chain }) => {
          this.storage.currentIndex = (this.storage.currentIndex - 1 + this.storage.matches.length) % this.storage.matches.length;

          const match = this.storage.matches[this.storage.currentIndex];
          return chain().setTextSelection(match.to).scrollIntoView().run();
        },

      replace:
        (replacement: string) =>
        ({ chain }) => {
          if (this.storage.matches.length === 0) return false;

          const match = this.storage.matches[this.storage.currentIndex];
          this.storage.matches.splice(this.storage.currentIndex, 1);

          if (this.storage.matches.length === 0) {
            this.storage.currentIndex = -1;
          } else {
            this.storage.currentIndex = this.storage.currentIndex % this.storage.matches.length;
          }

          return chain().setTextSelection({ from: match.from, to: match.to }).insertContent(replacement).scrollIntoView().run();
        },

      replaceAll:
        (replacement: string) =>
        ({ chain }) => {
          if (this.storage.matches.length === 0) return false;

          let command = chain();

          let offset = 0;
          for (const match of this.storage.matches) {
            command = command.setTextSelection({ from: match.from + offset, to: match.to + offset }).insertContent(replacement);
            offset += replacement.length - (match.to - match.from);
          }

          this.storage.matches = [];
          this.storage.currentIndex = -1;

          return command.run();
        },

      clearSearch:
        () =>
        ({ tr, dispatch }) => {
          this.storage.text = '';
          this.storage.currentIndex = -1;
          this.storage.matches = [];

          dispatch?.(tr);

          return true;
        },
    };
  },

  onTransaction({ editor, transaction }) {
    const { binding } = ySyncPluginKey.getState(editor.view.state);

    if (transaction.docChanged) {
      this.storage.matches = this.storage.matches
        .map((match) => {
          const from = relativePositionToAbsolutePosition(binding.doc, binding.type, match.relativeFrom, binding.mapping);
          const to = relativePositionToAbsolutePosition(binding.doc, binding.type, match.relativeTo, binding.mapping);

          if (from === null || to === null) {
            return null;
          }

          return { ...match, from, to };
        })
        .filter((match) => match !== null);
    }
  },

  addProseMirrorPlugins() {
    const storage = this.storage;

    return [
      new Plugin({
        props: {
          decorations(state) {
            return DecorationSet.create(
              state.doc,
              storage.matches.map((match, index) =>
                Decoration.inline(match.from, match.to, {
                  class: css({
                    color: '[#000]',
                    backgroundColor: '[#ffd700]',
                    '&[data-current-match="true"]': {
                      color: '[#fff]',
                      backgroundColor: '[#ff6b00]',
                    },
                  }),
                  'data-current-match': storage.currentIndex === index ? 'true' : 'false',
                }),
              ),
            );
          },
        },
      }),
    ];
  },
});
