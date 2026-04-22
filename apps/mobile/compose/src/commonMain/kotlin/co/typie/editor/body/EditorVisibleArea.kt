package co.typie.editor.body

import androidx.compose.ui.geometry.Size
import kotlin.math.max
import kotlin.math.min

internal data class EditorVisibleArea(
  val viewport: Size = Size.Zero,
  val headerHeight: Float = 0f,
  val topInset: Float = 0f,
  val safeBottomInset: Float = 0f,
  val imeInset: Float = 0f,
  val toolbarTop: Float? = null,
) {
  val topOcclusion: Float
    get() = topInset

  private val effectiveBottomInset: Float
    get() = max(safeBottomInset, imeInset)

  private val keyboardViewportBottom: Float
    get() = max(0f, viewport.height - effectiveBottomInset)

  private val toolbarViewportBottom: Float
    get() =
      toolbarTop?.takeIf { it.isFinite() }?.coerceIn(visibleViewportTop, viewport.height)
        ?: viewport.height

  val bottomOcclusion: Float
    get() = max(0f, viewport.height - visibleViewportBottom)

  val visibleViewportTop: Float
    get() = topOcclusion

  val visibleViewportBottom: Float
    get() = min(keyboardViewportBottom, toolbarViewportBottom).coerceAtLeast(visibleViewportTop)

  fun resolveVisibleEditorViewportTop(editorTopInViewport: Float): Float =
    max(visibleViewportTop, editorTopInViewport)

  val visibleBodySize: Size
    get() =
      Size(width = viewport.width, height = max(0f, visibleViewportBottom - visibleViewportTop))

  val visibleExtensionSize: Size
    get() = visibleBodySize
}
