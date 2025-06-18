import { callOrReturn, getExtensionField } from '@tiptap/core';
import {
  addColumnAfter,
  addColumnBefore,
  addRowAfter,
  addRowBefore,
  CellSelection,
  columnResizing,
  deleteColumn,
  deleteRow,
  deleteTable,
  fixTables,
  goToNextCell,
  mergeCells,
  setCellAttr,
  splitCell,
  tableEditing,
} from '@tiptap/pm/tables';
import { createNodeView } from '../../lib';
import Component from './Table.svelte';
import { deleteTableWhenAllCellsSelected } from './utils';
import type { ParentConfig } from '@tiptap/core';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    table: {
      insertTable: (options?: { rows?: number; cols?: number }) => ReturnType;
      addRowBefore: () => ReturnType;
      addRowAfter: () => ReturnType;
      addColumnBefore: () => ReturnType;
      addColumnAfter: () => ReturnType;
      deleteRow: () => ReturnType;
      deleteColumn: () => ReturnType;
      deleteTable: () => ReturnType;
      mergeCells: () => ReturnType;
      splitCell: () => ReturnType;
      setCellAttribute: (name: string, value: unknown) => ReturnType;
      setCellSelection: (position: { anchorCell: number; headCell?: number }) => ReturnType;
      goToNextCell: () => ReturnType;
      goToPreviousCell: () => ReturnType;
      fixTables: () => ReturnType;
      setTableBorderStyle: (borderStyle: string) => ReturnType;
    };
  }

  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface NodeConfig<Options, Storage> {
    tableRole?:
      | string
      | ((this: { name: string; options: Options; storage: Storage; parent: ParentConfig<NodeConfig<Options>>['tableRole'] }) => string);
  }
}

export const Table = createNodeView(Component, {
  name: 'table',
  group: 'block',
  content: 'table_row+',
  isolating: true,
  tableRole: 'table',

  addAttributes() {
    return {
      borderStyle: {
        default: 'solid',
        parseHTML: (element) => {
          return element.dataset.borderStyle;
        },
        renderHTML: ({ borderStyle }) => ({
          'data-border-style': borderStyle,
        }),
      },
    };
  },

  parseHTML() {
    return [{ tag: 'table' }];
  },

  addCommands() {
    return {
      insertTable:
        ({ rows: rowsCount = 3, cols: colsCount = 3 } = {}) =>
        ({ can, commands, state }) => {
          if (!can().isNodeAllowed(this.name)) {
            return false;
          }

          const cells = [];
          for (let index = 0; index < colsCount; index += 1) {
            // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
            cells.push(state.schema.nodes.table_cell.createAndFill()!);
          }

          const rows = [];
          for (let index = 0; index < rowsCount; index += 1) {
            rows.push(state.schema.nodes.table_row.createChecked(null, cells));
          }

          const table = state.schema.nodes.table.createChecked(null, rows);

          return commands.insertNode(table);
        },
      addRowBefore:
        () =>
        ({ state, dispatch }) => {
          return addRowBefore(state, dispatch);
        },
      addRowAfter:
        () =>
        ({ state, dispatch }) => {
          return addRowAfter(state, dispatch);
        },
      addColumnBefore:
        () =>
        ({ state, dispatch }) => {
          return addColumnBefore(state, dispatch);
        },
      addColumnAfter:
        () =>
        ({ state, dispatch }) => {
          return addColumnAfter(state, dispatch);
        },
      deleteRow:
        () =>
        ({ state, dispatch }) => {
          return deleteRow(state, dispatch);
        },
      deleteColumn:
        () =>
        ({ state, dispatch }) => {
          return deleteColumn(state, dispatch);
        },
      deleteTable:
        () =>
        ({ state, dispatch }) => {
          return deleteTable(state, dispatch);
        },
      mergeCells:
        () =>
        ({ state, dispatch }) => {
          return mergeCells(state, dispatch);
        },
      splitCell:
        () =>
        ({ state, dispatch }) => {
          return splitCell(state, dispatch);
        },
      setCellAttribute:
        (name, value) =>
        ({ state, dispatch }) => {
          return setCellAttr(name, value)(state, dispatch);
        },
      setCellSelection:
        (position) =>
        ({ tr, dispatch }) => {
          if (dispatch) {
            const selection = CellSelection.create(tr.doc, position.anchorCell, position.headCell);
            tr.setSelection(selection);
          }

          return true;
        },
      goToNextCell:
        () =>
        ({ state, dispatch }) => {
          return goToNextCell(1)(state, dispatch);
        },
      goToPreviousCell:
        () =>
        ({ state, dispatch }) => {
          return goToNextCell(-1)(state, dispatch);
        },
      fixTables:
        () =>
        ({ state, dispatch }) => {
          if (dispatch) {
            fixTables(state);
          }

          return true;
        },
      setTableBorderStyle:
        (borderStyle: string) =>
        ({ commands }) => {
          return commands.updateAttributes(this.name, { borderStyle });
        },
    };
  },

  addKeyboardShortcuts() {
    return {
      Tab: () => {
        if (this.editor.commands.goToNextCell()) {
          return true;
        }

        if (!this.editor.can().addRowAfter()) {
          return false;
        }

        return this.editor.chain().addRowAfter().goToNextCell().run();
      },
      'Shift-Tab': () => this.editor.commands.goToPreviousCell(),
      Backspace: deleteTableWhenAllCellsSelected,
      'Mod-Backspace': deleteTableWhenAllCellsSelected,
      Delete: deleteTableWhenAllCellsSelected,
      'Mod-Delete': deleteTableWhenAllCellsSelected,
    };
  },

  addProseMirrorPlugins() {
    return [
      ...(this.editor.isEditable && !window.__webview__
        ? [
            columnResizing({
              handleWidth: 5,
              cellMinWidth: 50,
              defaultCellMinWidth: 50,
              View: null,
              lastColumnResizable: false,
            }),
          ]
        : []),
      tableEditing({
        allowTableNodeSelection: true,
      }),
    ];
  },

  extendNodeSchema(extension) {
    const context = {
      name: extension.name,
      options: extension.options,
      storage: extension.storage,
    };

    return {
      tableRole: callOrReturn(getExtensionField(extension, 'tableRole', context)),
    };
  },
});
