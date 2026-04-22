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

  // TODO(editor-parity): 초기 document attrs를 믿지 말고, 에디터 세션이 실제 layout 상태를
  // 권위 있게 노출하면 그 값을 기준으로 layout spec을 잡아야 한다.
  return EditorBodyLayoutPolicy(pageColumnWidth = pageColumnWidth)
}
