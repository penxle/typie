package co.typie.editor.input

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
import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.Key
import co.typie.editor.ffi.KeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Movement
import co.typie.editor.ffi.NavigationOp
import co.typie.editor.ffi.SelectionOp
import kotlin.math.abs

internal object EditorImeCommandNormalizer {
  fun normalize(commands: List<EditCommand>, ime: Ime?): List<Message> {
    val selectionMessages = commands.resolveSelectionOnlyMessages(ime)
    if (selectionMessages != null) {
      return selectionMessages
    }

    val messages = mutableListOf<Message>()
    val ops = mutableListOf<FlatImeOp>()

    for (command in commands) {
      if (command is CommitTextCommand && command.text == "\n") {
        if (ops.isNotEmpty()) {
          messages += Message.Composition(CompositionOp.Flat(ops.toList()))
          ops.clear()
        }

        messages += Message.Key(KeyEvent(Key.Enter))
        continue
      }

      val op = command.toFlatImeOp() ?: continue
      ops += op
    }

    if (ops.isNotEmpty()) {
      messages += Message.Composition(CompositionOp.Flat(ops))
    }

    return messages
  }

  private fun List<EditCommand>.resolveSelectionOnlyMessages(ime: Ime?): List<Message>? {
    val target =
      when (val projection = projectSelectionOnlyCommand(ime)) {
        null -> return null
        SelectionOnlyEditCommandProjection.MissingIme -> return emptyList()
        is SelectionOnlyEditCommandProjection.Target -> projection.range
      }
    val selection = ime?.selection ?: return emptyList()
    val start = target.start
    val end = target.end

    return if (selection.start == selection.end && start == end) {
      val delta = start - selection.start
      if (delta == 0) {
        emptyList()
      } else {
        val direction = if (delta > 0) Direction.Forward else Direction.Backward
        List(abs(delta)) {
          Message.Navigation(NavigationOp.Move(Movement.Grapheme(direction), false))
        }
      }
    } else {
      listOf(Message.Selection(SelectionOp.SetFlat(start = start, end = end)))
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
