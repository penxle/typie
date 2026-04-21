package co.typie.editor.runtime

import androidx.compose.runtime.Stable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import co.typie.editor.PagePoint
import co.typie.editor.ffi.Size
import co.typie.editor.globalToLocal
import co.typie.editor.localToGlobal

@Stable
class EditorUiState {
  var focused by mutableStateOf(false)
    private set

  var editorBoundsInContainer by mutableStateOf(EditorBoundsInContainer())
    private set

  private val pageOffsets = mutableStateMapOf<Int, Offset>()
  private var extensionAreaBoundsInRoot: Rect = Rect.Zero
  private var editorBoundsInRoot: Rect = Rect.Zero

  fun updateFocus(focused: Boolean) {
    this.focused = focused
  }

  fun clear() {
    focused = false
    pageOffsets.clear()
    extensionAreaBoundsInRoot = Rect.Zero
    editorBoundsInRoot = Rect.Zero
    editorBoundsInContainer = EditorBoundsInContainer()
  }

  fun localToGlobal(page: Int, x: Float, y: Float): Offset? = localToGlobal(page, x, y, pageOffsets)

  fun globalToLocal(x: Float, y: Float, pageSizes: List<Size>): PagePoint? =
    globalToLocal(x, y, pageOffsets, pageSizes)

  internal fun updatePageOffset(page: Int, offset: Offset) {
    pageOffsets[page] = offset
  }

  internal fun clearPageOffset(page: Int) {
    pageOffsets.remove(page)
  }

  fun updateExtensionAreaBounds(boundsInRoot: Rect, density: Float) {
    extensionAreaBoundsInRoot = boundsInRoot
    syncEditorBoundsInContainer(density)
  }

  fun updateEditorBounds(boundsInRoot: Rect, density: Float) {
    editorBoundsInRoot = boundsInRoot
    syncEditorBoundsInContainer(density)
  }

  private fun syncEditorBoundsInContainer(density: Float) {
    if (
      density <= 0f ||
        extensionAreaBoundsInRoot.width <= 0f ||
        extensionAreaBoundsInRoot.height <= 0f ||
        editorBoundsInRoot.width <= 0f ||
        editorBoundsInRoot.height <= 0f
    ) {
      editorBoundsInContainer = EditorBoundsInContainer()
      return
    }

    editorBoundsInContainer =
      EditorBoundsInContainer(
        x = (editorBoundsInRoot.left - extensionAreaBoundsInRoot.left) / density,
        y = (editorBoundsInRoot.top - extensionAreaBoundsInRoot.top) / density,
        width = editorBoundsInRoot.width / density,
        height = editorBoundsInRoot.height / density,
      )
  }
}

data class EditorBoundsInContainer(
  val x: Float = 0f,
  val y: Float = 0f,
  val width: Float = 0f,
  val height: Float = 0f,
) {
  val isValid: Boolean
    get() = width > 0f && height > 0f
}

val LocalEditorUiState = compositionLocalOf<EditorUiState> { error("No EditorUiState provided") }
