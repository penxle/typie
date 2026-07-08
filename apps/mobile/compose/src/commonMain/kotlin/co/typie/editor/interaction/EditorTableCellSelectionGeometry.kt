package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import co.typie.editor.Editor
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.TableOverlay
import kotlin.math.max
import kotlin.math.min

internal const val EditorTableCellSelectionBorderWidthDp = 2f
internal const val EditorTableCellSelectionHandleRadiusDp = 7f
internal const val EditorTableCellSelectionHandleTouchTargetDp = 36f

internal data class EditorTableCellIndex(val row: Int, val col: Int)

internal data class EditorTableCellSelectionRange(
  val anchor: EditorTableCellIndex,
  val head: EditorTableCellIndex,
) {
  val rowStart: Int = min(anchor.row, head.row)
  val rowEnd: Int = max(anchor.row, head.row)
  val colStart: Int = min(anchor.col, head.col)
  val colEnd: Int = max(anchor.col, head.col)
}

internal data class EditorTableCellSelectionGeometry(val outline: Rect, val handleCenter: Offset?)

internal data class EditorTableCellSelection(
  val overlay: TableOverlay,
  val range: EditorTableCellSelectionRange,
  val geometry: EditorTableCellSelectionGeometry,
)

internal fun resolveTableCellSelections(editor: Editor): List<EditorTableCellSelection> {
  val selectionCollapsed = editor.selection.isCollapsed()
  return editor.tableOverlays.mapNotNull { overlay ->
    val range =
      resolveTableCellSelectionRange(overlay = overlay, selectionCollapsed = selectionCollapsed)
        ?: return@mapNotNull null
    val geometry =
      resolveTableCellSelectionGeometry(overlay = overlay, range = range) ?: return@mapNotNull null
    EditorTableCellSelection(overlay = overlay, range = range, geometry = geometry)
  }
}

internal fun resolveActiveTableCellSelection(editor: Editor): EditorTableCellSelection? =
  resolveTableCellSelections(editor).firstOrNull { selection ->
    selection.geometry.handleCenter != null
  }

internal fun hasActiveTableCellSelection(editor: Editor): Boolean =
  resolveTableCellSelections(editor).isNotEmpty()

internal fun resolveTableCellSelectionRange(
  overlay: TableOverlay,
  selectionCollapsed: Boolean,
): EditorTableCellSelectionRange? {
  if (!overlay.isFocused) {
    return null
  }

  overlay.cellSelection?.let { cellSelection ->
    return EditorTableCellSelectionRange(
      anchor = EditorTableCellIndex(row = cellSelection.anchorRow, col = cellSelection.anchorCol),
      head = EditorTableCellIndex(row = cellSelection.headRow, col = cellSelection.headCol),
    )
  }

  if (!selectionCollapsed) {
    return null
  }

  val localRow = overlay.focusedRowIndex ?: return null
  val col = overlay.focusedColIndex ?: return null
  val row = overlay.rows.getOrNull(localRow)?.index ?: return null
  return EditorTableCellSelectionRange(
    anchor = EditorTableCellIndex(row = row, col = col),
    head = EditorTableCellIndex(row = row, col = col),
  )
}

internal fun resolveTableCellSelectionGeometry(
  overlay: TableOverlay,
  range: EditorTableCellSelectionRange,
): EditorTableCellSelectionGeometry? {
  if (overlay.rows.isEmpty() || overlay.columns.isEmpty()) {
    return null
  }

  val firstVisibleRow = overlay.rows.first().index
  val lastVisibleRow = overlay.rows.last().index
  val visibleRowStart = max(range.rowStart, firstVisibleRow)
  val visibleRowEnd = min(range.rowEnd, lastVisibleRow)
  if (visibleRowStart > visibleRowEnd) {
    return null
  }

  val localRowStart = overlay.rows.indexOfFirst { row -> row.index == visibleRowStart }
  val localRowEnd = overlay.rows.indexOfFirst { row -> row.index == visibleRowEnd }
  if (localRowStart < 0 || localRowEnd < 0) {
    return null
  }

  val colStart = range.colStart
  val colEnd = range.colEnd
  if (
    overlay.columns.none { column -> column.index == colStart } ||
      overlay.columns.none { column -> column.index == colEnd }
  ) {
    return null
  }

  val left = overlay.bounds.x + tableColumnLeft(overlay = overlay, col = colStart)
  val right = overlay.bounds.x + tableColumnRight(overlay = overlay, col = colEnd)
  val top = overlay.bounds.y + tableRowTop(overlay = overlay, localRow = localRowStart)
  val bottom = overlay.bounds.y + tableRowBottom(overlay = overlay, localRow = localRowEnd)
  if (right <= left || bottom <= top) {
    return null
  }

  val headLocalRow = overlay.rows.indexOfFirst { row -> row.index == range.head.row }
  val handleCenter =
    if (headLocalRow >= 0 && overlay.columns.any { column -> column.index == range.head.col }) {
      val handleX =
        if (range.head.col >= range.anchor.col) {
          tableColumnRight(overlay = overlay, col = range.head.col)
        } else {
          tableColumnLeft(overlay = overlay, col = range.head.col)
        }
      val handleY =
        if (range.head.row >= range.anchor.row) {
          tableRowBottom(overlay = overlay, localRow = headLocalRow)
        } else {
          tableRowTop(overlay = overlay, localRow = headLocalRow)
        }
      Offset(x = overlay.bounds.x + handleX, y = overlay.bounds.y + handleY)
    } else {
      null
    }

  return EditorTableCellSelectionGeometry(
    outline = Rect(left = left, top = top, right = right, bottom = bottom),
    handleCenter = handleCenter,
  )
}

private fun tableColumnLeft(overlay: TableOverlay, col: Int): Float {
  val index = overlay.columns.indexOfFirst { column -> column.index == col }
  if (index <= 0) {
    return 0f
  }
  return overlay.columns[index - 1].position
}

private fun tableColumnRight(overlay: TableOverlay, col: Int): Float {
  val column = overlay.columns.firstOrNull { column -> column.index == col } ?: return 0f
  return column.position
}

private fun tableRowTop(overlay: TableOverlay, localRow: Int): Float {
  if (localRow <= 0) {
    return 0f
  }
  return overlay.rows[localRow - 1].position
}

private fun tableRowBottom(overlay: TableOverlay, localRow: Int): Float =
  overlay.rows.getOrNull(localRow)?.position ?: 0f
