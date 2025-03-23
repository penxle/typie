import { mergeAttributes, Node } from '@tiptap/core';
import { css } from '$styled-system/css';

export const TableCell = Node.create({
  name: 'table_cell',
  content: 'block+',
  isolating: true,
  tableRole: 'cell',

  addAttributes() {
    return {
      colspan: {
        default: 1,
      },
      rowspan: {
        default: 1,
      },
      colwidth: {
        default: null,
        parseHTML: (element) => {
          const colwidth = element.getAttribute('colwidth');
          const value = colwidth ? colwidth.split(',').map((width) => Number.parseInt(width, 10)) : null;

          return value;
        },
      },
    };
  },

  parseHTML() {
    return [{ tag: 'td' }];
  },

  renderHTML({ HTMLAttributes }) {
    return [
      'td',
      mergeAttributes(HTMLAttributes, {
        class: css({
          borderWidth: '1px',
          borderTopWidth: '0',
          borderColor: 'gray.300',
          paddingX: '14px',
          paddingY: '10px',
        }),
      }),
      0,
    ];
  },
});
