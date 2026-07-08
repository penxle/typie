package co.typie.screen.editor.editor.overlay

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect as ComposeRect
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.Alignment
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.TableBorderStyle
import co.typie.editor.ffi.TableOverlay
import co.typie.editor.ffi.TableOverlayCellSelection
import co.typie.editor.ffi.TableOverlayColumn
import co.typie.editor.ffi.TableOverlayRow
import co.typie.editor.runtime.EditorUiState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest

class EditorTableCellSelectionOverlayTest {
  @Test
  fun `cell selection overlay keeps placement for each table fragment`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("cell-text", 0, Affinity.Downstream),
          head = Position("cell-text", 0, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { selection },
          tableOverlaysProvider = {
            listOf(
              tableOverlay(
                bounds = Rect(x = 10f, y = 20f, width = 100f, height = 40f),
                rows =
                  listOf(
                    TableOverlayRow(index = 0, height = 40f, position = 40f, backgroundColor = null)
                  ),
              ),
              tableOverlay(
                bounds = Rect(x = 10f, y = 80f, width = 100f, height = 40f),
                rows =
                  listOf(
                    TableOverlayRow(index = 1, height = 40f, position = 40f, backgroundColor = null)
                  ),
              ),
            )
          },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val uiState = EditorUiState().apply { updatePageOffset(page = 0, offset = Offset.Zero) }

      val placements =
        resolveTableCellSelectionOverlayPlacements(
          editor = editor,
          uiState = uiState,
          editorRectInOverlay = ComposeRect.Zero,
          density = 1f,
        )

      assertEquals(2, placements.size)
      assertEquals(
        ComposeRect(left = 10f, top = 20f, right = 110f, bottom = 60f),
        placements[0].outline,
      )
      assertNull(placements[0].handleCenter)
      assertEquals(
        ComposeRect(left = 10f, top = 80f, right = 110f, bottom = 120f),
        placements[1].outline,
      )
      assertEquals(Offset(110f, 120f), placements[1].handleCenter)
    }

  private fun tableOverlay(bounds: Rect, rows: List<TableOverlayRow>): TableOverlay =
    TableOverlay(
      tableId = "table",
      pageIdx = 0,
      bounds = bounds,
      borderStyle = TableBorderStyle.Solid,
      align = Alignment.Left,
      proportion = 1f,
      contentWidth = 100f,
      rows = rows,
      columns =
        listOf(
          TableOverlayColumn(index = 0, widthAsPx = 50f, position = 50f, backgroundColor = null),
          TableOverlayColumn(index = 1, widthAsPx = 50f, position = 100f, backgroundColor = null),
        ),
      rowCount = 2,
      isLastRowFragment = true,
      isFocused = true,
      focusedRowIndex = null,
      focusedColIndex = null,
      cellSelection =
        TableOverlayCellSelection(
          backgroundColor = null,
          anchorRow = 0,
          anchorCol = 0,
          headRow = 1,
          headCol = 1,
        ),
    )
}
