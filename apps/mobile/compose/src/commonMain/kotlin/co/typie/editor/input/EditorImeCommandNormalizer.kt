// cspell:ignore DBFF DFFF

package co.typie.editor.input

import androidx.compose.ui.text.input.BackspaceCommand
import androidx.compose.ui.text.input.CommitTextCommand
import androidx.compose.ui.text.input.DeleteSurroundingTextCommand
import androidx.compose.ui.text.input.DeleteSurroundingTextInCodePointsCommand
import androidx.compose.ui.text.input.EditCommand
import androidx.compose.ui.text.input.EditProcessor
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
    if (isEmpty() || any { !it.isSelectionOnly() }) {
      return null
    }
    if (ime == null) {
      return emptyList()
    }

    val processor = EditProcessor()
    val oldValue = ime.toTextFieldValue()
    processor.reset(oldValue, null)
    val newValue = processor.apply(this)
    val start = ime.windowStart + ime.text.codePointOffsetAtUtf16Index(newValue.selection.start)
    val end = ime.windowStart + ime.text.codePointOffsetAtUtf16Index(newValue.selection.end)

    return if (ime.selection.start == ime.selection.end && start == end) {
      val delta = start - ime.selection.start
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

  private fun EditCommand.isSelectionOnly(): Boolean =
    this is SetSelectionCommand || this is MoveCursorCommand

  private fun String.codePointOffsetAtUtf16Index(index: Int): Int {
    var utf16Index = 0
    var codePointOffset = 0
    val target = index.coerceIn(0, length)
    while (utf16Index < target) {
      utf16Index += if (isHighSurrogateAt(utf16Index)) 2 else 1
      codePointOffset += 1
    }
    return codePointOffset
  }

  private fun String.isHighSurrogateAt(index: Int): Boolean =
    this[index] in '\uD800'..'\uDBFF' && index + 1 < length && this[index + 1] in '\uDC00'..'\uDFFF'
}
