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
    var hasActiveComposition = ime?.composing != null

    for (command in commands) {
      if (command is CommitTextCommand) {
        val text = command.text.replace("\r\n", "\n").replace('\r', '\n')
        if (text == "\n") {
          if (ops.isNotEmpty()) {
            messages += Message.TextInput(ops.toList())
            ops.clear()
          }

          messages += Message.Key(KeyEvent(Key.Enter))
          continue
        }
        // The editor has no inline newline: multi-line commits become
        // paragraph splits via the enter key path.
        text.split("\n").forEachIndexed { index, segment ->
          if (index > 0) {
            if (ops.isNotEmpty()) {
              messages += Message.TextInput(ops.toList())
              ops.clear()
            }
            messages += Message.Key(KeyEvent(Key.Enter))
          }
          if (segment.isNotEmpty() || index == 0) {
            ops += FlatImeOp.Compose(segment)
            ops += FlatImeOp.CommitAsIs
            hasActiveComposition = false
          }
        }
        continue
      }

      val op =
        if (command is FinishComposingTextCommand) {
          if (hasActiveComposition) {
            FlatImeOp.CommitAsIs
          } else {
            FlatImeOp.ClearComposition
          }
        } else {
          command.toFlatImeOp(ime)
        } ?: continue
      ops += op
      hasActiveComposition =
        when (op) {
          is FlatImeOp.Compose,
          is FlatImeOp.SetComposition -> true
          is FlatImeOp.ClearComposition,
          is FlatImeOp.CommitAsIs -> false
          else -> hasActiveComposition
        }
    }

    if (ops.isNotEmpty()) {
      messages += Message.TextInput(ops)
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

  private fun EditCommand.toFlatImeOp(ime: Ime?): FlatImeOp? =
    when (this) {
      is SetComposingTextCommand -> FlatImeOp.Compose(text)
      is SetSelectionCommand ->
        ime?.let {
          FlatImeOp.SetSelection(it.projectWindowUtf16Index(start), it.projectWindowUtf16Index(end))
        }
      is SetComposingRegionCommand ->
        // InputConnection.setComposingRegion semantics: reversed ranges swap
        // and a zero-length region clears the composition.
        if (start == end) {
          FlatImeOp.ClearComposition
        } else {
          ime?.let {
            FlatImeOp.SetComposition(
              it.projectWindowUtf16Index(minOf(start, end)),
              it.projectWindowUtf16Index(maxOf(start, end)),
            )
          }
        }
      is DeleteSurroundingTextCommand ->
        FlatImeOp.DeleteSurroundingUtf16(lengthBeforeCursor, lengthAfterCursor)

      is DeleteSurroundingTextInCodePointsCommand ->
        FlatImeOp.DeleteSurrounding(lengthBeforeCursor, lengthAfterCursor)

      is BackspaceCommand -> FlatImeOp.DeleteSurrounding(1, 0)
      is MoveCursorCommand -> FlatImeOp.MoveCursor(amount)
      else -> null
    }
}
