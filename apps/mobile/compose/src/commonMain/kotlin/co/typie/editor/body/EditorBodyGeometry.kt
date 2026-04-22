package co.typie.editor.body

import co.typie.editor.ffi.Size
import co.typie.editor.scroll.CursorVisibleMargin
import co.typie.editor.scroll.EditorScrollMode
import co.typie.editor.scroll.EditorScrollPolicy
import co.typie.editor.scroll.resolveEditorScrollPolicy
import kotlin.math.max

private const val DefaultExtensionPadding = 40f

internal data class EditorBodyGeometry(
  val pageColumnWidth: Float,
  val visibleBodyRect: EditorVisibleRect,
  val visibleExtensionRect: EditorVisibleRect,
  val minimumBodyHeight: Float,
  val defaultTopPadding: Float,
  val defaultBottomPadding: Float,
  val activeBottomPadding: Float,
  val scrollPolicy: EditorScrollPolicy,
)

internal fun resolveEditorBodyGeometry(
  visibleArea: EditorVisibleArea,
  layoutSpec: EditorDocumentLayoutSpec,
  pageSizes: List<Size>,
  typewriterEnabled: Boolean = false,
  typewriterPosition: Float = 0.5f,
  cursorHeight: Float = 0f,
): EditorBodyGeometry {
  val visibleBodyRect = visibleArea.visibleBodyRect
  val visibleExtensionRect = visibleArea.visibleExtensionRect
  val layoutPolicy =
    resolveEditorBodyLayoutPolicy(
      availableBodyWidth = visibleExtensionRect.width,
      layoutSpec = layoutSpec,
      pageSizes = pageSizes,
    )
  val intrinsicBottomSpace = layoutSpec.resolveIntrinsicBottomSpace()
  val defaultBottomPadding =
    max(0f, visibleArea.bottomOcclusion + CursorVisibleMargin - intrinsicBottomSpace)
  val minimumBodyHeight =
    resolveMinimumBodyHeight(
      viewportHeight = visibleArea.viewport.height,
      headerHeight = visibleArea.headerHeight,
      bottomOcclusion = visibleArea.bottomOcclusion,
    )
  val scrollPolicy =
    resolveEditorScrollPolicy(
      visibleArea = visibleArea,
      intrinsicBottomSpace = intrinsicBottomSpace,
      typewriterEnabled = typewriterEnabled,
      typewriterPosition = typewriterPosition,
      cursorHeight = cursorHeight,
    )
  val activeBottomPadding =
    when (scrollPolicy.mode) {
      EditorScrollMode.KeepCursorVisible -> defaultBottomPadding
      EditorScrollMode.Typewriter -> scrollPolicy.typewriterBottomPadding
    }

  return EditorBodyGeometry(
    pageColumnWidth = layoutPolicy.pageColumnWidth,
    visibleBodyRect = visibleBodyRect,
    visibleExtensionRect = visibleExtensionRect,
    minimumBodyHeight = minimumBodyHeight,
    defaultTopPadding = DefaultExtensionPadding,
    defaultBottomPadding = defaultBottomPadding,
    activeBottomPadding = activeBottomPadding,
    scrollPolicy = scrollPolicy,
  )
}

internal fun resolveMinimumBodyHeight(
  viewportHeight: Float,
  headerHeight: Float,
  bottomOcclusion: Float,
): Float = max(0f, viewportHeight - headerHeight - bottomOcclusion)
