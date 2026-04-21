package co.typie.editor.body

import co.typie.editor.ffi.Size
import kotlin.math.min

internal data class EditorBodyLayoutPolicy(val pageColumnWidth: Float)

internal fun resolveEditorBodyLayoutPolicy(
  availableBodyWidth: Float,
  layoutSpec: EditorDocumentLayoutSpec,
  pageSizes: List<Size>,
): EditorBodyLayoutPolicy {
  val maxPageWidth = pageSizes.maxOfOrNull(Size::width) ?: 0f
  val preferredColumnWidth =
    when (layoutSpec) {
      is EditorDocumentLayoutSpec.Continuous -> layoutSpec.maxWidth
      is EditorDocumentLayoutSpec.Paginated -> layoutSpec.pageWidth
    }
  val pageColumnWidth =
    when {
      preferredColumnWidth > 0f && availableBodyWidth > 0f ->
        min(preferredColumnWidth, availableBodyWidth)
      preferredColumnWidth > 0f -> preferredColumnWidth
      maxPageWidth > 0f && availableBodyWidth > 0f -> min(maxPageWidth, availableBodyWidth)
      maxPageWidth > 0f -> maxPageWidth
      else -> availableBodyWidth
    }

  // TODO(editor-parity): Source this layout spec from the live engine/runtime once the editor
  // session exposes authoritative layout state instead of relying on the initial document attrs.
  return EditorBodyLayoutPolicy(pageColumnWidth = pageColumnWidth)
}
