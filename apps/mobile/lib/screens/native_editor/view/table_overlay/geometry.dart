import 'dart:math' as math;

import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/table/models.dart';

class TableCellIndex {
  const TableCellIndex({required this.row, required this.col});

  final int row;
  final int col;

  TableCellIndex clamp({required int maxRow, required int maxCol}) {
    return TableCellIndex(row: row.clamp(0, maxRow), col: col.clamp(0, maxCol));
  }
}

class PagePoint {
  const PagePoint({required this.pageIdx, required this.x, required this.y});

  final int pageIdx;
  final double x;
  final double y;
}

int tableDefaultRow(TableOverlayInfo overlay) {
  return overlay.startRowIndex.clamp(0, overlay.totalRows - 1);
}

double tableClampDouble(double value, double min, double max) {
  if (max < min) {
    return min;
  }
  return value.clamp(min, max);
}

int tableIndexForOffset(List<double> positions, double offset) {
  for (var i = 0; i < positions.length; i++) {
    if (offset < positions[i]) {
      return i;
    }
  }
  return positions.length - 1;
}

double tableColLeft(TableOverlayInfo overlay, int colIndex) {
  if (colIndex <= 0) {
    return 0;
  }
  return overlay.colPositions[colIndex - 1];
}

double tableColRight(TableOverlayInfo overlay, int colIndex) {
  if (colIndex < 0) {
    return 0;
  }
  if (colIndex >= overlay.colPositions.length) {
    return overlay.bounds.width;
  }
  return overlay.colPositions[colIndex];
}

double tableRowTop(TableOverlayInfo overlay, int localRowIndex) {
  final rowInset = tableRowContentTopInset(overlay);
  if (localRowIndex <= 0) {
    return rowInset;
  }
  return rowInset + overlay.rowPositions[localRowIndex - 1];
}

double tableRowBottom(TableOverlayInfo overlay, int localRowIndex) {
  final rowInset = tableRowContentTopInset(overlay);
  if (localRowIndex < 0) {
    return rowInset;
  }
  if (localRowIndex >= overlay.rowPositions.length) {
    return rowInset + overlay.rowPositions.last;
  }
  return rowInset + overlay.rowPositions[localRowIndex];
}

double tableRowContentTopInset(TableOverlayInfo overlay) {
  return overlay.startRowIndex == 0 ? 1.0 : 0.0;
}

double tablePageTopOffset(Layout layout, List<PageSize> pages, int pageIdx) {
  if (pages.isEmpty) {
    return 0;
  }

  final isPaginated = layout is PaginatedLayout;
  final clamped = pageIdx.clamp(0, pages.length - 1);
  var top = 0.0;
  for (var i = 0; i < clamped; i++) {
    top += pages[i].height;
    if (isPaginated && i < pages.length - 1) {
      top += 24.0;
    }
  }
  return top;
}

int tablePageIndexForGlobalY(Layout layout, List<PageSize> pages, double globalY) {
  if (pages.isEmpty) {
    return 0;
  }

  final isPaginated = layout is PaginatedLayout;
  var top = 0.0;
  for (var i = 0; i < pages.length; i++) {
    final bottom = top + pages[i].height;
    if (globalY <= bottom || i == pages.length - 1) {
      return i;
    }
    if (isPaginated && i < pages.length - 1) {
      top = bottom + 24.0;
    } else {
      top = bottom;
    }
  }
  return pages.length - 1;
}

TableCellIndex tableCellAtOverlayOffset({
  required TableOverlayInfo overlay,
  required double localX,
  required double localY,
}) {
  if (overlay.colPositions.isEmpty || overlay.rowPositions.isEmpty) {
    return TableCellIndex(row: tableDefaultRow(overlay), col: 0);
  }
  final maxX = math.max(0, overlay.bounds.width - 0.001).toDouble();
  final maxY = math.max(0, overlay.rowPositions.last - 0.001).toDouble();
  final rowInset = tableRowContentTopInset(overlay);
  final x = tableClampDouble(localX, 0, maxX);
  final y = tableClampDouble(localY - rowInset, 0, maxY);
  final localRow = tableIndexForOffset(overlay.rowPositions, y);
  final localCol = tableIndexForOffset(overlay.colPositions, x);

  return TableCellIndex(
    row: (overlay.startRowIndex + localRow).clamp(0, overlay.totalRows - 1),
    col: localCol.clamp(0, overlay.colWidths.length - 1),
  );
}

