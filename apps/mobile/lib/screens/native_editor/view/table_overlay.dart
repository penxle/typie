import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/table/models.dart';
import 'package:typie/screens/native_editor/view/scope.dart';

const _handleGap = -9.0;
const _colHandleWidth = 24.0;
const _colHandleHeight = 18.0;
const _rowHandleWidth = 18.0;
const _rowHandleHeight = 24.0;
const _sheetOpenDelay = Duration(milliseconds: 40);

typedef _OverlayDispatch = void Function(Map<String, dynamic> message, {bool requestFocus});

class TableOverlay extends HookWidget {
  const TableOverlay({required this.pageIndex, super.key});

  final int pageIndex;

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final overlays = useValueListenable(scope.controller.tableOverlays);

    TableOverlayInfo? focused;
    for (final overlay in overlays) {
      if (overlay.pageIdx == pageIndex && overlay.isFocused) {
        focused = overlay;
        break;
      }
    }

    if (focused == null) {
      return const SizedBox.shrink();
    }

    return Positioned.fill(child: _FocusedTableOverlay(overlay: focused));
  }
}

class _FocusedTableOverlay extends HookWidget {
  const _FocusedTableOverlay({required this.overlay});

  final TableOverlayInfo overlay;

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    useListenable(scope.controller);

    final page = scope.controller.state.layout?.pages.elementAtOrNull(overlay.pageIdx);
    final pageWidth = page?.width ?? overlay.bounds.x + overlay.bounds.width + 24;
    final pageHeight = page?.height ?? overlay.bounds.y + overlay.bounds.height + 24;
    final cursor = scope.controller.state.cursor;
    final hasOverlayGeometry =
        overlay.colWidths.isNotEmpty &&
        overlay.totalRows > 0 &&
        overlay.rowHeights.isNotEmpty &&
        overlay.rowPositions.isNotEmpty;

    final selectedRow = useState<int>(0);
    final selectedCol = useState<int>(0);
    useEffect(() {
      if (!hasOverlayGeometry) {
        return null;
      }
      final focused = _focusedCellFromCursor(overlay, cursor);
      selectedRow.value = focused?.row ?? _defaultRow(overlay);
      selectedCol.value = focused?.col ?? 0;
      return null;
    }, [overlay.tableId, hasOverlayGeometry]);

    useEffect(() {
      if (!hasOverlayGeometry) {
        return null;
      }
      selectedRow.value = selectedRow.value.clamp(0, overlay.totalRows - 1);
      selectedCol.value = selectedCol.value.clamp(0, overlay.colWidths.length - 1);
      return null;
    }, [overlay.totalRows, overlay.colWidths.length, hasOverlayGeometry]);

    useEffect(
      () {
        if (!hasOverlayGeometry) {
          return null;
        }
        final focused = _focusedCellFromCursor(overlay, cursor);
        if (focused == null) {
          return null;
        }
        if (selectedRow.value != focused.row) {
          selectedRow.value = focused.row;
        }
        if (selectedCol.value != focused.col) {
          selectedCol.value = focused.col;
        }
        return null;
      },
      [
        cursor?.pageIdx,
        cursor?.x,
        cursor?.y,
        cursor?.height,
        cursor?.visible,
        overlay.bounds.x,
        overlay.bounds.y,
        overlay.bounds.width,
        overlay.bounds.height,
        overlay.startRowIndex,
        overlay.totalRows,
        overlay.colPositions.length,
        overlay.rowPositions.length,
        hasOverlayGeometry,
      ],
    );

    if (!hasOverlayGeometry) {
      return const SizedBox.shrink();
    }

    void dispatch(Map<String, dynamic> message, {bool requestFocus = true}) {
      scope.controller.dispatch(message);
      scope.controller.scrollIntoView();
      if (requestFocus) {
        scope.inputController.requestFocus();
      }
    }

    void selectRow(int row, {bool requestFocus = true}) {
      final clamped = row.clamp(0, overlay.totalRows - 1);
      selectedRow.value = clamped;
      dispatch({'type': 'selectTableRow', 'tableId': overlay.tableId, 'row': clamped}, requestFocus: requestFocus);
    }

