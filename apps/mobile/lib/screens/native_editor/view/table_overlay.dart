import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/table/models.dart';
import 'package:typie/screens/native_editor/view/document_overlay_layer.dart';
import 'package:typie/screens/native_editor/view/gesture.dart';
import 'package:typie/screens/native_editor/view/scope.dart';

import 'table_overlay/cell_selector.dart';
import 'table_overlay/column_resizer.dart';
import 'table_overlay/constants.dart';
import 'table_overlay/geometry.dart';
import 'table_overlay/handles.dart';

typedef OverlayDispatch = void Function(Map<String, dynamic> message);

List<Widget> _optionalWidget(Widget? widget) {
  if (widget == null) {
    return const [];
  }
  return [widget];
}

class TableOverlay extends HookWidget {
  const TableOverlay({
    required this.gesture,
    required this.viewWidth,
    required this.viewHeight,
    required this.dropPosition,
    required this.globalToViewport,
    super.key,
  });

  final GestureController gesture;
  final double viewWidth;
  final double viewHeight;
  final ValueNotifier<Offset?> dropPosition;
  final Offset? Function(Offset globalPosition) globalToViewport;

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final overlays = useValueListenable(scope.controller.tableOverlays);
    final layout = scope.controller.state.layout;
    if (layout == null || layout.pages.isEmpty) {
      return const SizedBox.shrink();
    }

    TableOverlayInfo? focused;
    for (final overlay in overlays) {
      if (overlay.isFocused) {
        focused = overlay;
        break;
      }
    }

    if (focused == null) {
      return const SizedBox.shrink();
    }
    final focusedOverlay = focused;

    return DocumentOverlayLayer(
      builder: (context, viewport) {
        if (!viewport.hasPage(focusedOverlay.pageIdx)) {
          return const SizedBox.shrink();
        }
        final pageRect = viewport.pageRect(focusedOverlay.pageIdx);
        final overlayWidth = math.max(pageRect.width, focusedOverlay.bounds.x + focusedOverlay.bounds.width + 24);
        final overlayHeight = math.max(pageRect.height, focusedOverlay.bounds.y + focusedOverlay.bounds.height + 24);
        return Positioned(
          left: pageRect.left,
          top: pageRect.top,
          width: overlayWidth,
          height: overlayHeight,
          child: _FocusedTableOverlay(
            overlay: focusedOverlay,
            gesture: gesture,
            viewWidth: viewWidth,
            viewHeight: viewHeight,
            dropPosition: dropPosition,
            globalToViewport: globalToViewport,
          ),
        );
      },
    );
  }
}

class _FocusedTableOverlay extends HookWidget {
  const _FocusedTableOverlay({
    required this.overlay,
    required this.gesture,
    required this.viewWidth,
    required this.viewHeight,
    required this.dropPosition,
    required this.globalToViewport,
  });

  final TableOverlayInfo overlay;
  final GestureController gesture;
  final double viewWidth;
  final double viewHeight;
  final ValueNotifier<Offset?> dropPosition;
  final Offset? Function(Offset globalPosition) globalToViewport;