PagePoint? tableCellCenterPagePoint({
  required TableOverlayInfo overlay,
  required Layout layout,
  required List<PageSize> pages,
  required TableCellIndex cell,
}) {
  final localRow = cell.row - overlay.startRowIndex;
  if (localRow < 0 || localRow >= overlay.rowHeights.length) {
    return null;
  }
  if (cell.col < 0 || cell.col >= overlay.colWidths.length) {
    return null;
  }

  final x = overlay.bounds.x + tableColLeft(overlay, cell.col) + overlay.colWidths[cell.col] * 0.5;
  final localY = tableRowTop(overlay, localRow) + overlay.rowHeights[localRow] * 0.5;

  if (layout is PaginatedLayout) {
    return PagePoint(pageIdx: overlay.pageIdx, x: x, y: overlay.bounds.y + localY);
  }

  final globalY = tablePageTopOffset(layout, pages, overlay.pageIdx) + overlay.bounds.y + localY;
  final pageIdx = tablePageIndexForGlobalY(layout, pages, globalY);
  return PagePoint(pageIdx: pageIdx, x: x, y: globalY - tablePageTopOffset(layout, pages, pageIdx));
}

TableCellIndex? tableCellFromSelectionEndpoint(
  TableOverlayInfo overlay,
  SelectionEndpointBounds? endpoint,
  Layout layout,
  List<PageSize> pages,
) {
  if (endpoint == null) {
    return null;
  }
  return tableCellFromPagePoint(
    overlay: overlay,
    layout: layout,
    pages: pages,
    pageIdx: endpoint.pageIdx,
    x: endpoint.x + endpoint.width * 0.5,
    y: endpoint.y + endpoint.height * 0.5,
  );
}

TableCellIndex? tableCellFromPagePoint({
  required TableOverlayInfo overlay,
  required Layout layout,
  required List<PageSize> pages,
  required int pageIdx,
  required double x,
  required double y,
}) {
  if (overlay.colPositions.isEmpty || overlay.rowPositions.isEmpty) {
    return null;
  }

  final localX = x - overlay.bounds.x;
  final isPaginated = layout is PaginatedLayout;
  final localY = isPaginated
      ? (() {
          if (pageIdx != overlay.pageIdx) {
            return double.nan;
          }
          return y - overlay.bounds.y;
        })()
      : (() {
          final overlayTop = tablePageTopOffset(layout, pages, overlay.pageIdx) + overlay.bounds.y;
          final pointGlobalY = tablePageTopOffset(layout, pages, pageIdx) + y;
          return pointGlobalY - overlayTop;
        })();

  if (localY.isNaN) {
    return null;
  }
  if (localX < 0 || localY < 0 || localX > overlay.bounds.width || localY > overlay.bounds.height) {
    return null;
  }

  final maxY = math.max(0, overlay.rowPositions.last - 0.001).toDouble();
  final rowInset = tableRowContentTopInset(overlay);
  final adjustedY = tableClampDouble(localY - rowInset, 0, maxY);
  final localRow = tableIndexForOffset(overlay.rowPositions, adjustedY);
  final localCol = tableIndexForOffset(overlay.colPositions, localX);

  return TableCellIndex(
    row: (overlay.startRowIndex + localRow).clamp(0, overlay.totalRows - 1),
    col: localCol.clamp(0, overlay.colWidths.length - 1),
  );
}

TableCellIndex? tableFocusedCellFromCursor(
  TableOverlayInfo overlay,
  CursorInfo? cursor,
  Layout? layout,
  List<PageSize> pages,
) {
  if (cursor == null || !cursor.visible) {
    return null;
  }
  if (layout == null) {
    return null;
  }
  return tableCellFromPagePoint(
    overlay: overlay,
    layout: layout,
    pages: pages,
    pageIdx: cursor.pageIdx,
    x: cursor.x,
    y: cursor.y + cursor.height * 0.5,
  );
}
