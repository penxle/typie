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
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Ime
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

    val ops = mutableListOf<FlatImeOp>()
    var hasActiveComposition = ime?.composing != null
    val buffer = ime?.toEditProcessor()
    for (command in commands) {
      // Coordinates refer to the text after preceding commands, not the initial IME snapshot.
      val windowText = buffer?.toTextFieldValue()?.text.orEmpty()
      val valueAfter = buffer?.apply(listOf(command))
      if (command is CommitTextCommand) {
        // Keep the native text (including CR/LF) intact. Later coordinates
        // address this buffer; only the engine knows the resulting paragraphs.
        if (hasActiveComposition) {
          ops += FlatImeOp.Compose(command.text)
          ops += FlatImeOp.CommitAsIs
        } else {
          ops += FlatImeOp.ReplaceSelection(command.text)
        }
        hasActiveComposition = false
        if (
          command.newCursorPosition != 1 &&
            !(command.newCursorPosition == 0 && command.text.isEmpty()) &&
            valueAfter != null
        ) {
          val text = valueAfter.text
          ops +=
            FlatImeOp.SetSelection(
              ime.windowStart + text.codePointOffsetAtUtf16Index(valueAfter.selection.min),
              ime.windowStart + text.codePointOffsetAtUtf16Index(valueAfter.selection.max),
            )
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
      if (
        command is SetComposingTextCommand && command.newCursorPosition != 1 && valueAfter != null
      ) {
        val text = valueAfter.text
        ops +=
          FlatImeOp.SetSelection(
            ime.windowStart + text.codePointOffsetAtUtf16Index(valueAfter.selection.min),
            ime.windowStart + text.codePointOffsetAtUtf16Index(valueAfter.selection.max),
          )
      }
      hasActiveComposition =
        valueAfter?.let { it.composition != null }
          ?: when (op) {
            is FlatImeOp.Compose -> op.text.isNotEmpty()
            is FlatImeOp.SetComposition -> op.start != op.end
            is FlatImeOp.ClearComposition,
            is FlatImeOp.CommitAsIs -> false
            else -> hasActiveComposition
          }
    }

    return if (ops.isEmpty()) emptyList() else listOf(Message.TextInput(ops))
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
            val from = it.windowStart + windowText.codePointOffsetAtUtf16Index(minOf(start, end))
            val to = it.windowStart + windowText.codePointOffsetAtUtf16Index(maxOf(start, end))
            if (from == to) FlatImeOp.ClearComposition else FlatImeOp.SetComposition(from, to)
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
