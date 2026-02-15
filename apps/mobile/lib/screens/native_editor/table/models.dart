class TableOverlayBounds {
  const TableOverlayBounds({required this.x, required this.y, required this.width, required this.height});

  final double x;
  final double y;
  final double width;
  final double height;
}

class TableOverlayInfo {
  const TableOverlayInfo({
    required this.pageIdx,
    required this.tableId,
    required this.bounds,
    required this.borderStyle,
    required this.align,
    required this.startRowIndex,
    required this.totalRows,
    required this.isFocused,
    required this.showCellSelector,
    required this.colWidths,
    required this.colPositions,
    required this.rowHeights,
    required this.rowPositions,
  });

  final int pageIdx;
  final String tableId;
  final TableOverlayBounds bounds;
  final String borderStyle;
  final String align;
  final int startRowIndex;
  final int totalRows;
  final bool isFocused;
  final bool showCellSelector;
  final List<double> colWidths;
  final List<double> colPositions;
  final List<double> rowHeights;
  final List<double> rowPositions;
}
