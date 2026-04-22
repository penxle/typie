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
    get() = (viewport.height - effectiveBottomInset).coerceAtLeast(0f)

  val bottomOcclusion: Float
    get() = (viewport.height - visibleViewportBottom).coerceAtLeast(0f)

  val visibleViewportTop: Float
    get() = topOcclusion

  val visibleViewportBottom: Float
    get() = keyboardViewportBottom.coerceAtLeast(visibleViewportTop)

  val visibleBodySize: Size
    get() =
      Size(
        width = viewport.width,
        height = (visibleViewportBottom - visibleViewportTop).coerceAtLeast(0f),
      )
}
