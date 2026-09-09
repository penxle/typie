package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.PagePoint
import co.typie.editor.ffi.CommandOutcome
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.SelectionPointUnit
import co.typie.editor.interaction.EditorInteractionEffects
import kotlin.concurrent.Volatile

internal class EditorPointSelectionSemantic(private val effects: EditorInteractionEffects) {
  @Volatile private var pendingSelectionGeneration = 0L

  fun cancelPendingSelection() {
    pendingSelectionGeneration += 1
  }

  fun applySelection(editor: Editor, op: SelectionOp): EditorState? {
    val update = editor.updateNow { enqueue(Message.Selection(op)) } ?: return null
    if (update.commandOutcomes.any { it is CommandOutcome.Rejected }) return null
    return update.snapshot
  }

  fun launchSelection(
    editor: Editor,
    op: SelectionOp,
    onApplied: ((EditorState) -> Unit)? = null,
    afterDispatch: (Boolean) -> Unit = {},
  ) = launchSelection(editor, op = { op }, onApplied = onApplied, afterDispatch = afterDispatch)

  fun launchCursorMove(
    editor: Editor,
    point: PagePoint,
    onApplied: ((EditorState) -> Unit)? = null,
    afterDispatch: (Boolean) -> Unit = {},
  ) =
    launchSelection(
      editor = editor,
      op = SelectionOp.SetAt(page = point.page, x = point.x, y = point.y),
      onApplied = onApplied,
      afterDispatch = afterDispatch,
    )

  fun launchSelectionExtension(
    editor: Editor,
    point: PagePoint,
    onApplied: ((EditorState) -> Unit)? = null,
    afterDispatch: (Boolean) -> Unit = {},
  ) =
    launchSelection(
      editor = editor,
      op = { point.selectionExtensionOp(currentSelection = editor.appliedState.selection) },
      onApplied = onApplied,
      afterDispatch = afterDispatch,
    )

  fun launchSelectionExtension(
    editor: Editor,
    point: PagePoint,
    context: EditorSelectionExtensionContext,
    onApplied: ((EditorState) -> Unit)? = null,
    afterDispatch: (Boolean) -> Unit = {},
  ) {
    val op =
      point.selectionExtensionOp(context = context)
        ?: run {
          afterDispatch(false)
          return
        }
    launchSelection(editor = editor, op = op, onApplied = onApplied, afterDispatch = afterDispatch)
  }

  fun launchUnitSelection(
    editor: Editor,
    point: PagePoint,
    unit: SelectionPointUnit,
    onApplied: ((EditorState) -> Unit)? = null,
    afterDispatch: (Boolean) -> Unit = {},
  ) =
    launchSelection(
      editor = editor,
      op = SelectionOp.SelectUnitAt(page = point.page, x = point.x, y = point.y, unit = unit),
      onApplied = onApplied,
      afterDispatch = afterDispatch,
    )

  suspend fun dispatchCursorMove(
    editor: Editor,
    point: PagePoint,
    onApplied: ((EditorState) -> Unit)? = null,
  ): Boolean =
    dispatchSelection(
      editor = editor,
      op = SelectionOp.SetAt(page = point.page, x = point.x, y = point.y),
      onApplied = onApplied,
    )

  suspend fun dispatchSelectionExtension(
    editor: Editor,
    point: PagePoint,
    onApplied: ((EditorState) -> Unit)? = null,
  ): Boolean =
    dispatchSelection(
      editor = editor,
      op = point.selectionExtensionOp(currentSelection = editor.appliedState.selection),
      onApplied = onApplied,
    )

  suspend fun dispatchUnitSelection(
    editor: Editor,
    point: PagePoint,
    unit: SelectionPointUnit,
    onApplied: ((EditorState) -> Unit)? = null,
  ): Boolean =
    dispatchSelection(
      editor = editor,
      op = SelectionOp.SelectUnitAt(page = point.page, x = point.x, y = point.y, unit = unit),
      onApplied = onApplied,
    )

  fun enqueueCursorMove(editor: Editor, point: PagePoint): Boolean {
    return editor.runCallback {
      editor.enqueue(
        Message.Selection(SelectionOp.SetAt(page = point.page, x = point.x, y = point.y))
      )
      true
    } ?: false
  }

  private fun launchSelection(
    editor: Editor,
    op: () -> SelectionOp,
    onApplied: ((EditorState) -> Unit)?,
    afterDispatch: (Boolean) -> Unit,
  ) {
    // Capture before launching: a synchronous mouse selection can overtake this coroutine.
    val generation = pendingSelectionGeneration
    effects.launchInteraction {
      if (generation != pendingSelectionGeneration) return@launchInteraction
      val dispatched = dispatchSelection(editor, op(), onApplied, generation)
      if (generation == pendingSelectionGeneration) afterDispatch(dispatched)
    }
  }

  private suspend fun dispatchSelection(
    editor: Editor,
    op: SelectionOp,
    onApplied: ((EditorState) -> Unit)?,
    generation: Long = pendingSelectionGeneration,
  ): Boolean {
    val update =
      editor.update(admit = { generation == pendingSelectionGeneration }) {
        enqueue(Message.Selection(op))
      } ?: return false
    if (generation != pendingSelectionGeneration) return false
    if (update.commandOutcomes.any { it is CommandOutcome.Rejected }) return false
    onApplied?.invoke(update.snapshot)
    return true
  }
}

private fun PagePoint.selectionExtensionOp(currentSelection: Selection?): SelectionOp =
  currentSelection?.let { selection ->
    SelectionOp.ExtendTo(
      anchor = selection.anchor,
      headPage = page,
      headX = x,
      headY = y,
      baseSelection = null,
      allowCollapse = true,
    )
  } ?: SelectionOp.SetAt(page = page, x = x, y = y)
