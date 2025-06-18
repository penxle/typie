import { createNodeView } from '../../lib';
import Component from './TableRow.svelte';

export const TableRow = createNodeView(Component, {
  name: 'table_row',
  content: 'table_cell+',
  tableRole: 'row',

  parseHTML() {
    return [{ tag: 'tr' }];
  },
});