    void selectCol(int col, {bool requestFocus = true}) {
      final clamped = col.clamp(0, overlay.colWidths.length - 1);
      selectedCol.value = clamped;
      dispatch({'type': 'selectTableColumn', 'tableId': overlay.tableId, 'col': clamped}, requestFocus: requestFocus);
    }

    final currentRow = selectedRow.value.clamp(0, overlay.totalRows - 1);
    final currentCol = selectedCol.value.clamp(0, overlay.colWidths.length - 1);
    final selectedColLeft = _colLeft(overlay, currentCol);
    final selectedColWidth = overlay.colWidths[currentCol];

    final selectedRowLocal = currentRow - overlay.startRowIndex;
    final isSelectedRowVisible =
        selectedRowLocal >= 0 &&
        selectedRowLocal < overlay.rowHeights.length &&
        selectedRowLocal < overlay.rowPositions.length;
    final selectedRowTop = isSelectedRowVisible ? _rowTop(overlay, selectedRowLocal) : 0.0;
    final selectedRowHeight = isSelectedRowVisible ? overlay.rowHeights[selectedRowLocal] : 0.0;

    final colHandleLeft = _clampDouble(
      overlay.bounds.x + selectedColLeft + (selectedColWidth - _colHandleWidth) / 2,
      4,
      math.max(4, pageWidth - _colHandleWidth - 4),
    );
    final colHandleTop = _clampDouble(
      overlay.bounds.y - _colHandleHeight - _handleGap,
      4,
      math.max(4, pageHeight - _colHandleHeight - 4),
    );

    final rowHandleLeft = _clampDouble(
      overlay.bounds.x - _rowHandleWidth - _handleGap,
      4,
      math.max(4, pageWidth - _rowHandleWidth - 4),
    );
    final rowHandleTop = isSelectedRowVisible
        ? _clampDouble(
            overlay.bounds.y + selectedRowTop + (selectedRowHeight - _rowHandleHeight) / 2,
            4,
            math.max(4, pageHeight - _rowHandleHeight - 4),
          )
        : 0.0;

    void moveHandleRowTo(int row) {
      selectedRow.value = math.max(0, row);
    }

    void moveHandleColTo(int col) {
      selectedCol.value = math.max(0, col);
    }

    Future<void> openRowMenu() async {
      if (!isSelectedRowVisible) {
        return;
      }
      selectRow(currentRow, requestFocus: false);
      await _showRowActions(context, scope, overlay, currentRow, dispatch, onSelectedRowChanged: moveHandleRowTo);
    }

    Future<void> openColumnMenu() async {
      selectCol(currentCol, requestFocus: false);
      await _showColumnActions(context, scope, overlay, currentCol, dispatch, onSelectedColChanged: moveHandleColTo);
    }

