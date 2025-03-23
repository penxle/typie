import { mergeAttributes, Node } from '@tiptap/core';
import { css } from '$styled-system/css';

export const TableRow = Node.create({
  name: 'table_row',
  content: 'table_cell+',
  tableRole: 'row',

  parseHTML() {
    return [{ tag: 'tr' }];
  },

  renderHTML({ HTMLAttributes }) {
    return [
      'tr',
      mergeAttributes(HTMLAttributes, {
        class: css({
          '&:last-child :is(td, th)': {
            borderBottom: 'none',
          },
        }),
      }),
      0,
    ];
  },
});
