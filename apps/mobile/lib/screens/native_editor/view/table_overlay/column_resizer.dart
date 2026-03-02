import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/screens/native_editor/table/models.dart';
import 'package:typie/screens/native_editor/view/interaction/controller.dart';
import 'package:typie/screens/native_editor/view/interaction/mode.dart';

import 'constants.dart';
import 'geometry.dart';

const tableBorderWidth = 1.0;
const tableResizeLimitEpsilon = 0.5;

class TableColumnResizer extends HookWidget {
  const TableColumnResizer({
    required this.interactionController,
    required this.overlay,
    required this.renderBounds,
    required this.selectedCol,
    required this.pageWidth,
    required this.onCommitColumnWidths,
    required this.onCommitTableWidth,
    super.key,
  });

  final EditorInteractionController interactionController;
  final TableOverlayInfo overlay;
  final TableOverlayBounds renderBounds;
  final int selectedCol;
  final double pageWidth;
  final ValueChanged<List<double>> onCommitColumnWidths;
  final ValueChanged<double> onCommitTableWidth;

  @override
  Widget build(BuildContext context) {
    final resizeDraft = useState<ColumnResizeDraft?>(null);
    final activeResizePointer = useState<int?>(null);
    final maxResizableCol = overlay.colWidthsAsPx.length - 1;
    final contentWidth = overlay.contentWidth;

    useEffect(() {
      final draft = resizeDraft.value;
      if (draft == null) {
        return null;
      }
      final shouldReset =
          draft.tableId != overlay.tableId ||
          overlay.colWidthsAsPx.isEmpty ||
          draft.colIndex > maxResizableCol ||
          draft.initialWidths.length != overlay.colWidthsAsPx.length;
      if (shouldReset) {
        interactionController.endAuxiliaryGesture();
        resizeDraft.value = null;
        activeResizePointer.value = null;
      }
      return null;
    }, [maxResizableCol, resizeDraft.value, overlay.tableId, overlay.colWidthsAsPx.length]);

    useEffect(() => interactionController.endAuxiliaryGesture, const []);

    if (overlay.colWidthsAsPx.isEmpty) {
      return const SizedBox.shrink();
    }

    final draft = resizeDraft.value;
    final isResizing = draft != null;
    final resizeCol = (isResizing ? draft.colIndex : selectedCol).clamp(0, maxResizableCol);
    final resizeDelta = isResizing
        ? (draft.colIndex == maxResizableCol
              ? clampTableResizeDelta(overlay, renderBounds.width, draft.deltaX, contentWidth)
              : clampColumnResizeDelta(draft.initialWidths, draft.colIndex, draft.deltaX))
        : 0.0;
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
      interactionController.startAuxiliaryGesture(AuxiliaryGestureKind.tableColumnResize);
      activeResizePointer.value = event.pointer;
      resizeDraft.value = ColumnResizeDraft(
        tableId: overlay.tableId,
        colIndex: selectedCol.clamp(0, maxResizableCol),
        startX: event.position.dx,
        initialWidths: List<double>.from(overlay.colWidthsAsPx),
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
      interactionController.updateAuxiliaryGesture(AuxiliaryGestureKind.tableColumnResize);
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
      interactionController.endAuxiliaryGesture();
      resizeDraft.value = null;

      if (current.colIndex == maxResizableCol) {
        if (contentWidth <= 0) {
          return;
        }
        final clampedDelta = clampTableResizeDelta(overlay, renderBounds.width, current.deltaX, contentWidth);
        if (clampedDelta.abs() <= 0.01) {
          return;
        }
        final currentTableWidth = renderBounds.width;
        final nextTableWidth = currentTableWidth + clampedDelta;
        onCommitTableWidth(nextTableWidth);
        return;
      }

      final nextWidths = applyColumnResizeDelta(current.initialWidths, current.colIndex, current.deltaX);
      if (!hasWidthChange(current.initialWidths, nextWidths)) {
        return;
      }
      onCommitColumnWidths(toRatioWidths(nextWidths));
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

double minTableWidthForColumns(int colCount) {
  if (colCount <= 0) {
    return 0;
  }
  return tableMinColumnWidth * colCount + tableBorderWidth * (colCount + 1);
}

double clampColumnResizeDelta(List<double> widths, int colIndex, double deltaX) {
  if (widths.isEmpty || colIndex < 0 || colIndex >= widths.length - 1) {
    return 0;
  }

  final minDelta = tableMinColumnWidth - widths[colIndex];
  final maxDelta = widths[colIndex + 1] - tableMinColumnWidth;
  return tableClampDouble(deltaX, minDelta, maxDelta);
}

double clampTableResizeDelta(TableOverlayInfo overlay, double currentTableWidth, double deltaX, double contentWidth) {
  final colCount = overlay.colWidthsAsPx.length;
  if (colCount <= 0 || contentWidth <= 0) {
    return 0;
  }

  final minTableWidth = math.max(minTableWidthForColumns(colCount), overlay.minProportionWidth);
  final maxTableWidth = math.max(
    minTableWidth,
    overlay.maxProportionWidth > 0 ? overlay.maxProportionWidth : contentWidth,
  );
  if (minTableWidth > maxTableWidth) {
    return 0;
  }

  final effectiveMinTableWidth = currentTableWidth <= minTableWidth + tableResizeLimitEpsilon
      ? currentTableWidth
      : minTableWidth;
  final minDelta = effectiveMinTableWidth - currentTableWidth;
  final maxDelta = maxTableWidth - currentTableWidth;
  return tableClampDouble(deltaX, minDelta, maxDelta);
}

List<double> applyColumnResizeDelta(List<double> initialWidths, int colIndex, double deltaX) {
  if (initialWidths.isEmpty || colIndex < 0 || colIndex >= initialWidths.length - 1) {
    return initialWidths;
  }

  final next = List<double>.from(initialWidths);
  final clampedDelta = clampColumnResizeDelta(initialWidths, colIndex, deltaX);

  next[colIndex] = next[colIndex] + clampedDelta;
  next[colIndex + 1] = next[colIndex + 1] - clampedDelta;
  return next;
}

List<double> toRatioWidths(List<double> widths) {
  if (widths.isEmpty) {
    return const [];
  }

  final safe = widths.map((width) => width.isFinite && width > 0 ? width : 0).toList(growable: false);
  final total = safe.fold<double>(0, (sum, width) => sum + width);
  if (total <= 0) {
    final fallback = 1 / widths.length;
    return List<double>.filled(widths.length, fallback);
  }

  return safe.map((width) => width / total).toList(growable: false);
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
