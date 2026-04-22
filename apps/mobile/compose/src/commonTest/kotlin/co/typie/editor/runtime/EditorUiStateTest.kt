package co.typie.editor.runtime

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
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

    val global = state.localToGlobal(page = 1, x = 100f, y = 50f)
    assertNotNull(global)
    assertEquals(140f, global.x)
    assertEquals(650f, global.y)

    val local =
      state.globalToLocal(
        x = 90f,
        y = 620f,
        pageSizes = listOf(Size(width = 300f, height = 600f), Size(width = 300f, height = 800f)),
      )
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

    val global = state.localToGlobal(page = 1, x = 100f, y = 50f)
    assertNotNull(global)
    assertEquals(240f, global.x)
    assertEquals(1300f, global.y)

    val local =
      state.globalToLocal(
        x = 140f,
        y = 1240f,
        pageSizes = listOf(Size(width = 300f, height = 600f), Size(width = 300f, height = 800f)),
      )
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
    assertNull(state.localToGlobal(page = 0, x = 0f, y = 0f))
    assertFalse(state.editorBoundsInContainer.isValid)
  }

  @Test
  fun `ui state resolves editor bounds relative to extension area container`() {
    val state = EditorUiState()

    state.updateExtensionAreaBounds(boundsInRoot = Rect(0f, 120f, 400f, 920f), density = 2f)
    state.updateEditorBounds(boundsInRoot = Rect(40f, 200f, 360f, 680f), density = 2f)

    assertEquals(20f, state.editorBoundsInContainer.x)
    assertEquals(40f, state.editorBoundsInContainer.y)
    assertEquals(160f, state.editorBoundsInContainer.width)
    assertEquals(240f, state.editorBoundsInContainer.height)
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
}
