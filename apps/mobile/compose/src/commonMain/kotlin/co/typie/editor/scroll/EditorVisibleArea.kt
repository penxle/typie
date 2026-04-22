package co.typie.editor.scroll

import androidx.compose.ui.geometry.Size
import kotlin.math.max

internal data class EditorVisibleArea(
  val viewport: Size = Size.Zero,
  val headerHeight: Float = 0f,
  val topInset: Float = 0f,
  val safeBottomInset: Float = 0f,
  val imeInset: Float = 0f,
) {
  // viewport 자체가 이미 toolbar를 제외한 scroll viewport이므로, 여기서는
  // top/bottom system occlusion만 반영한다.
  val topOcclusion: Float
    get() = topInset

  private val effectiveBottomInset: Float
    get() = max(safeBottomInset, imeInset)

  private val keyboardViewportBottom: Float
    get() = max(0f, viewport.height - effectiveBottomInset)

  val bottomOcclusion: Float
    get() = max(0f, viewport.height - visibleViewportBottom)

  val visibleViewportTop: Float
    get() = topOcclusion

  val visibleViewportBottom: Float
    get() = keyboardViewportBottom.coerceAtLeast(visibleViewportTop)

  fun resolveVisibleEditorViewportTop(editorTopInViewport: Float): Float =
    max(visibleViewportTop, editorTopInViewport)

  val visibleBodySize: Size
    get() =
      Size(width = viewport.width, height = max(0f, visibleViewportBottom - visibleViewportTop))

  val visibleExtensionSize: Size
    get() = visibleBodySize
}