    return Stack(
      clipBehavior: Clip.none,
      children: [
        Positioned(
          left: colHandleLeft,
          top: colHandleTop,
          child: _SelectorHandleButton(
            width: _colHandleWidth,
            height: _colHandleHeight,
            icon: LucideLightIcons.ellipsis,
            onTap: openColumnMenu,
          ),
        ),
        if (isSelectedRowVisible)
          Positioned(
            left: rowHandleLeft,
            top: rowHandleTop,
            child: _SelectorHandleButton(
              width: _rowHandleWidth,
              height: _rowHandleHeight,
              icon: LucideLightIcons.ellipsis_vertical,
              onTap: openRowMenu,
            ),
          ),
      ],
    );
  }

  Future<void> _showRowActions(
    BuildContext context,
    ContentScope scope,
    TableOverlayInfo overlay,
    int selectedRow,
    _OverlayDispatch dispatch, {
    required ValueChanged<int> onSelectedRowChanged,
  }) async {
    final isFirst = selectedRow == 0;
    final isLast = selectedRow == overlay.totalRows - 1;
    final isOnlyRow = overlay.totalRows <= 1;

    scope.inputController.dismissKeyboard();
    await Future<void>.delayed(_sheetOpenDelay);
    if (!context.mounted) {
      return;
    }
    final colors = context.colors;

    await context.showBottomSheet(
      overlayOpacity: 0.05,
      child: BottomMenu(
        header: Text(
          '행 ${selectedRow + 1}',
          style: TextStyle(fontSize: 17, fontWeight: FontWeight.w600, color: colors.textSubtle),
        ),
        items: [
          if (!isFirst)
            BottomMenuItem(
              icon: LucideLightIcons.arrow_up_to_line,
              label: '위에 행 추가',
              onTap: () {
                onSelectedRowChanged(selectedRow + 1);
                dispatch({'type': 'addTableRow', 'tableId': overlay.tableId, 'afterRow': selectedRow - 1});
              },
            ),
          BottomMenuItem(
            icon: LucideLightIcons.arrow_down_to_line,
            label: '아래에 행 추가',
            onTap: () {
              onSelectedRowChanged(selectedRow);
              dispatch({'type': 'addTableRow', 'tableId': overlay.tableId, 'afterRow': selectedRow});
            },
          ),
          if (!isFirst)
            BottomMenuItem(
              icon: LucideLightIcons.move_up,
              label: '위로 이동',
              onTap: () {
                onSelectedRowChanged(selectedRow - 1);
                dispatch({
                  'type': 'moveTableRow',
                  'tableId': overlay.tableId,
                  'fromRow': selectedRow,
                  'toRow': selectedRow - 1,
                });
              },
            ),
          if (!isLast)
            BottomMenuItem(
              icon: LucideLightIcons.move_down,
              label: '아래로 이동',
              onTap: () {
                onSelectedRowChanged(selectedRow + 1);
                dispatch({
                  'type': 'moveTableRow',
                  'tableId': overlay.tableId,
                  'fromRow': selectedRow,
                  'toRow': selectedRow + 1,
                });
              },
            ),
          BottomMenuItem(
            icon: LucideLightIcons.trash_2,
            label: isOnlyRow ? '테이블 삭제' : '행 삭제',
            iconColor: colors.textDanger,
            labelColor: colors.textDanger,
            onTap: () {
              if (isOnlyRow) {
                dispatch({'type': 'deleteNode', 'nodeId': overlay.tableId});
              } else {
                onSelectedRowChanged(selectedRow < overlay.totalRows - 1 ? selectedRow : selectedRow - 1);
                dispatch({'type': 'deleteTableRow', 'tableId': overlay.tableId, 'row': selectedRow});
              }
            },
          ),
        ],
      ),
    );
  }

  Future<void> _showColumnActions(
    BuildContext context,
    ContentScope scope,
    TableOverlayInfo overlay,
    int selectedCol,
    _OverlayDispatch dispatch, {
    required ValueChanged<int> onSelectedColChanged,
  }) async {
    final isFirst = selectedCol == 0;
    final isLast = selectedCol == overlay.colWidths.length - 1;
    final isOnlyColumn = overlay.colWidths.length <= 1;

    scope.inputController.dismissKeyboard();
    await Future<void>.delayed(_sheetOpenDelay);
    if (!context.mounted) {
      return;
    }
    final colors = context.colors;

    await context.showBottomSheet(
      overlayOpacity: 0.05,
      child: BottomMenu(
        header: Text(
          '열 ${selectedCol + 1}',
          style: TextStyle(fontSize: 17, fontWeight: FontWeight.w600, color: colors.textSubtle),
        ),
        items: [
          if (!isFirst)
            BottomMenuItem(
              icon: LucideLightIcons.arrow_left_to_line,
              label: '왼쪽에 열 추가',
              onTap: () {
                onSelectedColChanged(selectedCol + 1);
                dispatch({'type': 'addTableColumn', 'tableId': overlay.tableId, 'afterCol': selectedCol - 1});
              },
            ),
          BottomMenuItem(
            icon: LucideLightIcons.arrow_right_to_line,
            label: '오른쪽에 열 추가',
            onTap: () {
              onSelectedColChanged(selectedCol);
              dispatch({'type': 'addTableColumn', 'tableId': overlay.tableId, 'afterCol': selectedCol});
            },
          ),
          if (!isFirst)
            BottomMenuItem(
              icon: LucideLightIcons.move_left,
              label: '왼쪽으로 이동',
              onTap: () {
                onSelectedColChanged(selectedCol - 1);
                dispatch({
                  'type': 'moveTableColumn',
                  'tableId': overlay.tableId,
                  'fromCol': selectedCol,
                  'toCol': selectedCol - 1,
                });
              },
            ),
          if (!isLast)
            BottomMenuItem(
              icon: LucideLightIcons.move_right,
              label: '오른쪽으로 이동',
              onTap: () {
                onSelectedColChanged(selectedCol + 1);
                dispatch({
                  'type': 'moveTableColumn',
                  'tableId': overlay.tableId,
                  'fromCol': selectedCol,
                  'toCol': selectedCol + 1,
                });
              },
            ),
          BottomMenuItem(
            icon: LucideLightIcons.trash_2,
            label: isOnlyColumn ? '테이블 삭제' : '열 삭제',
            iconColor: colors.textDanger,
            labelColor: colors.textDanger,
            onTap: () {
              if (isOnlyColumn) {
                dispatch({'type': 'deleteNode', 'nodeId': overlay.tableId});
              } else {
                onSelectedColChanged(selectedCol < overlay.colWidths.length - 1 ? selectedCol : selectedCol - 1);
                dispatch({'type': 'deleteTableColumn', 'tableId': overlay.tableId, 'col': selectedCol});
              }
            },
          ),
        ],
      ),
    );
  }
}

