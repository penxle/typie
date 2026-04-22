package co.typie.screen.editor.editor.layout

import kotlin.math.max
import kotlin.math.min

internal data class EditorMeasuredSize(val width: Float = 0f, val height: Float = 0f)

internal data class EditorVisibleRect(val width: Float = 0f, val height: Float = 0f)

internal data class EditorVisibleArea(
  val viewport: EditorMeasuredSize = EditorMeasuredSize(),
  val headerHeight: Float = 0f,
  val topInset: Float = 0f,
  val imeInset: Float = 0f,
  val toolbarTop: Float? = null,
) {
  val topOcclusion: Float
    get() = topInset

  private val keyboardViewportBottom: Float
    get() = max(0f, viewport.height - imeInset)

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

  val visibleBodyRect: EditorVisibleRect
    get() =
      EditorVisibleRect(
        width = viewport.width,
        height = max(0f, visibleViewportBottom - visibleViewportTop),
      )

  val visibleExtensionRect: EditorVisibleRect
    get() = visibleBodyRect
}
