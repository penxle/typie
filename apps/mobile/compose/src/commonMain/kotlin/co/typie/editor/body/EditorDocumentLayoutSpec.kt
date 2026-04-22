package co.typie.editor.body

import co.typie.editor.ffi.LayoutMode

internal sealed interface EditorDocumentLayoutSpec {
  data class Continuous(val maxWidth: Float) : EditorDocumentLayoutSpec

  data class Paginated(
    val pageWidth: Float,
    val pageHeight: Float,
    val pageMarginTop: Float,
    val pageMarginBottom: Float,
    val pageMarginLeft: Float,
    val pageMarginRight: Float,
  ) : EditorDocumentLayoutSpec
}

internal fun LayoutMode.toEditorDocumentLayoutSpec(): EditorDocumentLayoutSpec =
  when (this) {
    is LayoutMode.Continuous -> EditorDocumentLayoutSpec.Continuous(maxWidth = maxWidth)
    is LayoutMode.Paginated ->
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = pageWidth,
        pageHeight = pageHeight,
        pageMarginTop = pageMarginTop,
        pageMarginBottom = pageMarginBottom,
        pageMarginLeft = pageMarginLeft,
        pageMarginRight = pageMarginRight,
      )
  }

internal fun EditorDocumentLayoutSpec.resolveIntrinsicBottomSpace(): Float =
  when (this) {
    is EditorDocumentLayoutSpec.Continuous -> 20f
    is EditorDocumentLayoutSpec.Paginated -> pageMarginBottom
  }