class _SelectorHandleButton extends StatelessWidget {
  const _SelectorHandleButton({required this.width, required this.height, required this.icon, required this.onTap});

  final double width;
  final double height;
  final IconData icon;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      child: Container(
        width: width,
        height: height,
        decoration: BoxDecoration(
          color: context.colors.surfaceDefault,
          borderRadius: BorderRadius.circular(6),
          border: Border.all(color: context.colors.borderStrong),
          boxShadow: [
            BoxShadow(color: Colors.black.withValues(alpha: 0.08), offset: const Offset(0, 1), blurRadius: 4),
          ],
        ),
        child: Icon(icon, size: 14, color: context.colors.textSubtle),
      ),
    );
  }
}

class _TableCellIndex {
  const _TableCellIndex({required this.row, required this.col});

  final int row;
  final int col;
}

int _defaultRow(TableOverlayInfo overlay) {
  return overlay.startRowIndex.clamp(0, overlay.totalRows - 1);
}

double _clampDouble(double value, double min, double max) {
  if (max < min) {
    return min;
  }
  return value.clamp(min, max);
}

int _indexForOffset(List<double> positions, double offset) {
  for (var i = 0; i < positions.length; i++) {
    if (offset < positions[i]) {
      return i;
    }
  }
  return positions.length - 1;
}

double _colLeft(TableOverlayInfo overlay, int colIndex) {
  if (colIndex <= 0) {
    return 0;
  }
  return overlay.colPositions[colIndex - 1];
}

double _rowTop(TableOverlayInfo overlay, int localRowIndex) {
  if (localRowIndex <= 0) {
    return 0;
  }
  return overlay.rowPositions[localRowIndex - 1];
}

_TableCellIndex? _focusedCellFromCursor(TableOverlayInfo overlay, CursorInfo? cursor) {
  if (cursor == null || !cursor.visible || cursor.pageIdx != overlay.pageIdx) {
    return null;
  }
  if (overlay.colPositions.isEmpty || overlay.rowPositions.isEmpty) {
    return null;
  }

  final localX = cursor.x - overlay.bounds.x;
  final localY = cursor.y + cursor.height * 0.5 - overlay.bounds.y;
  if (localX < 0 || localY < 0 || localX > overlay.bounds.width || localY > overlay.bounds.height) {
    return null;
  }

  final localRow = _indexForOffset(overlay.rowPositions, localY);
  final localCol = _indexForOffset(overlay.colPositions, localX);

  return _TableCellIndex(
    row: (overlay.startRowIndex + localRow).clamp(0, overlay.totalRows - 1),
    col: localCol.clamp(0, overlay.colWidths.length - 1),
  );
}
