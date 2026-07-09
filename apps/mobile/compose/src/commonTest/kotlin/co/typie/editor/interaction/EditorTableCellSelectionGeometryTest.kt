package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect as ComposeRect
import co.typie.editor.ffi.Alignment
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.TableBorderStyle
import co.typie.editor.ffi.TableOverlay
import co.typie.editor.ffi.TableOverlayCellSelection
import co.typie.editor.ffi.TableOverlayColumn
import co.typie.editor.ffi.TableOverlayRow
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class EditorTableCellSelectionGeometryTest {
  @Test
  fun `collapsed focused cell resolves single-cell range and handle`() {
    val overlay = tableOverlay(isFocused = true, focusedRowIndex = 1, focusedColIndex = 0)

    val range = resolveTableCellSelectionRange(overlay = overlay, selectionCollapsed = true)

    assertEquals(
      EditorTableCellSelectionRange(
        anchor = EditorTableCellIndex(row = 1, col = 0),
        head = EditorTableCellIndex(row = 1, col = 0),
      ),
      range,
    )
    assertEquals(
      EditorTableCellSelectionGeometry(
        outline = ComposeRect(left = 10f, top = 60f, right = 60f, bottom = 100f),
        handleCenter = Offset(60f, 100f),
      ),
      resolveTableCellSelectionGeometry(overlay = overlay, range = range!!),
    )
  }

  @Test
  fun `reversed cell selection places handle at head corner`() {
    val overlay =
      tableOverlay(
        isFocused = true,
        cellSelection = cellSelection(anchorRow = 1, anchorCol = 1, headRow = 0, headCol = 0),
      )

    val range = resolveTableCellSelectionRange(overlay = overlay, selectionCollapsed = false)

    assertEquals(
      EditorTableCellSelectionRange(
        anchor = EditorTableCellIndex(row = 1, col = 1),
        head = EditorTableCellIndex(row = 0, col = 0),
      ),
      range,
    )
    assertEquals(
      EditorTableCellSelectionGeometry(
        outline = ComposeRect(left = 10f, top = 20f, right = 110f, bottom = 100f),
        handleCenter = Offset(10f, 20f),
      ),
      resolveTableCellSelectionGeometry(overlay = overlay, range = range!!),
    )
  }

  @Test
  fun `cell selection spanning fragments keeps outline on fragment without head`() {
    val overlay =
      tableOverlay(
        isFocused = true,
        rows = listOf(TableOverlayRow(index = 0, height = 40f, position = 40f)),
        cellSelection = cellSelection(anchorRow = 0, anchorCol = 0, headRow = 1, headCol = 1),
      )

    val range = resolveTableCellSelectionRange(overlay = overlay, selectionCollapsed = false)

    val geometry = resolveTableCellSelectionGeometry(overlay = overlay, range = range!!)
    assertEquals(ComposeRect(left = 10f, top = 20f, right = 110f, bottom = 60f), geometry?.outline)
    assertNull(geometry?.handleCenter)
  }

  @Test
  fun `non-focused table boundary selection does not keep table handle active`() {
    val overlay =
      tableOverlay(
        isFocused = false,
        cellSelection = cellSelection(anchorRow = 0, anchorCol = 0, headRow = 1, headCol = 1),
      )

    assertNull(resolveTableCellSelectionRange(overlay = overlay, selectionCollapsed = false))
  }

  private fun tableOverlay(
    isFocused: Boolean = false,
    focusedRowIndex: Int? = null,
    focusedColIndex: Int? = null,
    cellSelection: TableOverlayCellSelection? = null,
    rows: List<TableOverlayRow> =
      listOf(
        TableOverlayRow(index = 0, height = 40f, position = 40f),
        TableOverlayRow(index = 1, height = 40f, position = 80f),
      ),
  ): TableOverlay =
    TableOverlay(
      tableId = "table",
      pageIdx = 0,
      bounds = Rect(x = 10f, y = 20f, width = 100f, height = 80f),
      borderStyle = TableBorderStyle.Solid,
      align = Alignment.Left,
      proportion = 1f,
      contentWidth = 100f,
      minProportionWidth = 83f,
      maxProportionWidth = 100f,
      rows = rows,
      columns =
        listOf(
          TableOverlayColumn(index = 0, widthAsPx = 50f, position = 50f),
          TableOverlayColumn(index = 1, widthAsPx = 50f, position = 100f),
        ),
      rowCount = 2,
      isLastRowFragment = true,
      isFocused = isFocused,
      focusedRowIndex = focusedRowIndex,
      focusedColIndex = focusedColIndex,
      cellSelection = cellSelection,
    )

  private fun cellSelection(
    anchorRow: Int,
    anchorCol: Int,
    headRow: Int,
    headCol: Int,
  ): TableOverlayCellSelection =
    TableOverlayCellSelection(
      anchorRow = anchorRow,
      anchorCol = anchorCol,
      headRow = headRow,
      headCol = headCol,
    )
}
