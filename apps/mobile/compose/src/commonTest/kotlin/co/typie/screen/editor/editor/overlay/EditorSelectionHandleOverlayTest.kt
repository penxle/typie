package co.typie.screen.editor.editor.overlay

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect as ComposeRect
import co.typie.editor.EditorState
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionEndpoints
import co.typie.editor.ffi.Size
import co.typie.editor.interaction.gestures.EditorSelectionHandleType
import co.typie.editor.interaction.gestures.resolveSelectionHandleGeometry
import co.typie.editor.runtime.EditorUiState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class EditorSelectionHandleOverlayTest {
  @Test
  fun `from handle hit target and paint offsets match legacy selection handle`() {
    val geometry =
      resolveSelectionHandleGeometry(
        type = EditorSelectionHandleType.From,
        endpointTopLeftInOverlay = Offset(100f, 200f),
        stemHeightPx = 8f,
        radiusPx = 8f,
        stemWidthPx = 2f,
        touchTargetPx = 44f,
      )

    assertEquals(Offset(77f, 174f), geometry.touchTargetTopLeft)
    assertEquals(44f, geometry.touchTargetSize.width)
    assertEquals(44f, geometry.touchTargetSize.height)
    assertEquals(Offset(14f, 10f), geometry.paintTopLeftInTouchTarget)
  }

  @Test
  fun `to handle hit target and paint offsets match legacy selection handle`() {
    val geometry =
      resolveSelectionHandleGeometry(
        type = EditorSelectionHandleType.To,
        endpointTopLeftInOverlay = Offset(100f, 200f),
        stemHeightPx = 8f,
        radiusPx = 8f,
        stemWidthPx = 2f,
        touchTargetPx = 44f,
      )

    assertEquals(Offset(79f, 190f), geometry.touchTargetTopLeft)
    assertEquals(44f, geometry.touchTargetSize.width)
    assertEquals(44f, geometry.touchTargetSize.height)
    assertEquals(Offset(14f, 10f), geometry.paintTopLeftInTouchTarget)
  }

  @Test
  fun `each selection handle requires a frame for its own endpoint page`() {
    val selection =
      Selection(
        anchor = Position("text", 0, Affinity.Downstream),
        head = Position("text", 5, Affinity.Downstream),
      )
    val state =
      EditorState.Initial.copy(
        selection = selection,
        selectionEndpoints =
          SelectionEndpoints(
            from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
            to = PageRect(pageIdx = 1, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
            fromPosition = selection.anchor,
            toPosition = selection.head,
          ),
        pageSizes = listOf(Size(width = 100f, height = 100f), Size(width = 100f, height = 100f)),
      )
    val uiState =
      EditorUiState().apply {
        updatePageOffset(page = 0, offset = Offset.Zero)
        updatePageOffset(page = 1, offset = Offset(0f, 100f))
      }

    assertEquals(
      2,
      resolveSelectionHandleOverlayPlacements(
          state = state,
          uiState = uiState,
          editorRectInOverlay = ComposeRect.Zero,
          density = 1f,
          pagePresented = { true },
        )
        ?.size,
    )
    val firstPageOnly =
      assertNotNull(
        resolveSelectionHandleOverlayPlacements(
          state = state,
          uiState = uiState,
          editorRectInOverlay = ComposeRect.Zero,
          density = 1f,
          pagePresented = { page -> page == 0 },
        )
      )
    assertEquals(listOf(EditorSelectionHandleType.From), firstPageOnly.map { it.type })
    assertNull(
      resolveSelectionHandleOverlayPlacements(
        state = state,
        uiState = uiState,
        editorRectInOverlay = ComposeRect.Zero,
        density = 1f,
        pagePresented = { false },
      )
    )
  }
}
