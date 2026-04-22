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

internal fun EditorDocumentLayoutSpec.resolveIntrinsicBottomSpace(): Float =
  when (this) {
    is EditorDocumentLayoutSpec.Continuous -> 20f
    is EditorDocumentLayoutSpec.Paginated ->
      40f // TODO(editor-parity): FFI가 페이지 하단 고유 여백을 노출하면 이 고정값을 엔진 값으로
  // 교체해야 한다.
  }
