package co.typie.editor.input

import androidx.compose.ui.text.input.CommitTextCommand
import androidx.compose.ui.text.input.FinishComposingTextCommand
import androidx.compose.ui.text.input.SetComposingRegionCommand
import androidx.compose.ui.text.input.SetComposingTextCommand
import androidx.compose.ui.text.input.SetSelectionCommand
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
  fun `commit text normalizes to composition replacement and commit`() {
    val messages =
      EditorImeCommandNormalizer.normalize(listOf(CommitTextCommand("a", 1)), ime = null)

    assertEquals(
      listOf(Message.TextInput(listOf(FlatImeOp.Compose("a"), FlatImeOp.CommitAsIs))),
      messages,
    )
  }

  @Test
  fun `commit text during active preedit replaces composition`() {
    val ime =
      Ime(text = "안", windowStart = 0, selection = ImeRange(1, 1), composing = ImeRange(0, 1))
    val messages =
      EditorImeCommandNormalizer.normalize(listOf(CommitTextCommand("안녕하세요", 1)), ime = ime)

    assertEquals(
      listOf(Message.TextInput(listOf(FlatImeOp.Compose("안녕하세요"), FlatImeOp.CommitAsIs))),
      messages,
    )
  }

  @Test
  fun `autocomplete selection batch replaces active preedit before trailing commit`() {
    val ime =
      Ime(text = " 안 ", windowStart = 0, selection = ImeRange(2, 2), composing = ImeRange(1, 2))
    val messages =
      EditorImeCommandNormalizer.normalize(
        listOf(
          CommitTextCommand("안녕하세요", 1),
          FinishComposingTextCommand(),
          CommitTextCommand(" ", 1),
        ),
        ime = ime,
      )

    assertEquals(
      listOf(
        Message.TextInput(
          listOf(
            FlatImeOp.Compose("안녕하세요"),
            FlatImeOp.CommitAsIs,
            FlatImeOp.ClearComposition,
            FlatImeOp.Compose(" "),
            FlatImeOp.CommitAsIs,
          )
        )
      ),
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
  fun `finish composing command clears composition without active preedit`() {
    val messages =
      EditorImeCommandNormalizer.normalize(listOf(FinishComposingTextCommand()), ime = null)

    assertEquals(listOf(Message.TextInput(listOf(FlatImeOp.ClearComposition))), messages)
  }

  @Test
  fun `finish composing command commits active preedit as-is`() {
    val ime =
      Ime(text = "ㅎ", windowStart = 0, selection = ImeRange(1, 1), composing = ImeRange(0, 1))
    val messages =
      EditorImeCommandNormalizer.normalize(listOf(FinishComposingTextCommand()), ime = ime)

    assertEquals(listOf(Message.TextInput(listOf(FlatImeOp.CommitAsIs))), messages)
  }

  @Test
  fun `finish composing command commits preedit started in same command batch`() {
    val messages =
      EditorImeCommandNormalizer.normalize(
        listOf(SetComposingTextCommand("ㅎ", 1), FinishComposingTextCommand()),
        ime = null,
      )

    assertEquals(
      listOf(Message.TextInput(listOf(FlatImeOp.Compose("ㅎ"), FlatImeOp.CommitAsIs))),
      messages,
    )
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
  fun `mixed selection and commit batch projects window-relative offsets to absolute`() {
    val ime =
      Ime(
        text = "텐데. ㅎㅇ",
        windowStart = 4559,
        selection = ImeRange(4565, 4565),
        composing = null,
      )

    val messages =
      EditorImeCommandNormalizer.normalize(
        listOf(
          SetSelectionCommand(4, 6),
          CommitTextCommand("", 0),
          CommitTextCommand("ㅎ", 1),
          CommitTextCommand("아", 1),
        ),
        ime = ime,
      )

    assertEquals(
      listOf(
        Message.TextInput(
          listOf(
            FlatImeOp.SetSelection(4563, 4565),
            FlatImeOp.Compose(""),
            FlatImeOp.CommitAsIs,
            FlatImeOp.Compose("ㅎ"),
            FlatImeOp.CommitAsIs,
            FlatImeOp.Compose("아"),
            FlatImeOp.CommitAsIs,
          )
        )
      ),
      messages,
    )
  }

  @Test
  fun `mixed batch selection offsets convert utf16 indices to code point offsets`() {
    val ime =
      Ime(
        text = "a😀b",
        windowStart = 10,
        selection = ImeRange(13, 13),
        composing = null,
      )

    val messages =
      EditorImeCommandNormalizer.normalize(
        listOf(SetSelectionCommand(1, 3), CommitTextCommand("x", 1)),
        ime = ime,
      )

    assertEquals(
      listOf(
        Message.TextInput(
          listOf(FlatImeOp.SetSelection(11, 12), FlatImeOp.Compose("x"), FlatImeOp.CommitAsIs)
        )
      ),
      messages,
    )
  }

  @Test
  fun `mixed batch composing region projects window-relative offsets to absolute`() {
    val ime =
      Ime(
        text = "텐데. ㅎㅇ",
        windowStart = 4559,
        selection = ImeRange(4565, 4565),
        composing = null,
      )

    val messages =
      EditorImeCommandNormalizer.normalize(
        listOf(SetComposingRegionCommand(4, 6), SetComposingTextCommand("화", 1)),
        ime = ime,
      )

    assertEquals(
      listOf(
        Message.TextInput(listOf(FlatImeOp.SetComposition(4563, 4565), FlatImeOp.Compose("화")))
      ),
      messages,
    )
  }

  @Test
  fun `zero-length composing region clears composition`() {
    val ime = Ime(text = "가나다", windowStart = 0, selection = ImeRange(3, 3), composing = null)

    val messages =
      EditorImeCommandNormalizer.normalize(
        listOf(SetComposingRegionCommand(0, 0), SetComposingTextCommand("라", 1)),
        ime = ime,
      )

    assertEquals(
      listOf(Message.TextInput(listOf(FlatImeOp.ClearComposition, FlatImeOp.Compose("라")))),
      messages,
    )
  }

  @Test
  fun `reversed composing region swaps to ordered range`() {
    val ime = Ime(text = "가나다", windowStart = 0, selection = ImeRange(3, 3), composing = null)

    val messages =
      EditorImeCommandNormalizer.normalize(
        listOf(SetComposingRegionCommand(3, 1), SetComposingTextCommand("라", 1)),
        ime = ime,
      )

    assertEquals(
      listOf(Message.TextInput(listOf(FlatImeOp.SetComposition(1, 3), FlatImeOp.Compose("라")))),
      messages,
    )
  }

  @Test
  fun `mixed batch selection command without ime snapshot is dropped`() {
    val messages =
      EditorImeCommandNormalizer.normalize(
        listOf(SetSelectionCommand(0, 2), CommitTextCommand("x", 1)),
        ime = null,
      )

    assertEquals(
      listOf(Message.TextInput(listOf(FlatImeOp.Compose("x"), FlatImeOp.CommitAsIs))),
      messages,
    )
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
