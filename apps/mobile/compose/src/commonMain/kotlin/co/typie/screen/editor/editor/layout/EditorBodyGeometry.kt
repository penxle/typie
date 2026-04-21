package co.typie.screen.editor.editor.layout

import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.resolveEditorBodyLayoutPolicy
import co.typie.editor.ffi.Size
import kotlin.math.max

private const val DefaultBottomPaddingSafety = 24f
private const val MinimumBottomPadding = 48f
private const val DefaultExtensionPadding = 40f

internal data class EditorBodyGeometry(
  val pageColumnWidth: Float,
  val visibleBodyRect: EditorVisibleRect,
  val visibleExtensionRect: EditorVisibleRect,
  val minimumBodyHeight: Float,
  val defaultTopPadding: Float,
  val defaultBottomPadding: Float,
  val typewriterBottomPadding: Float,
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
  val defaultBottomPadding =
    max(MinimumBottomPadding, visibleArea.bottomOcclusion + DefaultBottomPaddingSafety)
  val minimumBodyHeight =
    resolveMinimumBodyHeight(
      viewportHeight = visibleArea.viewport.height,
      headerHeight = visibleArea.headerHeight,
      imeInset = visibleArea.imeInset,
      toolbarHeight = visibleArea.toolbarHeight,
    )

  return EditorBodyGeometry(
    pageColumnWidth = layoutPolicy.pageColumnWidth,
    visibleBodyRect = visibleBodyRect,
    visibleExtensionRect = visibleExtensionRect,
    minimumBodyHeight = minimumBodyHeight,
    defaultTopPadding = DefaultExtensionPadding,
    defaultBottomPadding = defaultBottomPadding,
    typewriterBottomPadding =
      defaultBottomPadding, // TODO(editor-parity): Compute cursor-aware typewriter padding once
                           // cursor metrics are available.
  )
}

internal fun resolveMinimumBodyHeight(
  viewportHeight: Float,
  headerHeight: Float,
  imeInset: Float,
  toolbarHeight: Float,
): Float = max(0f, viewportHeight - headerHeight - imeInset - toolbarHeight)
