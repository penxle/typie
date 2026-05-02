package co.typie.editor.runtime

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Rect as FfiRect
import co.typie.editor.ffi.Size
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class EditorUiStateTest {
  @Test
  fun `ui state maps between page-local and viewport coordinates`() {
    val state = EditorUiState()
    state.updatePageOffset(page = 0, offset = Offset(40f, 0f))
    state.updatePageOffset(page = 1, offset = Offset(40f, 600f))
    val viewportTransform =
      state.resolveViewportTransform(
        pageSizes = listOf(Size(width = 300f, height = 600f), Size(width = 300f, height = 800f))
      )

    val global = viewportTransform.localToGlobal(page = 1, x = 100f, y = 50f)
    assertNotNull(global)
    assertEquals(140f, global.x)
    assertEquals(650f, global.y)

    val local = viewportTransform.globalToLocal(x = 90f, y = 620f)
    assertNotNull(local)
    assertEquals(1, local.page)
    assertEquals(50f, local.x)
    assertEquals(20f, local.y)
  }

  @Test
  fun `ui state applies display zoom when mapping coordinates`() {
    val state = EditorUiState()
    state.updatePageOffset(page = 0, offset = Offset(40f, 0f))
    state.updatePageOffset(page = 1, offset = Offset(40f, 1200f))
    state.updateDisplayZoom(2f)
    val viewportTransform =
      state.resolveViewportTransform(
        pageSizes = listOf(Size(width = 300f, height = 600f), Size(width = 300f, height = 800f))
      )

    val global = viewportTransform.localToGlobal(page = 1, x = 100f, y = 50f)
    assertNotNull(global)
    assertEquals(240f, global.x)
    assertEquals(1300f, global.y)

    val local = viewportTransform.globalToLocal(x = 140f, y = 1240f)
    assertNotNull(local)
    assertEquals(1, local.page)
    assertEquals(50f, local.x)
    assertEquals(20f, local.y)
  }

  @Test
  fun `clear resets focus and layout metrics`() {
    val state = EditorUiState()
    state.updateFocus(true)
    state.updatePageOffset(page = 0, offset = Offset(0f, 0f))
    state.updateExtensionAreaBounds(boundsInRoot = Rect(0f, 100f, 300f, 800f), density = 2f)
    state.updateEditorBounds(boundsInRoot = Rect(20f, 180f, 280f, 580f), density = 2f)

    state.clear()

    assertFalse(state.focused)
    assertNull(state.resolveViewportTransform().localToGlobal(page = 0, x = 0f, y = 0f))
    assertFalse(state.editorBoundsInContainer.isValid)
  }

  @Test
  fun `ui state resolves editor bounds relative to extension area container`() {
    val state = EditorUiState()

    state.updateExtensionAreaBounds(boundsInRoot = Rect(0f, 120f, 400f, 920f), density = 2f)
    state.updateEditorBounds(
      boundsInRoot = Rect(40f, 200f, 360f, 680f),
      clippedBoundsInRoot = Rect(40f, 240f, 360f, 640f),
      density = 2f,
    )

    assertEquals(20f, state.editorBoundsInContainer.x)
    assertEquals(40f, state.editorBoundsInContainer.y)
    assertEquals(160f, state.editorBoundsInContainer.width)
    assertEquals(240f, state.editorBoundsInContainer.height)
    assertEquals(Rect(40f, 200f, 360f, 680f), state.editorRectInRoot())
    assertEquals(Rect(40f, 240f, 360f, 640f), state.textClippingRectInRoot())
  }

  @Test
  fun `ui state converts container coordinates into editor local coordinates`() {
    val state = EditorUiState()

    state.updateExtensionAreaBounds(boundsInRoot = Rect(0f, 120f, 400f, 920f), density = 2f)
    state.updateEditorBounds(boundsInRoot = Rect(40f, 200f, 360f, 680f), density = 2f)

    val point = state.containerToEditorLocal(x = 80f, y = 100f)

    assertNotNull(point)
    assertEquals(60f, point.x)
    assertEquals(60f, point.y)
  }

  @Test
  fun `ui state maps cursor page coordinates into root coordinates`() {
    val state = EditorUiState()
    state.updateDisplayZoom(1.5f)
    state.updatePagePositionInRoot(page = 2, positionInRoot = Offset(200f, 300f), density = 2f)

    val rect =
      state.cursorRectInRoot(
        CursorMetrics(
          pageIdx = 2,
          caret = FfiRect(x = 10f, y = 20f, width = 1f, height = 18f),
          line = FfiRect(x = 0f, y = 18f, width = 100f, height = 20f),
        )
      )

    assertNotNull(rect)
    assertEquals(230f, rect.left)
    assertEquals(360f, rect.top)
    assertEquals(233f, rect.right)
    assertEquals(414f, rect.bottom)
  }

  @Test
  fun `ui state returns null root geometry before layout is known`() {
    val state = EditorUiState()

    assertNull(state.editorRectInRoot())
    assertNull(state.textClippingRectInRoot())
    assertNull(
      state.cursorRectInRoot(
        CursorMetrics(
          pageIdx = 0,
          caret = FfiRect(x = 0f, y = 0f, width = 1f, height = 18f),
          line = FfiRect(x = 0f, y = 0f, width = 100f, height = 20f),
        )
      )
    )
  }
}
