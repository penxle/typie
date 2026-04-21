package co.typie.screen.editor.editor.layout

import kotlin.math.max

internal data class EditorMeasuredSize(val width: Float = 0f, val height: Float = 0f)

internal data class EditorVisibleRect(val width: Float = 0f, val height: Float = 0f)

internal data class EditorVisibleArea(
  val viewport: EditorMeasuredSize = EditorMeasuredSize(),
  val headerHeight: Float = 0f,
  val topInset: Float = 0f,
  val imeInset: Float = 0f,
  val toolbarHeight: Float = 0f,
) {
  val topOcclusion: Float
    get() = topInset

  val bottomOcclusion: Float
    get() = imeInset + toolbarHeight

  val visibleBodyRect: EditorVisibleRect
    get() =
      EditorVisibleRect(
        width = viewport.width,
        height = max(0f, viewport.height - topOcclusion - bottomOcclusion),
      )

  val visibleExtensionRect: EditorVisibleRect
    get() = visibleBodyRect
}
