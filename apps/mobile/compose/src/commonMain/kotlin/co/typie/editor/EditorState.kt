package co.typie.editor

import co.typie.editor.ffi.BlockState
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ModifierState
import co.typie.editor.ffi.PlainRootNode
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.Size

data class EditorState(
  val version: Long,
  val cursor: CursorMetrics?,
  val selection: Selection?,
  val pageSizes: List<Size>,
  val rootAttrs: PlainRootNode?,
  val modifierState: ModifierState? = null,
  val blockState: BlockState? = null,
  val ime: Ime?,
) {
  companion object {
    val Initial: EditorState =
      EditorState(
        version = 0L,
        cursor = null,
        selection = null,
        pageSizes = emptyList(),
        rootAttrs = null,
        modifierState = null,
        blockState = null,
        ime = null,
      )
  }
}
