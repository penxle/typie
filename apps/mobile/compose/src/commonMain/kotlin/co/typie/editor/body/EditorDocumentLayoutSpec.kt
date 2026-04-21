package co.typie.editor.body

import co.typie.editor.ffi.LayoutMode

internal sealed interface EditorDocumentLayoutSpec {
  data class Continuous(val maxWidth: Float) : EditorDocumentLayoutSpec

  data class Paginated(val pageWidth: Float) : EditorDocumentLayoutSpec
}

internal fun LayoutMode.toEditorDocumentLayoutSpec(): EditorDocumentLayoutSpec =
  when (this) {
    is LayoutMode.Continuous -> EditorDocumentLayoutSpec.Continuous(maxWidth = maxWidth)
    is LayoutMode.Paginated -> EditorDocumentLayoutSpec.Paginated(pageWidth = pageWidth)
  }