  @override
  Widget build(BuildContext context) {
    final selectionHandleColor = context.colors.textDefault;
    final scope = ContentScope.of(context);
    useListenable(scope.controller);

    final page = scope.controller.state.layout?.pages.elementAtOrNull(overlay.pageIdx);
    final layout = scope.controller.state.layout;
    final renderBounds = overlay.bounds;
    final pageWidth = math.max(
      page?.width ?? renderBounds.x + renderBounds.width + 24,
      renderBounds.x + renderBounds.width + 24,
    );
    final pageHeight = math.max(
      page?.height ?? renderBounds.y + renderBounds.height + 24,
      renderBounds.y + renderBounds.height + 24,
    );
    final cursor = scope.controller.state.cursor;
    final hasOverlayGeometry =
        overlay.colWidths.isNotEmpty &&
        overlay.totalRows > 0 &&
        overlay.rowHeights.isNotEmpty &&
        overlay.rowPositions.isNotEmpty;

    final selectedRow = useState<int>(0);
    final selectedCol = useState<int>(0);
    final selection = scope.controller.state.selection;
    final cellSelectionDrag = useState<CellSelectionDragDraft?>(null);
    final cellHandleDragPosition = useValueNotifier<Offset?>(null);

    useEffect(() {
      if (!hasOverlayGeometry) {
        return null;
      }
      final focused = tableFocusedCellFromCursor(overlay, cursor, layout);
      selectedRow.value = focused?.row ?? tableDefaultRow(overlay);
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
        final focused = tableFocusedCellFromCursor(overlay, cursor, layout);
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
        renderBounds.x,
        renderBounds.y,
        renderBounds.width,
        renderBounds.height,
        overlay.startRowIndex,
        overlay.totalRows,
        overlay.colPositions.length,
        overlay.rowPositions.length,
        hasOverlayGeometry,
        layout?.isPaginated,
        layout?.pages.length,
      ],
    );

    useEffect(() {
      resetCellSelectionDragIfTableChanged(draftState: cellSelectionDrag, tableId: overlay.tableId);
      return null;
    }, [cellSelectionDrag.value, overlay.tableId]);

    if (!hasOverlayGeometry) {
      return const SizedBox.shrink();
    }

    void dispatch(Map<String, dynamic> message) {
      scope.controller.dispatch(message);
    }

    void dispatchAndScrollAndFocus(Map<String, dynamic> message) {
      dispatch(message);
      scope.controller.scrollIntoView();
      scope.inputController.requestFocus();
    }

    void selectRow(int row) {
      final clamped = row.clamp(0, overlay.totalRows - 1);
      selectedRow.value = clamped;
      dispatch({'type': 'selectTableRow', 'tableId': overlay.tableId, 'row': clamped});
      scope.controller.scrollIntoView();
    }

    void selectCol(int col) {
      final clamped = col.clamp(0, overlay.colWidths.length - 1);
      selectedCol.value = clamped;
      dispatch({'type': 'selectTableColumn', 'tableId': overlay.tableId, 'col': clamped});
    }

    final currentRow = selectedRow.value.clamp(0, overlay.totalRows - 1);
    final currentCol = selectedCol.value.clamp(0, overlay.colWidths.length - 1);
    final selectedColLeft = tableColLeft(overlay, currentCol);
    final selectedColWidth = overlay.colWidths[currentCol];

    final selectedRowLocal = currentRow - overlay.startRowIndex;
    final isSelectedRowVisible =
        selectedRowLocal >= 0 &&
        selectedRowLocal < overlay.rowHeights.length &&
        selectedRowLocal < overlay.rowPositions.length;
    final selectedRowTop = isSelectedRowVisible ? tableRowTop(overlay, selectedRowLocal) : 0.0;
    final selectedRowHeight = isSelectedRowVisible ? overlay.rowHeights[selectedRowLocal] : 0.0;

    final colHandleLeft = tableClampDouble(
      renderBounds.x + selectedColLeft + (selectedColWidth - tableColumnHandleWidth) / 2,
      4,
      math.max(4, pageWidth - tableColumnHandleWidth - 4),
    );
    final colHandleTop = tableClampDouble(
      renderBounds.y - tableColumnHandleHeight - tableHandleGap,
      4,
      math.max(4, pageHeight - tableColumnHandleHeight - 4),
    );

    final rowHandleLeft = tableClampDouble(
      renderBounds.x - tableRowHandleWidth - tableHandleGap,
      4,
      math.max(4, pageWidth - tableRowHandleWidth - 4),
    );
    final rowHandleTop = isSelectedRowVisible
        ? tableClampDouble(
            renderBounds.y + selectedRowTop + (selectedRowHeight - tableRowHandleHeight) / 2,
            4,
            math.max(4, pageHeight - tableRowHandleHeight - 4),
          )
        : 0.0;

    final cellSelector = TableCellSelectorController(
      context: context,
      scope: scope,
      gesture: gesture,
      overlay: overlay,
      layout: layout,
      selection: selection,
      renderBounds: renderBounds,
      fallbackRow: currentRow,
      fallbackCol: currentCol,
      dragDraftState: cellSelectionDrag,
      cellHandleDragPosition: cellHandleDragPosition,
      viewWidth: viewWidth,
      viewHeight: viewHeight,
      dropPosition: dropPosition,
      globalToViewport: globalToViewport,
    );
    final resizeCol = cellSelector.rightEdgeCol;
    final cellHandleWidget = buildCellSelectionHandleWidget(cellSelector, selectionHandleColor);

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
      selectRow(currentRow);
      await _showRowActions(
        context,
        scope,
        overlay,
        currentRow,
        dispatchAndScrollAndFocus,
        onSelectedRowChanged: moveHandleRowTo,
      );
    }

