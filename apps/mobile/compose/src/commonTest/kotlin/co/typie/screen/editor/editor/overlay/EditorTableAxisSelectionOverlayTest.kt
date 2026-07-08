package co.typie.screen.editor.editor.overlay

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import co.typie.editor.ffi.Alignment
import co.typie.editor.ffi.Axis
import co.typie.editor.ffi.Rect as FfiRect
import co.typie.editor.ffi.TableBorderStyle
import co.typie.editor.ffi.TableOverlay
import co.typie.editor.ffi.TableOverlayCellSelection
import co.typie.editor.ffi.TableOverlayColumn
import co.typie.editor.ffi.TableOverlayRow
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorTableAxisSelectionOverlayTest {
  @Test
  fun `collapsed focused cell resolves row and column selectors`() {
    val overlay = tableOverlay(isFocused = true, focusedRowIndex = 1, focusedColIndex = 0)

    val selectors = resolveTableAxisSelectors(overlay = overlay, selectionCollapsed = true)

    assertEquals(
      listOf(
        EditorTableAxisSelector(
          overlay = overlay,
          axis = Axis.Horizontal,
          index = 1,
          count = 2,
          backgroundColor = "row-blue",
          center = Offset(10f, 80f),
        ),
        EditorTableAxisSelector(
          overlay = overlay,
          axis = Axis.Vertical,
          index = 0,
          count = 2,
          backgroundColor = "col-gray",
          center = Offset(35f, 20f),
        ),
      ),
      selectors,
    )
  }

  @Test
  fun `full row cell selection resolves only row selector`() {
    val overlay =
      tableOverlay(
        isFocused = true,
        cellSelection = cellSelection(anchorRow = 1, anchorCol = 0, headRow = 1, headCol = 1),
      )

    val selectors = resolveTableAxisSelectors(overlay = overlay, selectionCollapsed = false)

    assertEquals(listOf(Axis.Horizontal to 1), selectors.map { it.axis to it.index })
  }

  @Test
  fun `full column cell selection resolves only column selector`() {
    val overlay =
      tableOverlay(
        isFocused = true,
        cellSelection = cellSelection(anchorRow = 0, anchorCol = 1, headRow = 1, headCol = 1),
      )

    val selectors = resolveTableAxisSelectors(overlay = overlay, selectionCollapsed = false)

    assertEquals(listOf(Axis.Vertical to 1), selectors.map { it.axis to it.index })
  }

  @Test
  fun `row selector is omitted when selected row is outside visible fragment`() {
    val overlay =
      tableOverlay(
        isFocused = true,
        rows =
          listOf(TableOverlayRow(index = 0, height = 40f, position = 40f, backgroundColor = null)),
        cellSelection = cellSelection(anchorRow = 1, anchorCol = 0, headRow = 1, headCol = 1),
      )

    val selectors = resolveTableAxisSelectors(overlay = overlay, selectionCollapsed = false)

    assertEquals(emptyList(), selectors)
  }

  @Test
  fun `column selector overlaps table top edge like legacy handle`() {
    val rect =
      resolveSelectorButtonRect(
        center = Offset(80f, 100f),
        axis = Axis.Vertical,
        widthPx = 24f,
        heightPx = 18f,
        tableOverlapPx = 9f,
        screenPaddingPx = 4f,
        overlaySize = Size(width = 300f, height = 400f),
      )

    assertEquals(Rect(left = 68f, top = 91f, right = 92f, bottom = 109f), rect)
  }

  @Test
  fun `row selector overlaps table left edge like legacy handle`() {
    val rect =
      resolveSelectorButtonRect(
        center = Offset(100f, 80f),
        axis = Axis.Horizontal,
        widthPx = 18f,
        heightPx = 24f,
        tableOverlapPx = 9f,
        screenPaddingPx = 4f,
        overlaySize = Size(width = 300f, height = 400f),
      )

    assertEquals(Rect(left = 91f, top = 68f, right = 109f, bottom = 92f), rect)
  }

  private fun tableOverlay(
    isFocused: Boolean = false,
    focusedRowIndex: Int? = null,
    focusedColIndex: Int? = null,
    cellSelection: TableOverlayCellSelection? = null,
    rows: List<TableOverlayRow> =
      listOf(
        TableOverlayRow(index = 0, height = 40f, position = 40f, backgroundColor = null),
        TableOverlayRow(index = 1, height = 40f, position = 80f, backgroundColor = "row-blue"),
      ),
  ): TableOverlay =
    TableOverlay(
      tableId = "table",
      pageIdx = 0,
      bounds = FfiRect(x = 10f, y = 20f, width = 100f, height = 80f),
      borderStyle = TableBorderStyle.Solid,
      align = Alignment.Left,
      proportion = 1f,
      contentWidth = 100f,
      rows = rows,
      columns =
        listOf(
          TableOverlayColumn(
            index = 0,
            widthAsPx = 50f,
            position = 50f,
            backgroundColor = "col-gray",
          ),
          TableOverlayColumn(index = 1, widthAsPx = 50f, position = 100f, backgroundColor = null),
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
      backgroundColor = null,
      anchorRow = anchorRow,
      anchorCol = anchorCol,
      headRow = headRow,
      headCol = headCol,
    )
}
