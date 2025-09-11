import { Extension, textInputRule } from '@tiptap/core';

// NOTE: 스마트 따옴표 변환은 AutoSurround에서도 처리함
export const Typography = Extension.create({
  name: 'typography',

  addInputRules() {
    return [
      textInputRule({
        find: /--$/,
        replace: '—',
      }),

      textInputRule({
        find: /\.{3}$/,
        replace: '…',
      }),

      textInputRule({
        find: /\u201C[^\u201D]*(")$/,
        replace: '”',
      }),
      textInputRule({
        find: /(?:^|[^\u201C])(")$/,
        replace: '“',
      }),

      textInputRule({
        find: /\u2018[^\u2019]*(')$/,
        replace: '’',
      }),
      textInputRule({
        find: /(?:^|[^\u2018])(')$/,
        replace: '‘',
      }),
    ];
  },
});
