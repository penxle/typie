package co.typie.screen.editor.editor.overlay

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import co.typie.editor.ffi.Alignment
import co.typie.editor.ffi.Rect as FfiRect
import co.typie.editor.ffi.TableBorderStyle
import co.typie.editor.ffi.TableOverlay
import co.typie.editor.ffi.TableOverlayCellSelection
import co.typie.editor.ffi.TableOverlayColumn
import co.typie.editor.ffi.TableOverlayRow
import co.typie.editor.interaction.EditorTableCellSelection
import co.typie.editor.interaction.resolveTableCellSelectionGeometry
import co.typie.editor.interaction.resolveTableCellSelectionRange
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class EditorTableColumnResizeOverlayTest {
  @Test
  fun `focused cell resolves the selected column right edge as a column resize target`() {
    val selection = tableCellSelection(tableOverlay(focusedRowIndex = 0, focusedColIndex = 0))

    val target = checkNotNull(resolveTableColumnResizeTarget(selection))

    assertEquals(0, target.colIndex)
    assertEquals(0, target.localColIndex)
    assertFalse(target.isTableResize)
    assertEquals(60f, target.pageX)
  }

  @Test
  fun `last selected column resolves the table edge as a table resize target`() {
    val selection = tableCellSelection(tableOverlay(focusedRowIndex = 0, focusedColIndex = 2))

    val target = checkNotNull(resolveTableColumnResizeTarget(selection))

    assertEquals(2, target.colIndex)
    assertTrue(target.isTableResize)
    assertEquals(190f, target.pageX)
  }

  @Test
  fun `column resize clamps both adjacent columns to the minimum width`() {
    val widths = listOf(50f, 70f, 60f)

    assertEquals(listOf(80f, 40f, 60f), resizeTableColumnWidths(widths, colIndex = 0, deltaX = 40f))
    assertEquals(
      listOf(40f, 80f, 60f),
      resizeTableColumnWidths(widths, colIndex = 0, deltaX = -20f),
    )
  }

  @Test
  fun `table resize proportion clamps to overlay min and max widths`() {
    val overlay = tableOverlay(bounds = FfiRect(x = 10f, y = 20f, width = 150f, height = 80f))

    assertEquals(124, resolveTableResizeProportion(overlay = overlay, deltaX = -80f))
    assertEquals(160, resolveTableResizeProportion(overlay = overlay, deltaX = 80f))
  }

  @Test
  fun `table resize committed delta matches rounded proportion`() {
    val overlay = tableOverlay(bounds = FfiRect(x = 10f, y = 20f, width = 150f, height = 80f))

    assertEquals(0f, resolveTableResizeCommittedDelta(overlay = overlay, deltaX = 0.4f))
    assertEquals(10f, resolveTableResizeCommittedDelta(overlay = overlay, deltaX = 9.6f))
  }

  @Test
  fun `table resize preview delta follows the pointer before rounded commit`() {
    val overlay = tableOverlay(bounds = FfiRect(x = 10f, y = 20f, width = 150f, height = 80f))

    assertEquals(0.4f, resolveTableResizePreviewDelta(overlay = overlay, deltaX = 0.4f))
    assertEquals(9.6f, resolveTableResizePreviewDelta(overlay = overlay, deltaX = 9.6f))
  }

  @Test
  fun `drag delta converts screen pixels to page units`() {
    assertEquals(10f, dragDeltaToPageDelta(deltaPx = 60f, pxPerPageUnit = 6f))
    assertEquals(0f, dragDeltaToPageDelta(deltaPx = 60f, pxPerPageUnit = 0f))
  }

  @Test
  fun `resize hit rect excludes the active cell selection handle target`() {
    val rects =
      splitTableColumnResizeHitRects(
        centerX = 100f,
        top = 0f,
        bottom = 100f,
        halfWidth = 12f,
        blockedCenter = Offset(100f, 100f),
        blockedHalfSize = 18f,
      )

    assertEquals(listOf(Rect(left = 88f, top = 0f, right = 112f, bottom = 82f)), rects)
  }

  private fun tableCellSelection(overlay: TableOverlay): EditorTableCellSelection {
    val range = checkNotNull(resolveTableCellSelectionRange(overlay, selectionCollapsed = true))
    val geometry = checkNotNull(resolveTableCellSelectionGeometry(overlay, range))
    return EditorTableCellSelection(overlay = overlay, range = range, geometry = geometry)
  }

  private fun tableOverlay(
    bounds: FfiRect = FfiRect(x = 10f, y = 20f, width = 100f, height = 80f),
    focusedRowIndex: Int? = null,
    focusedColIndex: Int? = null,
    cellSelection: TableOverlayCellSelection? = null,
  ): TableOverlay =
    TableOverlay(
      tableId = "table",
      pageIdx = 0,
      bounds = bounds,
      borderStyle = TableBorderStyle.Solid,
      align = Alignment.Left,
      proportion = 1f,
      contentWidth = 100f,
      minProportionWidth = 83f,
      maxProportionWidth = 160f,
      rows =
        listOf(
          TableOverlayRow(index = 0, height = 40f, position = 40f, backgroundColor = null),
          TableOverlayRow(index = 1, height = 40f, position = 80f, backgroundColor = null),
        ),
      columns =
        listOf(
          TableOverlayColumn(index = 0, widthAsPx = 50f, position = 50f, backgroundColor = null),
          TableOverlayColumn(index = 1, widthAsPx = 70f, position = 120f, backgroundColor = null),
          TableOverlayColumn(index = 2, widthAsPx = 60f, position = 180f, backgroundColor = null),
        ),
      rowCount = 2,
      isLastRowFragment = true,
      isFocused = true,
      focusedRowIndex = focusedRowIndex,
      focusedColIndex = focusedColIndex,
      cellSelection = cellSelection,
    )
}
