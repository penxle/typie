import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';

const ZERO_WIDTH_SPACE = '\u200B';
const DOC_START_POS = 2;

/**
 * NOTE: Safari에서 첫 글자 조합 시 중복 입력되는 문제 회피를 위한 extension
 */
export const SafariIMEFix = Extension.create({
  name: 'safariIMEFix',

  addProseMirrorPlugins() {
    const isSafari = /^((?!chrome|android).)*safari/i.test(navigator.userAgent);

    if (!isSafari) {
      return [];
    }

    const key = new PluginKey('safariIMEFix');

    return [
      new Plugin({
        key,
        props: {
          handleDOMEvents: {
            compositionstart: (view) => {
              const { doc } = view.state;
              if (doc.textContent === '') {
                const { state, dispatch } = view;
                const tr = state.tr;
                tr.insertText(ZERO_WIDTH_SPACE, DOC_START_POS);
                tr.setMeta('addToHistory', false);
                dispatch(tr);
              }
              return false;
            },

            compositionupdate: () => {
              return false;
            },

            compositionend: (view) => {
              const { state, dispatch } = view;
              const { doc, tr } = state;

              if (doc.textContent.startsWith(ZERO_WIDTH_SPACE)) {
                tr.delete(DOC_START_POS, DOC_START_POS + 1);
                tr.setMeta('addToHistory', false);
                dispatch(tr);
              }

              return false;
            },
          },
        },
      }),
    ];
  },
});
