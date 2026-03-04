import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/table/models.dart';
import 'package:typie/screens/native_editor/view/interaction/controller.dart';
import 'package:typie/screens/native_editor/view/scope.dart';

import 'constants.dart';
import 'geometry.dart';
import 'handles.dart';

Widget? buildCellSelectionOutlineWidget(TableCellSelectorController controller, Color color) {
  if (!controller.shouldShow || controller.visual == null) {
    return null;
  }
  final visual = controller.visual!;

  return Positioned(
    left: visual.rect.left,
    top: visual.rect.top,
    width: visual.rect.width,
    height: visual.rect.height,
    child: IgnorePointer(
      child: Container(
        decoration: BoxDecoration(
          border: Border.all(color: color, width: tableCellSelectionBorderWidth),
        ),
      ),
    ),
  );
}

Widget? buildCellSelectionHandleWidget(TableCellSelectorController controller, Color color) {
  if (!controller.shouldShow || controller.visual?.handleCenter == null) {
    return null;
  }
  final handleCenter = controller.visual!.handleCenter!;
  return Positioned(
    left: handleCenter.dx - tableCellSelectionHandleTouchSize / 2,
    top: handleCenter.dy - tableCellSelectionHandleTouchSize / 2,
    child: CellSelectionHandle(
      color: color,
      onPanDown: controller.beginDrag,
      onPanStart: controller.startDrag,
      onPanUpdate: controller.updateDrag,
      onPanEnd: controller.endDragFromPanEnd,
      onPanCancel: controller.endDrag,
    ),
  );
}

class TableSelectionRange {
  TableSelectionRange({required this.anchor, required this.head})
    : rowStart = math.min(anchor.row, head.row),
      rowEnd = math.max(anchor.row, head.row),
      colStart = math.min(anchor.col, head.col),
      colEnd = math.max(anchor.col, head.col);

  factory TableSelectionRange.single({required int row, required int col}) {
    final cell = TableCellIndex(row: row, col: col);
    return TableSelectionRange(anchor: cell, head: cell);
  }

  final TableCellIndex anchor;
  final TableCellIndex head;
  final int rowStart;
  final int rowEnd;
  final int colStart;
  final int colEnd;

  bool get isSingleCell => rowStart == rowEnd && colStart == colEnd;

  TableSelectionRange clamp({required int maxRow, required int maxCol}) {
    return TableSelectionRange(
      anchor: anchor.clamp(maxRow: maxRow, maxCol: maxCol),
      head: head.clamp(maxRow: maxRow, maxCol: maxCol),
    );
  }
}

class CellSelectionDragDraft {
  const CellSelectionDragDraft({required this.tableId, required this.anchor, required this.head});

  final String tableId;
  final TableCellIndex anchor;
  final TableCellIndex head;

  CellSelectionDragDraft copyWith({TableCellIndex? head}) {
    return CellSelectionDragDraft(tableId: tableId, anchor: anchor, head: head ?? this.head);
  }

  TableSelectionRange get range => TableSelectionRange(anchor: anchor, head: head);
}

class TableSelectionVisual {
  const TableSelectionVisual({required this.rect, required this.handleCenter});

  final Rect rect;
  final Offset? handleCenter;
}

class TableCellSelectorState {
  const TableCellSelectorState({required this.shouldShow, required this.range, required this.visual});

  final bool shouldShow;
  final TableSelectionRange range;
  final TableSelectionVisual? visual;
}

class TableCellSelectorController {
  TableCellSelectorController({
    required this.context,
    required this.scope,
    required this.interactionController,
    required this.overlay,
    required this.layout,
    required this.pages,
    required this.selection,
    required this.renderBounds,
    required this.fallbackRow,
    required this.fallbackCol,
    required this.dragDraftState,
    required this.cellHandleDragPosition,
    required this.viewWidth,
    required this.viewHeight,
    required this.dropPosition,
  }) : state = _resolveTableCellSelectorState(
         overlay: overlay,
         selection: selection,
         layout: layout,
         pages: pages,
         renderBounds: renderBounds,
         dragDraft: dragDraftState.value,
         fallbackRow: fallbackRow,
         fallbackCol: fallbackCol,
       );

  final BuildContext context;
  final ContentScope scope;
  final EditorInteractionController interactionController;
  final TableOverlayInfo overlay;
  final Layout? layout;
  final List<PageSize> pages;
  final EditorSelection? selection;
  final TableOverlayBounds renderBounds;
  final int fallbackRow;
  final int fallbackCol;
  final ValueNotifier<CellSelectionDragDraft?> dragDraftState;
  final ValueNotifier<Offset?> cellHandleDragPosition;
  final double viewWidth;
  final double viewHeight;
  final ValueNotifier<Offset?> dropPosition;
  final TableCellSelectorState state;

  bool get shouldShow => state.shouldShow;
  TableSelectionVisual? get visual => state.visual;
  int get rightEdgeCol => state.range.colEnd;

