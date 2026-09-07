package co.typie.editor.input

import androidx.compose.ui.text.input.BackspaceCommand
import androidx.compose.ui.text.input.CommitTextCommand
import androidx.compose.ui.text.input.DeleteSurroundingTextCommand
import androidx.compose.ui.text.input.DeleteSurroundingTextInCodePointsCommand
import androidx.compose.ui.text.input.EditCommand
import androidx.compose.ui.text.input.EditingBuffer
import androidx.compose.ui.text.input.FinishComposingTextCommand
import androidx.compose.ui.text.input.MoveCursorCommand
import androidx.compose.ui.text.input.SetComposingRegionCommand
import androidx.compose.ui.text.input.SetComposingTextCommand
import androidx.compose.ui.text.input.SetSelectionCommand
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.Key
import co.typie.editor.ffi.KeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NavigationOp
import co.typie.editor.ffi.SelectionOp

internal object EditorImeCommandNormalizer {
  fun normalize(
    commands: List<EditCommand>,
    ime: Ime?,
    documentNavigation: NavigationOp.Move? = null,
  ): List<Message> {
    val selectionMessages = commands.resolveSelectionOnlyMessages(ime)
    if (selectionMessages != null) {
      return if (ime != null && ime.composing == null && documentNavigation != null) {
        listOf(Message.Navigation(documentNavigation))
      } else {
        selectionMessages
      }
    }

    val messages = mutableListOf<Message>()
    val ops = mutableListOf<FlatImeOp>()
    var hasActiveComposition = ime?.composing != null
    val buffer =
      ime?.toTextFieldValue()?.let { value ->
        EditingBuffer(value.annotatedString, value.selection).also { buffer ->
          value.composition?.let { SetComposingRegionCommand(it.min, it.max).applyTo(buffer) }
        }
      }
    fun flushOps() {
      if (ops.isEmpty()) return
      messages += Message.TextInput(ops.toList())
      ops.clear()
    }

    for (command in commands) {
      // Coordinates refer to the text after preceding commands, not the initial IME snapshot.
      val windowText = buffer?.toString().orEmpty()
      buffer?.let(command::applyTo)
      if (command is CommitTextCommand) {
        val text = command.text.replace("\r\n", "\n").replace('\r', '\n')
        if (text == "\n") {
          flushOps()

          messages += Message.Key(KeyEvent(Key.Enter))
          continue
        }
        // The editor has no inline newline: multi-line commits become
        // paragraph splits via the enter key path.
        text.split("\n").forEachIndexed { index, segment ->
          if (index > 0) {
            flushOps()
            messages += Message.Key(KeyEvent(Key.Enter))
          }
          if (segment.isNotEmpty() || index == 0) {
            // commitText replaces an active preedit, but otherwise it is a committed
            // selection replacement and must stay inside the native edit transaction.
            if (hasActiveComposition) {
              ops += FlatImeOp.Compose(segment)
              ops += FlatImeOp.CommitAsIs
            } else {
              ops += FlatImeOp.ReplaceSelection(segment)
            }
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
          command.toFlatImeOp(ime, windowText)
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

    flushOps()

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

    return if (start == end && selection.start == start && selection.end == end) {
      emptyList()
    } else {
      // This is an explicit position, not an instruction to navigate the document.
      listOf(Message.Selection(SelectionOp.SetFlat(start = start, end = end)))
    }
  }

  private fun EditCommand.toFlatImeOp(ime: Ime?, windowText: String): FlatImeOp? =
    when (this) {
      is SetComposingTextCommand -> FlatImeOp.Compose(text)
      is SetSelectionCommand ->
        ime?.let {
          FlatImeOp.SetSelection(
            it.windowStart + windowText.codePointOffsetAtUtf16Index(start),
            it.windowStart + windowText.codePointOffsetAtUtf16Index(end),
          )
        }
      is SetComposingRegionCommand ->
        // InputConnection.setComposingRegion semantics: reversed ranges swap
        // and a zero-length region clears the composition.
        if (start == end) {
          FlatImeOp.ClearComposition
        } else {
          ime?.let {
            FlatImeOp.SetComposition(
              it.windowStart + windowText.codePointOffsetAtUtf16Index(minOf(start, end)),
              it.windowStart + windowText.codePointOffsetAtUtf16Index(maxOf(start, end)),
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
