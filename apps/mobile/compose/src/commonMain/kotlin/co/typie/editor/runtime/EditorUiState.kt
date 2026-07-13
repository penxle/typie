package co.typie.editor.runtime

import androidx.compose.runtime.Stable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import co.typie.editor.EditorViewportTransform
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Size

@Stable
class EditorUiState {
  var focused by mutableStateOf(false)
    private set

  internal val contextMenu = EditorContextMenuState()

  var displayZoom by mutableStateOf(1f)
    private set

  var editorBoundsInContainer by mutableStateOf(EditorBoundsInContainer())
    private set

  private val pageOffsets = mutableStateMapOf<Int, Offset>()
  // Root-space page positions are for IME geometry; pageOffsets remain editor-local.
  private val pagePositionsInRoot = mutableStateMapOf<Int, PagePositionInRoot>()
  private var extensionAreaBoundsInRoot: Rect = Rect.Zero
  private var editorBoundsInRoot: Rect = Rect.Zero
  private var editorClippedBoundsInRoot: Rect = Rect.Zero

  fun updateFocus(focused: Boolean) {
    this.focused = focused
  }

  fun clear() {
    focused = false
    contextMenu.reset()
    displayZoom = 1f
    pageOffsets.clear()
    pagePositionsInRoot.clear()
    extensionAreaBoundsInRoot = Rect.Zero
    editorBoundsInRoot = Rect.Zero
    editorClippedBoundsInRoot = Rect.Zero
    editorBoundsInContainer = EditorBoundsInContainer()
  }

  fun updateDisplayZoom(displayZoom: Float) {
    val normalized =
      if (displayZoom.isFinite() && displayZoom > 0f) {
        displayZoom
      } else {
        1f
      }
    if (this.displayZoom == normalized) {
      return
    }

    this.displayZoom = normalized
  }

  fun containerToEditorLocal(x: Float, y: Float): Offset? {
    val bounds = editorBoundsInContainer
    if (!bounds.isValid) {
      return null
    }

    return Offset(x - bounds.x, y - bounds.y)
  }

  fun resolveViewportTransform(pageSizes: List<Size> = emptyList()): EditorViewportTransform =
    EditorViewportTransform(
      pageOffsets = pageOffsets,
      pageSizes = pageSizes,
      displayZoom = displayZoom,
    )

  fun editorRectInRoot(): Rect? = editorBoundsInRoot.takeIf { it.isUsable }

  internal fun extensionAreaRectInRoot(): Rect? = extensionAreaBoundsInRoot.takeIf { it.isUsable }

  fun textClippingRectInRoot(): Rect? = editorClippedBoundsInRoot.takeIf { it.isUsable }

  fun cursorRectInRoot(cursor: CursorMetrics?): Rect? {
    cursor ?: return null
    val pagePositionInRoot = pagePositionsInRoot[cursor.pageIdx] ?: return null
    if (pagePositionInRoot.density <= 0f) {
      return null
    }
    val caret = cursor.caret
    if (
      !caret.x.isFinite() ||
        !caret.y.isFinite() ||
        !caret.width.isFinite() ||
        !caret.height.isFinite() ||
        caret.width < 0f ||
        caret.height <= 0f
    ) {
      return null
    }

    val scale = displayZoom * pagePositionInRoot.density
    val left = pagePositionInRoot.position.x + caret.x * scale
    val top = pagePositionInRoot.position.y + caret.y * scale
    return Rect(
      left = left,
      top = top,
      right = left + caret.width * scale,
      bottom = top + caret.height * scale,
    )
  }

  internal fun updatePageOffset(page: Int, offset: Offset) {
    if (pageOffsets[page] == offset) {
      return
    }

    pageOffsets[page] = offset
  }

  internal fun updatePagePositionInRoot(page: Int, positionInRoot: Offset, density: Float) {
    val position = PagePositionInRoot(position = positionInRoot, density = density)
    if (pagePositionsInRoot[page] == position) {
      return
    }

    pagePositionsInRoot[page] = position
  }

  internal fun clearPageOffset(page: Int) {
    pageOffsets.remove(page)
    pagePositionsInRoot.remove(page)
  }

  fun updateExtensionAreaBounds(boundsInRoot: Rect, density: Float) {
    extensionAreaBoundsInRoot = boundsInRoot
    syncEditorBoundsInContainer(density)
  }

  fun updateEditorBounds(
    boundsInRoot: Rect,
    clippedBoundsInRoot: Rect = boundsInRoot,
    density: Float,
  ) {
    editorBoundsInRoot = boundsInRoot
    editorClippedBoundsInRoot = clippedBoundsInRoot
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

private data class PagePositionInRoot(val position: Offset, val density: Float)

private val Rect.isUsable: Boolean
  get() =
    width > 0f &&
      height > 0f &&
      left.isFinite() &&
      top.isFinite() &&
      right.isFinite() &&
      bottom.isFinite()

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