  void beginDrag(DragDownDetails details) {
    if (!context.mounted) {
      return;
    }
    interactionController.beginTableCellHandleDragDown(details);
  }

  void startDrag(DragStartDetails details) {
    if (!context.mounted) {
      return;
    }
    if (!shouldShow || visual?.handleCenter == null) {
      return;
    }
    final minVisibleRow = overlay.startRowIndex;
    final maxVisibleRow = overlay.startRowIndex + overlay.rowHeights.length - 1;
    if (maxVisibleRow < minVisibleRow) {
      return;
    }

    TableCellIndex clampToVisible(TableCellIndex cell) {
      return TableCellIndex(
        row: cell.row.clamp(minVisibleRow, maxVisibleRow),
        col: cell.col.clamp(0, overlay.colWidthsAsPx.length - 1),
      );
    }

    final anchor = clampToVisible(state.range.anchor);
    final head = clampToVisible(state.range.head);
    dragDraftState.value = CellSelectionDragDraft(tableId: overlay.tableId, anchor: anchor, head: head);

    final viewportPosition = interactionController.viewportPositionFromGlobal(details.globalPosition);
    SelectionHandleInfo? anchorHandle;
    if (layout != null) {
      final anchorPoint = tableCellCenterPagePoint(overlay: overlay, layout: layout!, pages: pages, cell: anchor);
      if (anchorPoint != null) {
        anchorHandle = SelectionEndpointBounds(
          pageIdx: anchorPoint.pageIdx,
          x: anchorPoint.x,
          y: anchorPoint.y,
          width: 0,
          height: 0,
        );
      }
    }

    final started = interactionController.startTableCellHandleDrag(
      anchorHandle: anchorHandle,
      viewportPosition: viewportPosition,
      cellHandleDragPosition: cellHandleDragPosition,
    );
    if (!started) {
      dragDraftState.value = null;
    }
  }

  void updateDrag(DragUpdateDetails details) {
    if (!context.mounted) {
      return;
    }
    final draft = dragDraftState.value;
    if (draft == null) {
      return;
    }
    _updateHeadFromGlobalPosition(details.globalPosition);

    final viewportPosition = interactionController.viewportPositionFromGlobal(details.globalPosition);
    if (viewportPosition == null) {
      return;
    }
    final updated = interactionController.updateTableCellHandleDrag(
      viewportPosition: viewportPosition,
      cellHandleDragPosition: cellHandleDragPosition,
      tableDropPosition: dropPosition,
      viewWidth: viewWidth,
      viewHeight: viewHeight,
    );
    if (!updated) {
      return;
    }
  }

  void endDragFromPanEnd(DragEndDetails _) => endDrag();

  void endDrag() {
    interactionController.endTableCellHandleDrag(cellHandleDragPosition: cellHandleDragPosition);
    if (!context.mounted) {
      return;
    }
    dragDraftState.value = null;
  }

  void _updateHeadFromGlobalPosition(Offset globalPosition) {
    final draft = dragDraftState.value;
    if (draft == null) {
      return;
    }

    final renderBox = context.findRenderObject() as RenderBox?;
    if (renderBox == null) {
      return;
    }

    final local = renderBox.globalToLocal(globalPosition);
    final nextCell = tableCellAtOverlayOffset(
      overlay: overlay,
      localX: local.dx - renderBounds.x,
      localY: local.dy - renderBounds.y,
    );
    final nextHead = nextCell.clamp(maxRow: overlay.totalRows - 1, maxCol: overlay.colWidthsAsPx.length - 1);

    if (nextHead.row == draft.head.row && nextHead.col == draft.head.col) {
      return;
    }

    final nextDraft = draft.copyWith(head: nextHead);
    dragDraftState.value = nextDraft;
    _dispatchRange(nextDraft);
  }

  void _dispatchRange(CellSelectionDragDraft draft) {
    final anchor = draft.anchor;
    final head = draft.head;

    if (anchor.row == head.row && anchor.col == head.col) {
      scope.controller.dispatch({'type': 'collapseSelection', 'toAnchor': true});
      scope.inputController.requestFocus();
      return;
    }

    if (layout == null) {
      return;
    }
    final anchorPoint = tableCellCenterPagePoint(overlay: overlay, layout: layout!, pages: pages, cell: anchor);
    final headPoint = tableCellCenterPagePoint(overlay: overlay, layout: layout!, pages: pages, cell: head);
    if (anchorPoint == null || headPoint == null) {
      return;
    }

    scope.controller.dispatch({
      'type': 'extendSelectionTo',
      'anchorPageIdx': anchorPoint.pageIdx,
      'anchorX': anchorPoint.x,
      'anchorY': anchorPoint.y,
      'headPageIdx': headPoint.pageIdx,
      'headX': headPoint.x,
      'headY': headPoint.y,
    });
    scope.inputController.requestFocus();
  }
}

