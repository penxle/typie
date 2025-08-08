import { createNodeView } from '../../lib';
import Component from './TableCell.svelte';

export const TableCell = createNodeView(Component, {
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
});
