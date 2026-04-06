package co.typie.editor.compose

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
import co.typie.editor.Editor
import co.typie.editor.ffi.CompositionIntent
import co.typie.editor.ffi.DeletionIntent
import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.Intent
import co.typie.editor.ffi.Key
import co.typie.editor.ffi.KeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Movement
import co.typie.editor.ffi.NavigationIntent
import co.typie.editor.ffi.SelectionIntent

internal object EditorEditCommandHandler {
  fun handle(editor: Editor, commands: List<EditCommand>) {
    for (command in commands) {
      dispatch(editor, command)
    }
  }

  private fun dispatch(editor: Editor, command: EditCommand) {
    when (command) {
      is CommitTextCommand -> {
        if (command.text == "\n") {
          editor.enqueue(Message.Key(KeyEvent(Key.Enter)))
        } else {
          editor.enqueue(Message.Intent(Intent.Composition(CompositionIntent.Commit(command.text))))
        }
      }
      is SetComposingTextCommand -> {
        editor.enqueue(
          Message.Intent(Intent.Composition(CompositionIntent.Update(command.text, null)))
        )
      }
      is SetComposingRegionCommand -> {
        editor.enqueue(
          Message.Intent(Intent.Composition(
            CompositionIntent.SetRegion(command.start, command.end)
          ))
        )
      }
      is FinishComposingTextCommand -> {
        editor.enqueue(Message.Intent(Intent.Composition(CompositionIntent.CommitAsIs)))
      }
      is DeleteSurroundingTextCommand -> {
        editor.enqueue(
          Message.Intent(Intent.Deletion(
            DeletionIntent.Surrounding(command.lengthBeforeCursor, command.lengthAfterCursor)
          ))
        )
      }
      is DeleteSurroundingTextInCodePointsCommand -> {
        editor.enqueue(
          Message.Intent(Intent.Deletion(
            DeletionIntent.SurroundingCodePoints(command.lengthBeforeCursor, command.lengthAfterCursor)
          ))
        )
      }
      is SetSelectionCommand -> {
        editor.enqueue(
          Message.Intent(Intent.Selection(
            SelectionIntent.SetFlat(command.start, command.end)
          ))
        )
      }
      is BackspaceCommand -> {
        editor.enqueue(Message.Key(KeyEvent(Key.Backspace)))
      }
      is MoveCursorCommand -> {
        // TODO(future work): amount's magnitude is currently ignored.
        // NavigationIntent.Move lacks a count field, so we emit a single
        // grapheme step in the appropriate direction. In practice IMEs send
        // ±1, so this is usually correct. Proper fix: add `count: usize` to
        // NavigationIntent::Move in Rust and forward `abs(command.amount)`.
        val direction = if (command.amount >= 0) Direction.Forward else Direction.Backward
        editor.enqueue(
          Message.Intent(Intent.Navigation(
            NavigationIntent.Move(Movement.Grapheme(direction), extend = false)
          ))
        )
      }
      else -> {
        // EditCommand is an open interface; Compose or CMP may introduce subtypes we don't handle.
        // Dropping them is safe because the Skiko text input machinery is pull-based — it
        // re-reads editor state via request.value on subsequent queries and reconciles its own model.
      }
    }
  }
}