void resetCellSelectionDragIfTableChanged({
  required ValueNotifier<CellSelectionDragDraft?> draftState,
  required String tableId,
}) {
  final draft = draftState.value;
  if (draft == null || draft.tableId == tableId) {
    return;
  }
  draftState.value = null;
}

TableCellSelectorState _resolveTableCellSelectorState({
  required TableOverlayInfo overlay,
  required EditorSelection? selection,
  required Layout? layout,
  required List<PageSize> pages,
  required TableOverlayBounds renderBounds,
  required CellSelectionDragDraft? dragDraft,
  required int fallbackRow,
  required int fallbackCol,
}) {
  final stateRange = _tableSelectionRangeFromSelection(
    overlay: overlay,
    selection: selection,
    layout: layout,
    pages: pages,
  );
  final fallbackRange = TableSelectionRange.single(row: fallbackRow, col: fallbackCol);
  final activeRange = (dragDraft?.range ?? stateRange ?? fallbackRange).clamp(
    maxRow: overlay.totalRows - 1,
    maxCol: overlay.colWidthsAsPx.length - 1,
  );
  final visual = _tableSelectionVisual(overlay: overlay, renderBounds: renderBounds, range: activeRange);
  final shouldShow = _shouldShowCellSelector(overlay: overlay, selection: selection, stateRange: stateRange);

  return TableCellSelectorState(shouldShow: shouldShow, range: activeRange, visual: visual);
}

bool _shouldShowCellSelector({
  required TableOverlayInfo overlay,
  required EditorSelection? selection,
  required TableSelectionRange? stateRange,
}) {
  if (overlay.showCellSelector) {
    return true;
  }
  if (selection?.collapsed ?? false) {
    return true;
  }
  return stateRange?.isSingleCell ?? false;
}

TableSelectionRange? _tableSelectionRangeFromSelection({
  required TableOverlayInfo overlay,
  required EditorSelection? selection,
  required Layout? layout,
  required List<PageSize> pages,
}) {
  if (selection == null || layout == null) {
    return null;
  }
  if (overlay.colWidthsAsPx.isEmpty || overlay.rowHeights.isEmpty) {
    return null;
  }

  final anchor = tableCellFromSelectionEndpoint(overlay, selection.anchorBounds, layout, pages);
  final head = tableCellFromSelectionEndpoint(overlay, selection.headBounds, layout, pages);
  if (anchor == null || head == null) {
    return null;
  }

  return TableSelectionRange(
    anchor: anchor,
    head: head,
  ).clamp(maxRow: overlay.totalRows - 1, maxCol: overlay.colWidthsAsPx.length - 1);
}

TableSelectionVisual? _tableSelectionVisual({
  required TableOverlayInfo overlay,
  required TableOverlayBounds renderBounds,
  required TableSelectionRange range,
}) {
  if (overlay.colWidthsAsPx.isEmpty || overlay.rowHeights.isEmpty || overlay.rowPositions.isEmpty) {
    return null;
  }

  final minVisibleRow = overlay.startRowIndex;
  final maxVisibleRow = overlay.startRowIndex + overlay.rowHeights.length - 1;
  if (maxVisibleRow < minVisibleRow) {
    return null;
  }

  final visibleRowStart = math.max(range.rowStart, minVisibleRow);
  final visibleRowEnd = math.min(range.rowEnd, maxVisibleRow);
  if (visibleRowStart > visibleRowEnd) {
    return null;
  }

  final localRowStart = visibleRowStart - overlay.startRowIndex;
  final localRowEnd = visibleRowEnd - overlay.startRowIndex;

  final colStart = range.colStart.clamp(0, overlay.colWidthsAsPx.length - 1);
  final colEnd = range.colEnd.clamp(0, overlay.colWidthsAsPx.length - 1);
  final left = renderBounds.x + tableColLeft(overlay, colStart);
  final right = renderBounds.x + tableColRight(overlay, colEnd);
  final top = renderBounds.y + tableRowTop(overlay, localRowStart);
  final bottom = renderBounds.y + tableRowBottom(overlay, localRowEnd);

  if (right <= left || bottom <= top) {
    return null;
  }

  Offset? handleCenter;
  final handleRow = range.head.row.clamp(0, overlay.totalRows - 1);
  final handleCol = range.head.col.clamp(0, overlay.colWidthsAsPx.length - 1);
  final handleLocalRow = handleRow - overlay.startRowIndex;
  if (handleLocalRow >= 0 && handleLocalRow < overlay.rowPositions.length) {
    final handleX = range.head.col >= range.anchor.col
        ? tableColRight(overlay, handleCol)
        : tableColLeft(overlay, handleCol);
    final handleY = range.head.row >= range.anchor.row
        ? tableRowBottom(overlay, handleLocalRow)
        : tableRowTop(overlay, handleLocalRow);
    handleCenter = Offset(renderBounds.x + handleX, renderBounds.y + handleY);
  }

  return TableSelectionVisual(rect: Rect.fromLTRB(left, top, right, bottom), handleCenter: handleCenter);
}
