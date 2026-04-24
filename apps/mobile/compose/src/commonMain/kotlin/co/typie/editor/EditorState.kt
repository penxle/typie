package co.typie.editor

import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.DocumentAttrs
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.Size

data class EditorState(
  val version: Long,
  val cursor: CursorMetrics?,
  val selection: Selection?,
  val pageSizes: List<Size>,
  val documentAttrs: DocumentAttrs?,
  val ime: Ime?,
) {
  companion object {
    val Initial: EditorState =
      EditorState(
        version = 0L,
        cursor = null,
        selection = null,
        pageSizes = emptyList(),
        documentAttrs = null,
        ime = null,
      )
  }
}
