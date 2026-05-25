package co.typie.editor.input

import androidx.compose.ui.text.input.CommitTextCommand
import androidx.compose.ui.text.input.SetSelectionCommand
import co.typie.editor.ffi.CompositionOp
import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.Key
import co.typie.editor.ffi.KeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Movement
import co.typie.editor.ffi.NavigationOp
import co.typie.editor.ffi.SelectionOp
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorImeCommandNormalizerTest {
  @Test
  fun `commit text normalizes to flat replace selection`() {
    val messages =
      EditorImeCommandNormalizer.normalize(listOf(CommitTextCommand("a", 1)), ime = null)

    assertEquals(
      listOf(Message.Composition(CompositionOp.Flat(listOf(FlatImeOp.ReplaceSelection("a"))))),
      messages,
    )
  }

  @Test
  fun `newline commit normalizes to enter key`() {
    val messages =
      EditorImeCommandNormalizer.normalize(listOf(CommitTextCommand("\n", 1)), ime = null)

    assertEquals(listOf(Message.Key(KeyEvent(Key.Enter))), messages)
  }

  @Test
  fun `collapsed selection command normalizes to navigation delta`() {
    val ime = Ime(text = "hello", windowStart = 10, selection = ImeRange(12, 12), composing = null)

    val messages =
      EditorImeCommandNormalizer.normalize(listOf(SetSelectionCommand(4, 4)), ime = ime)

    assertEquals(
      listOf(
        Message.Navigation(NavigationOp.Move(Movement.Grapheme(Direction.Forward), false)),
        Message.Navigation(NavigationOp.Move(Movement.Grapheme(Direction.Forward), false)),
      ),
      messages,
    )
  }

  @Test
  fun `range selection command normalizes to flat selection set`() {
    val ime =
      Ime(text = "a\uD83D\uDE00b", windowStart = 20, selection = ImeRange(20, 20), composing = null)

    val messages =
      EditorImeCommandNormalizer.normalize(listOf(SetSelectionCommand(0, 3)), ime = ime)

    assertEquals(listOf(Message.Selection(SelectionOp.SetFlat(start = 20, end = 22))), messages)
  }

  @Test
  fun `range selection command remains absolute selection in common normalizer`() {
    val text = "abcdefghijklmnopqrst"

    assertEquals(
      listOf(Message.Selection(SelectionOp.SetFlat(start = 15, end = 18))),
      EditorImeCommandNormalizer.normalize(
        listOf(SetSelectionCommand(15, 18)),
        ime = Ime(text = text, windowStart = 0, selection = ImeRange(16, 18), composing = null),
      ),
    )

    assertEquals(
      listOf(Message.Selection(SelectionOp.SetFlat(start = 12, end = 18))),
      EditorImeCommandNormalizer.normalize(
        listOf(SetSelectionCommand(12, 18)),
        ime = Ime(text = text, windowStart = 0, selection = ImeRange(11, 18), composing = null),
      ),
    )
  }
}
