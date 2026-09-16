package co.typie.screen.editor.editor.overlay

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect as ComposeRect
import androidx.compose.ui.graphics.ImageBitmap
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
import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class EditorSelectionHandleOverlayTest {
  @Test
  fun `android selection handles attach below the line and extend outward`() {
    for (type in listOf(EditorSelectionHandleType.From, EditorSelectionHandleType.To)) {
      val geometry =
        resolveSelectionHandleGeometry(
          type = type,
          endpointTopLeftInOverlay = Offset(100f, 200f),
          stemHeightPx = 20f,
          radiusPx = 12f,
          stemWidthPx = 2f,
          touchTargetPx = 44f,
          platform = Platform.Android,
        )
      val paint = geometry.touchTargetTopLeft + geometry.paintTopLeftInTouchTarget
      assertEquals(220f, paint.y)
      assertEquals(if (type == EditorSelectionHandleType.From) 76f else 100f, paint.x)
      assertTrue(geometry.containsTouch(paint + Offset(12f, 12f)))
    }
  }

  @Test
  fun `native drawable bounds preserve hotspots padding and minimum touch size`() {
    val images =
      mapOf(
        EditorSelectionHandleType.Cursor to ImageBitmap(53, 63),
        EditorSelectionHandleType.From to ImageBitmap(116, 58),
        EditorSelectionHandleType.To to ImageBitmap(116, 58),
      )
    val expectedPaintLeft =
      mapOf(
        EditorSelectionHandleType.Cursor to 174f,
        EditorSelectionHandleType.From to 113f,
        EditorSelectionHandleType.To to 171f,
      )
    for ((type, image) in images) {
      for (minimumTouchSize in listOf(44f, 120f)) {
        val geometry =
          resolveSelectionHandleOverlayGeometry(
            placement = EditorSelectionHandleOverlayPlacement(type, Offset(200f, 300f), 20f),
            density = minimumTouchSize / 44f,
            platform = Platform.Android,
            image = image,
          )
        val paint = geometry.touchTargetTopLeft + geometry.paintTopLeftInTouchTarget
        assertEquals(Offset(expectedPaintLeft.getValue(type), 320f), paint)
        assertEquals(maxOf(minimumTouchSize, image.width.toFloat()), geometry.touchTargetSize.width)
        assertEquals(
          maxOf(minimumTouchSize, image.height.toFloat()),
          geometry.touchTargetSize.height,
        )
        assertTrue(geometry.containsTouch(paint))
        assertTrue(
          geometry.containsTouch(paint + Offset(image.width.toFloat(), image.height.toFloat()))
        )
        assertFalse(geometry.containsTouch(geometry.touchTargetTopLeft - Offset(1f, 0f)))
      }
    }
  }

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
          directTouchInteraction = true,
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
          directTouchInteraction = true,
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
        directTouchInteraction = true,
      )
    )
  }

  @Test
  fun `selection handles follow direct touch and android selection gesture visibility`() {
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
            from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 0f, height = 8f)),
            to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 0f, height = 8f)),
            fromPosition = selection.anchor,
            toPosition = selection.head,
          ),
        pageSizes = listOf(Size(width = 100f, height = 100f)),
      )

    assertNull(
      resolveSelectionHandleOverlayPlacements(
        state = state,
        uiState = EditorUiState(),
        editorRectInOverlay = ComposeRect.Zero,
        density = 1f,
        directTouchInteraction = false,
      )
    )
    for (hidden in listOf(false, true, false)) {
      val placements =
        resolveSelectionHandleOverlayPlacements(
          state = state,
          uiState = EditorUiState().apply { updatePageOffset(0, Offset.Zero) },
          editorRectInOverlay = ComposeRect.Zero,
          density = 1f,
          directTouchInteraction = true,
          platform = Platform.Android,
          selectionHandlesHidden = hidden,
        )
      assertEquals(if (hidden) 0 else 2, placements?.size ?: 0)
    }
  }
}
