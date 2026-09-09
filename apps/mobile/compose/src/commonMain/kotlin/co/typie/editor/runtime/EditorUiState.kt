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
import co.typie.editor.ffi.Rect as FfiRect
import co.typie.editor.ffi.Size

@Stable
class EditorUiState {
  var focused by mutableStateOf(false)
    private set

  private var inputSessionOwner by mutableStateOf<Any?>(null)

  internal val editorInputSessionActive: Boolean
    get() = inputSessionOwner != null

  internal val contextMenu = EditorContextMenuState()

  var displayZoom by mutableStateOf(1f)
    private set

  var editorBoundsInContainer by mutableStateOf(EditorBoundsInContainer())
    private set

  private val pageOffsets = mutableStateMapOf<Int, Offset>()
  // Root-space page positions are for IME geometry; pageOffsets remain editor-local.
  private val pagePositionsInRoot = mutableStateMapOf<Int, PagePositionInRoot>()
  private var interactionSurfaceBoundsInRoot: Rect = Rect.Zero
  private var editorBoundsInRoot: Rect = Rect.Zero
  private var editorClippedBoundsInRoot: Rect = Rect.Zero

  fun updateFocus(focused: Boolean) {
    this.focused = focused
  }

  internal fun acquireInputSession(owner: Any) {
    inputSessionOwner = owner
  }

  internal fun releaseInputSession(owner: Any) {
    if (inputSessionOwner === owner) {
      inputSessionOwner = null
    }
  }

  fun clear() {
    focused = false
    inputSessionOwner = null
    contextMenu.reset()
    displayZoom = 1f
    pageOffsets.clear()
    pagePositionsInRoot.clear()
    interactionSurfaceBoundsInRoot = Rect.Zero
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

  internal fun containsDocumentInteraction(positionInRoot: Offset): Boolean =
    interactionSurfaceBoundsInRoot.isUsable &&
      interactionSurfaceBoundsInRoot.contains(positionInRoot)

  fun textClippingRectInRoot(): Rect? = editorClippedBoundsInRoot.takeIf { it.isUsable }

  // Every page is laid out, including offscreen pages. Use the document origin so moving the
  // selection between pages cannot be mistaken for scrolling the text.
  internal fun unclippedTextOffsetInRoot(): Offset? = pagePositionsInRoot[0]?.position

  fun cursorRectInRoot(cursor: CursorMetrics?): Rect? = cursor?.let {
    pageRectInRoot(it.pageIdx, it.caret)
  }

  internal fun pageRectInRoot(page: Int, rect: FfiRect): Rect? {
    val pagePositionInRoot = pagePositionsInRoot[page] ?: return null
    if (pagePositionInRoot.density <= 0f) {
      return null
    }
    if (
      !rect.x.isFinite() ||
        !rect.y.isFinite() ||
        !rect.width.isFinite() ||
        !rect.height.isFinite() ||
        rect.width < 0f ||
        rect.height <= 0f
    ) {
      return null
    }

    val scale = displayZoom * pagePositionInRoot.density
    val left = pagePositionInRoot.position.x + rect.x * scale
    val top = pagePositionInRoot.position.y + rect.y * scale
    return Rect(
      left = left,
      top = top,
      right = left + rect.width * scale,
      bottom = top + rect.height * scale,
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

  fun updateInteractionSurfaceBounds(boundsInRoot: Rect, density: Float) {
    interactionSurfaceBoundsInRoot = boundsInRoot
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
        interactionSurfaceBoundsInRoot.width <= 0f ||
        interactionSurfaceBoundsInRoot.height <= 0f ||
        editorBoundsInRoot.width <= 0f ||
        editorBoundsInRoot.height <= 0f
    ) {
      editorBoundsInContainer = EditorBoundsInContainer()
      return
    }

    editorBoundsInContainer =
      EditorBoundsInContainer(
        x = (editorBoundsInRoot.left - interactionSurfaceBoundsInRoot.left) / density,
        y = (editorBoundsInRoot.top - interactionSurfaceBoundsInRoot.top) / density,
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

  internal fun toPxRect(density: Float): Rect? {
    if (!isValid || density <= 0f) {
      return null
    }
    return Rect(
      left = x * density,
      top = y * density,
      right = (x + width) * density,
      bottom = (y + height) * density,
    )
  }
}

val LocalEditorUiState = compositionLocalOf<EditorUiState> { error("No EditorUiState provided") }
