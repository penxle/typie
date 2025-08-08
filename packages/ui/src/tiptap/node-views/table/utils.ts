import { findParentNodeClosestToPos } from '@tiptap/core';
import { Node as ProseMirrorNode } from '@tiptap/pm/model';
import { CellSelection } from '@tiptap/pm/tables';
import type { KeyboardShortcutCommand } from '@tiptap/core';
import type { DOMOutputSpec } from '@tiptap/pm/model';

export function getColStyleDeclaration(minWidth: number, width: number | undefined): [string, string] {
  if (width) {
    // apply the stored width unless it is below the configured minimum cell width
    return ['width', `${Math.max(width, minWidth)}px`];
  }

  // set the minimum with on the column if it has no stored width
  return ['min-width', `${minWidth}px`];
}

export type ColGroup =
  | {
      colgroup: DOMOutputSpec;
      tableWidth: string;
      tableMinWidth: string;
    }
  | Record<string, never>;

/**
 * Creates a colgroup element for a table node in ProseMirror.
 *
 * @param node - The ProseMirror node representing the table.
 * @param cellMinWidth - The minimum width of a cell in the table.
 * @param overrideCol - (Optional) The index of the column to override the width of.
 * @param overrideValue - (Optional) The width value to use for the overridden column.
 * @returns An object containing the colgroup element, the total width of the table, and the minimum width of the table.
 */
export function createColGroup(node: ProseMirrorNode, cellMinWidth: number): ColGroup;
export function createColGroup(node: ProseMirrorNode, cellMinWidth: number, overrideCol: number, overrideValue: number): ColGroup;
export function createColGroup(node: ProseMirrorNode, cellMinWidth: number, overrideCol?: number, overrideValue?: number): ColGroup {
  let totalWidth = 0;
  let fixedWidth = true;
  const cols: DOMOutputSpec[] = [];
  const row = node.firstChild;

  if (!row) {
    return {};
  }

  for (let i = 0, col = 0; i < row.childCount; i += 1) {
    const { colspan, colwidth } = row.child(i).attrs;

    for (let j = 0; j < colspan; j += 1, col += 1) {
      const hasWidth = overrideCol === col ? overrideValue : colwidth && (colwidth[j] as number | undefined);

      totalWidth += hasWidth || cellMinWidth;

      if (!hasWidth) {
        fixedWidth = false;
      }

      const [property, value] = getColStyleDeclaration(cellMinWidth, hasWidth);

      cols.push(['col', { style: `${property}: ${value}` }]);
    }
  }

  const tableWidth = fixedWidth ? `${totalWidth}px` : '';
  const tableMinWidth = fixedWidth ? '' : `${totalWidth}px`;

  const colgroup: DOMOutputSpec = ['colgroup', {}, ...cols];

  return { colgroup, tableWidth, tableMinWidth };
}

export function isCellSelection(value: unknown): value is CellSelection {
  return value instanceof CellSelection;
}

export const deleteTableWhenAllCellsSelected: KeyboardShortcutCommand = ({ editor }) => {
  const { selection } = editor.state;

  if (!isCellSelection(selection)) {
    return false;
  }

  let cellCount = 0;
  const table = findParentNodeClosestToPos(selection.ranges[0].$from, (node) => {
    return node.type.name === 'table';
  });

  table?.node.descendants((node) => {
    if (node.type.name === 'table') {
      return false;
    }

    if (['tableCell', 'tableHeader'].includes(node.type.name)) {
      cellCount += 1;
    }
  });

  const allCellsSelected = cellCount === selection.ranges.length;

  if (!allCellsSelected) {
    return false;
  }

  editor.commands.deleteTable();

  return true;
};
