import { Extension } from '@tiptap/core';
import { Plugin, PluginKey, TextSelection } from '@tiptap/pm/state';

export type AutoSurroundOptions = {
  pairs: {
    trigger: string;
    left: string;
    right: string;
  }[];
};

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Storage {
    autoSurround: { enabled: boolean };
  }
}

export const AutoSurround = Extension.create<AutoSurroundOptions>({
  name: 'autoSurround',

  priority: 1000, // NOTE: Typography보다 높은 우선순위여야 함

  addOptions() {
    return {
      pairs: [
        { trigger: '(', left: '(', right: ')' },
        { trigger: '[', left: '[', right: ']' },
        { trigger: '{', left: '{', right: '}' },
        { trigger: '"', left: '“', right: '”' }, // 스마트 따옴표 변환
        { trigger: "'", left: '‘', right: '’' }, // 스마트 따옴표 변환
        { trigger: '“', left: '“', right: '”' },
        { trigger: '‘', left: '‘', right: '’' },
        { trigger: '`', left: '`', right: '`' },
        { trigger: '<', left: '<', right: '>' },
        { trigger: '「', left: '「', right: '」' },
        { trigger: '『', left: '『', right: '』' },
        { trigger: '《', left: '《', right: '》' },
        { trigger: '〈', left: '〈', right: '〉' },
        { trigger: '【', left: '【', right: '】' },
        { trigger: '〔', left: '〔', right: '〕' },
        { trigger: '*', left: '*', right: '*' },
        { trigger: '_', left: '_', right: '_' },
        { trigger: '=', left: '=', right: '=' },
        { trigger: '+', left: '+', right: '+' },
        { trigger: '-', left: '-', right: '-' },
        { trigger: '~', left: '~', right: '~' },
        { trigger: '|', left: '|', right: '|' },
        { trigger: '^', left: '^', right: '^' },
      ],
    };
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey('autoSurround'),
        priority: 1000,
        props: {
          handleTextInput: (view, _from, _to, text) => {
            const { state, dispatch } = view;
            const { selection } = state;

            if (!this.editor.storage.autoSurround.enabled) {
              return false;
            }

            if (selection.empty) {
              return false;
            }

            const pair = this.options.pairs.find((p) => p.trigger === text);
            if (!pair) {
              return false;
            }

            const selectedText = state.doc.textBetween(selection.from, selection.to, '');

            const tr = state.tr;

            tr.insertText(pair.left + selectedText + pair.right, selection.from, selection.to);

            const newPos = selection.from + pair.left.length + selectedText.length + pair.right.length;
            tr.setSelection(TextSelection.create(tr.doc, selection.from, newPos));

            dispatch(tr);
            return true;
          },
        },
      }),
    ];
  },
});
