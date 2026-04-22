package co.typie.editor.body

import androidx.compose.ui.geometry.Size
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.scroll.CursorVisibleMargin
import co.typie.editor.scroll.EditorScrollMode
import co.typie.editor.scroll.EditorScrollPolicy
import co.typie.editor.scroll.resolveEditorScrollPolicy
import kotlin.math.max
import kotlin.math.min

private const val DefaultExtensionPadding = 40f

internal data class EditorBodyGeometry(
  val pageColumnWidth: Float,
  val visibleBodySize: Size,
  val visibleExtensionSize: Size,
  val minimumBodyHeight: Float,
  val defaultTopPadding: Float,
  val defaultBottomPadding: Float,
  val activeBottomPadding: Float,
  val scrollPolicy: EditorScrollPolicy,
)

internal fun resolveEditorBodyGeometry(
  visibleArea: EditorVisibleArea,
  layoutSpec: EditorDocumentLayoutSpec,
  pageSizes: List<PageSize>,
  typewriterEnabled: Boolean = false,
  typewriterPosition: Float = 0.5f,
  cursorHeight: Float = 0f,
): EditorBodyGeometry {
  val visibleBodySize = visibleArea.visibleBodySize
  val visibleExtensionSize = visibleArea.visibleExtensionSize
  val maxPageWidth = pageSizes.maxOfOrNull(PageSize::width) ?: 0f
  val preferredPageWidth =
    when (layoutSpec) {
      is EditorDocumentLayoutSpec.Continuous -> layoutSpec.maxWidth
      is EditorDocumentLayoutSpec.Paginated -> layoutSpec.pageWidth
    }
  val pageColumnWidth =
    when {
      preferredPageWidth > 0f && visibleExtensionSize.width > 0f ->
        min(preferredPageWidth, visibleExtensionSize.width)
      preferredPageWidth > 0f -> preferredPageWidth
      maxPageWidth > 0f && visibleExtensionSize.width > 0f ->
        min(maxPageWidth, visibleExtensionSize.width)
      maxPageWidth > 0f -> maxPageWidth
      else -> visibleExtensionSize.width
    }
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
    pageColumnWidth = pageColumnWidth,
    visibleBodySize = visibleBodySize,
    visibleExtensionSize = visibleExtensionSize,
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
