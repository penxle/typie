import { Mark, mergeAttributes } from '@tiptap/core';
import { css } from '@typie/styled-system/css';

export const Bold = Mark.create({
  name: 'bold',

  renderHTML({ HTMLAttributes }) {
    return ['b', mergeAttributes(HTMLAttributes, { class: css({ fontWeight: 'bold' }) }), 0];
  },
});
