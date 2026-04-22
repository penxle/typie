package co.typie.screen.editor.editor.layout

import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.resolveEditorBodyLayoutPolicy
import co.typie.editor.body.resolveIntrinsicBottomSpace
import co.typie.editor.ffi.Size
import co.typie.screen.editor.editor.scroll.CursorVisibleMargin
import co.typie.screen.editor.editor.scroll.EditorScrollMode
import co.typie.screen.editor.editor.scroll.EditorScrollPolicy
import co.typie.screen.editor.editor.scroll.resolveEditorScrollPolicy
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
      defaultBottomPadding = defaultBottomPadding,
    )
  val activeBottomPadding =
    when (scrollPolicy.mode) {
      EditorScrollMode.KeepCursorVisible -> defaultBottomPadding
      EditorScrollMode.Typewriter ->
        max(
          defaultBottomPadding,
          max(0f, scrollPolicy.typewriterBottomPadding - intrinsicBottomSpace),
        )
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
