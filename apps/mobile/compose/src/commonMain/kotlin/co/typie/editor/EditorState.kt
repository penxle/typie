package co.typie.editor

import co.typie.editor.ffi.BlockState
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.ExternalElement
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.Modifier as EditorModifier
import co.typie.editor.ffi.ModifierState
import co.typie.editor.ffi.PlaceholderMetrics
import co.typie.editor.ffi.PlainRootNode
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionEndpoints
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.ffi.TrackedRangeEndpoints

data class EditorState(
  val version: Long,
  val documentRevision: Long = 0L,
  val cursor: CursorMetrics?,
  val placeholder: PlaceholderMetrics? = null,
  val selection: Selection?,
  val selectionEndpoints: SelectionEndpoints? = null,
  val pageSizes: List<Size>,
  val externalElements: List<ExternalElement>,
  val rootAttrs: PlainRootNode?,
  val rootModifiers: List<EditorModifier>?,
  val modifierState: ModifierState? = null,
  val blockState: BlockState? = null,
  val ime: Ime?,
  val trackedRanges: List<TrackedRange> = emptyList(),
  val trackedRangesContainingSelectionHead: List<TrackedRangeEndpoints> = emptyList(),
) {
  companion object {
    val Initial: EditorState =
      EditorState(
        version = 0L,
        documentRevision = 0L,
        cursor = null,
        selection = null,
        pageSizes = emptyList(),
        externalElements = emptyList(),
        rootAttrs = null,
        rootModifiers = null,
        modifierState = null,
        blockState = null,
        ime = null,
      )
  }
}