    Future<void> openColumnMenu() async {
      selectCol(currentCol);
      await _showColumnActions(
        context,
        scope,
        overlay,
        currentCol,
        dispatchAndScrollAndFocus,
        onSelectedColChanged: moveHandleColTo,
      );
    }

    return Listener(
      behavior: HitTestBehavior.translucent,
      onPointerUp: (_) => cellSelector.endDrag(),
      onPointerCancel: (_) => cellSelector.endDrag(),
      child: Stack(
        clipBehavior: Clip.none,
        children: [
          ...buildCellSelectionOutlineWidgets(cellSelector, selectionHandleColor),
          TableColumnResizer(
            overlay: overlay,
            renderBounds: renderBounds,
            selectedCol: resizeCol,
            pageWidth: pageWidth,
            onCommit: (nextWidths) {
              dispatch({'type': 'setColumnWidths', 'tableId': overlay.tableId, 'colWidths': nextWidths});
              scope.controller.scrollIntoView();
            },
          ),
          Positioned(
            left: colHandleLeft,
            top: colHandleTop,
            child: SelectorHandleButton(
              width: tableColumnHandleWidth,
              height: tableColumnHandleHeight,
              icon: LucideLightIcons.ellipsis,
              onTap: openColumnMenu,
            ),
          ),
          if (isSelectedRowVisible)
            Positioned(
              left: rowHandleLeft,
              top: rowHandleTop,
              child: SelectorHandleButton(
                width: tableRowHandleWidth,
                height: tableRowHandleHeight,
                icon: LucideLightIcons.ellipsis_vertical,
                onTap: openRowMenu,
              ),
            ),
          ..._optionalWidget(cellHandleWidget),
        ],
      ),
    );
  }

  Future<void> _showRowActions(
    BuildContext context,
    ContentScope scope,
    TableOverlayInfo overlay,
    int selectedRow,
    OverlayDispatch dispatch, {
    required ValueChanged<int> onSelectedRowChanged,
  }) async {
    final isFirst = selectedRow == 0;
    final isLast = selectedRow == overlay.totalRows - 1;
    final isOnlyRow = overlay.totalRows <= 1;

    scope.inputController.dismissKeyboard();
    await Future<void>.delayed(tableOverlaySheetOpenDelay);
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
          BottomMenuItem(
            icon: LucideLightIcons.arrow_up_to_line,
            label: '위에 행 추가',
            onTap: () {
              onSelectedRowChanged(selectedRow + 1);
              dispatch({'type': 'addTableRow', 'tableId': overlay.tableId, 'row': selectedRow, 'before': true});
            },
          ),
          BottomMenuItem(
            icon: LucideLightIcons.arrow_down_to_line,
            label: '아래에 행 추가',
            onTap: () {
              onSelectedRowChanged(selectedRow);
              dispatch({'type': 'addTableRow', 'tableId': overlay.tableId, 'row': selectedRow, 'before': false});
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
    OverlayDispatch dispatch, {
    required ValueChanged<int> onSelectedColChanged,
  }) async {
    final isFirst = selectedCol == 0;
    final isLast = selectedCol == overlay.colWidths.length - 1;
    final isOnlyColumn = overlay.colWidths.length <= 1;

    scope.inputController.dismissKeyboard();
    await Future<void>.delayed(tableOverlaySheetOpenDelay);
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
          BottomMenuItem(
            icon: LucideLightIcons.arrow_left_to_line,
            label: '왼쪽에 열 추가',
            onTap: () {
              onSelectedColChanged(selectedCol + 1);
              dispatch({'type': 'addTableColumn', 'tableId': overlay.tableId, 'col': selectedCol, 'before': true});
            },
          ),
          BottomMenuItem(
            icon: LucideLightIcons.arrow_right_to_line,
            label: '오른쪽에 열 추가',
            onTap: () {
              onSelectedColChanged(selectedCol);
              dispatch({'type': 'addTableColumn', 'tableId': overlay.tableId, 'col': selectedCol, 'before': false});
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
