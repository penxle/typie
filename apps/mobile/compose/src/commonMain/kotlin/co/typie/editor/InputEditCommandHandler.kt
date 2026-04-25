package co.typie.editor

import androidx.compose.ui.text.input.BackspaceCommand
import androidx.compose.ui.text.input.CommitTextCommand
import androidx.compose.ui.text.input.DeleteSurroundingTextCommand
import androidx.compose.ui.text.input.DeleteSurroundingTextInCodePointsCommand
import androidx.compose.ui.text.input.EditCommand
import androidx.compose.ui.text.input.FinishComposingTextCommand
import androidx.compose.ui.text.input.MoveCursorCommand
import androidx.compose.ui.text.input.SetComposingRegionCommand
import androidx.compose.ui.text.input.SetComposingTextCommand
import androidx.compose.ui.text.input.SetSelectionCommand
import co.typie.editor.ffi.CompositionOp
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Key
import co.typie.editor.ffi.KeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.syncWithBringIntoView

internal object InputEditCommandHandler {
  fun handle(
    editor: Editor,
    bringIntoViewRequests: EditorBringIntoViewRequests,
    commands: List<EditCommand>,
  ) {
    editor.syncWithBringIntoView(bringIntoViewRequests) {
      val ops = mutableListOf<FlatImeOp>()

      for (command in commands) {
        if (command is CommitTextCommand && command.text == "\n") {
          if (ops.isNotEmpty()) {
            enqueue(Message.Composition(CompositionOp.Flat(ops.toList())))
            ops.clear()
          }

          enqueue(Message.Key(KeyEvent(Key.Enter)))
          continue
        }

        val op = command.toFlatImeOp() ?: continue
        ops.add(op)
      }

      if (ops.isNotEmpty()) {
        enqueue(Message.Composition(CompositionOp.Flat(ops)))
      }

      beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentCursorLine) }
    }
  }

  private fun EditCommand.toFlatImeOp(): FlatImeOp? =
    when (this) {
      is CommitTextCommand -> FlatImeOp.ReplaceSelection(text)
      is SetComposingTextCommand -> FlatImeOp.Compose(text)
      is SetSelectionCommand -> FlatImeOp.SetSelection(start, end)
      is SetComposingRegionCommand -> FlatImeOp.SetComposition(start, end)
      is FinishComposingTextCommand -> FlatImeOp.ClearComposition
      is DeleteSurroundingTextCommand ->
        FlatImeOp.DeleteSurroundingUtf16(lengthBeforeCursor, lengthAfterCursor)

      is DeleteSurroundingTextInCodePointsCommand ->
        FlatImeOp.DeleteSurrounding(lengthBeforeCursor, lengthAfterCursor)

      is BackspaceCommand -> FlatImeOp.DeleteSurrounding(1, 0)
      is MoveCursorCommand -> FlatImeOp.MoveCursor(amount)
      else -> null
    }
}
