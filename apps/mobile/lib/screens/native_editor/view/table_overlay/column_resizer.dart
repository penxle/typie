import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/screens/native_editor/table/models.dart';

import 'constants.dart';
import 'geometry.dart';

class TableColumnResizer extends HookWidget {
  const TableColumnResizer({
    required this.overlay,
    required this.renderBounds,
    required this.selectedCol,
    required this.pageWidth,
    required this.onCommit,
    super.key,
  });

  final TableOverlayInfo overlay;
  final TableOverlayBounds renderBounds;
  final int selectedCol;
  final double pageWidth;
  final ValueChanged<List<double>> onCommit;

  @override
  Widget build(BuildContext context) {
    final resizeDraft = useState<ColumnResizeDraft?>(null);
    final activeResizePointer = useState<int?>(null);

    useEffect(() {
      final draft = resizeDraft.value;
      if (draft == null) {
        return null;
      }
      final shouldReset =
          draft.tableId != overlay.tableId ||
          draft.colIndex >= overlay.colWidths.length ||
          draft.initialWidths.length != overlay.colWidths.length;
      if (shouldReset) {
        resizeDraft.value = null;
        activeResizePointer.value = null;
      }
      return null;
    }, [resizeDraft.value, overlay.tableId, overlay.colWidths.length]);

    final draft = resizeDraft.value;
    final isResizing = draft != null;
    final resizeCol = (isResizing ? draft.colIndex : selectedCol).clamp(0, overlay.colWidths.length - 1);
    final resizeDelta = isResizing ? clampColumnResizeDelta(draft.initialWidths, draft.colIndex, draft.deltaX) : 0.0;
    final resizeHandleCenterX = renderBounds.x + tableColRight(overlay, resizeCol) + resizeDelta;
    final resizeHandleLeft = tableClampDouble(
      resizeHandleCenterX - tableColumnResizeTouchWidth / 2,
      0,
      math.max(0, pageWidth - tableColumnResizeTouchWidth),
    );

    void beginColumnResize(PointerDownEvent event) {
      if (activeResizePointer.value != null) {
        return;
      }
      activeResizePointer.value = event.pointer;
      resizeDraft.value = ColumnResizeDraft(
        tableId: overlay.tableId,
        colIndex: selectedCol,
        startX: event.position.dx,
        initialWidths: List<double>.from(overlay.colWidths),
        deltaX: 0,
      );
    }

    void updateColumnResize(PointerMoveEvent event) {
      if (event.pointer != activeResizePointer.value) {
        return;
      }
      final current = resizeDraft.value;
      if (current == null) {
        return;
      }
      resizeDraft.value = current.copyWith(deltaX: event.position.dx - current.startX);
    }

    void endColumnResize(PointerEvent event) {
      if (event.pointer != activeResizePointer.value) {
        return;
      }
      activeResizePointer.value = null;
      final current = resizeDraft.value;
      if (current == null) {
        return;
      }
      final nextWidths = applyColumnResizeDelta(current.initialWidths, current.colIndex, current.deltaX);
      resizeDraft.value = null;
      if (!hasWidthChange(current.initialWidths, nextWidths)) {
        return;
      }
      onCommit(nextWidths);
    }

    return Positioned(
      left: resizeHandleLeft,
      top: renderBounds.y,
      child: RawGestureDetector(
        behavior: HitTestBehavior.opaque,
        gestures: {
          EagerGestureRecognizer: GestureRecognizerFactoryWithHandlers<EagerGestureRecognizer>(
            EagerGestureRecognizer.new,
            (EagerGestureRecognizer instance) {},
          ),
        },
        child: Listener(
          behavior: HitTestBehavior.opaque,
          onPointerDown: beginColumnResize,
          onPointerMove: updateColumnResize,
          onPointerUp: endColumnResize,
          onPointerCancel: endColumnResize,
          child: SizedBox(
            width: tableColumnResizeTouchWidth,
            height: renderBounds.height,
            child: Align(
              alignment: Alignment.topCenter,
              child: Container(
                margin: const EdgeInsets.only(top: 2),
                width: tableColumnResizeVisualWidth,
                height: math.max(0, renderBounds.height - 4),
                decoration: BoxDecoration(
                  color: isResizing ? context.colors.accentBrand : context.colors.accentBrand.withValues(alpha: 0.35),
                  borderRadius: BorderRadius.circular(tableColumnResizeVisualWidth),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class ColumnResizeDraft {
  const ColumnResizeDraft({
    required this.tableId,
    required this.colIndex,
    required this.startX,
    required this.initialWidths,
    required this.deltaX,
  });

  final String tableId;
  final int colIndex;
  final double startX;
  final List<double> initialWidths;
  final double deltaX;

  ColumnResizeDraft copyWith({double? deltaX}) {
    return ColumnResizeDraft(
      tableId: tableId,
      colIndex: colIndex,
      startX: startX,
      initialWidths: initialWidths,
      deltaX: deltaX ?? this.deltaX,
    );
  }
}

double clampColumnResizeDelta(List<double> widths, int colIndex, double deltaX) {
  if (widths.isEmpty || colIndex < 0 || colIndex >= widths.length) {
    return 0;
  }

  final minDelta = tableMinColumnWidth - widths[colIndex];
  if (colIndex == widths.length - 1) {
    return math.max(minDelta, deltaX);
  }

  final maxDelta = widths[colIndex + 1] - tableMinColumnWidth;
  return tableClampDouble(deltaX, minDelta, maxDelta);
}

List<double> applyColumnResizeDelta(List<double> initialWidths, int colIndex, double deltaX) {
  if (initialWidths.isEmpty || colIndex < 0 || colIndex >= initialWidths.length) {
    return initialWidths;
  }

  final next = List<double>.from(initialWidths);
  final clampedDelta = clampColumnResizeDelta(initialWidths, colIndex, deltaX);

  if (colIndex == next.length - 1) {
    next[colIndex] = next[colIndex] + clampedDelta;
    return next;
  }

  next[colIndex] = next[colIndex] + clampedDelta;
  next[colIndex + 1] = next[colIndex + 1] - clampedDelta;
  return next;
}

bool hasWidthChange(List<double> before, List<double> after) {
  if (before.length != after.length) {
    return true;
  }
  for (var i = 0; i < before.length; i++) {
    if ((before[i] - after[i]).abs() > 0.01) {
      return true;
    }
  }
  return false;
}
