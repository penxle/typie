package co.typie.editor.scroll

import androidx.compose.ui.geometry.Size

internal data class EditorVisibleArea(
  val viewport: Size = Size.Zero,
  val headerHeight: Float = 0f,
  val topInset: Float = 0f,
  val safeBottomInset: Float = 0f,
  val imeInset: Float = 0f,
  val bottomOcclusionInset: Float = 0f,
) {
  val topOcclusion: Float
    get() = topInset

  private val effectiveBottomOcclusion: Float
    get() = maxOf(safeBottomInset, imeInset, bottomOcclusionInset)

  private val bottomOccludedViewportBottom: Float
    get() = (viewport.height - effectiveBottomOcclusion).coerceAtLeast(0f)

  val bottomOcclusion: Float
    get() = (viewport.height - visibleViewportBottom).coerceAtLeast(0f)

  val visibleViewportTop: Float
    get() = topOcclusion

  val visibleViewportBottom: Float
    get() = bottomOccludedViewportBottom.coerceAtLeast(visibleViewportTop)

  val visibleBodySize: Size
    get() =
      Size(
        width = viewport.width,
        height = (visibleViewportBottom - visibleViewportTop).coerceAtLeast(0f),
      )
}
