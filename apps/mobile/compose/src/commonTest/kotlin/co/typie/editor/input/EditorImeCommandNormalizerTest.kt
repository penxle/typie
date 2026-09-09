package co.typie.editor.input

import androidx.compose.ui.text.input.CommitTextCommand
import androidx.compose.ui.text.input.DeleteSurroundingTextInCodePointsCommand
import androidx.compose.ui.text.input.FinishComposingTextCommand
import androidx.compose.ui.text.input.SetComposingRegionCommand
import androidx.compose.ui.text.input.SetComposingTextCommand
import androidx.compose.ui.text.input.SetSelectionCommand
import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Movement
import co.typie.editor.ffi.NavigationOp
import co.typie.editor.ffi.SelectionOp
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorImeCommandNormalizerTest {
  @Test
  fun `commit cursor position is retained before a later edit`() {
    val ime =
      Ime(text = "\u2028\u2029", windowStart = 0, selection = ImeRange(1, 1), composing = null)
    val messages =
      EditorImeCommandNormalizer.normalize(
        listOf(CommitTextCommand("a\nb", 0), CommitTextCommand("X", 1)),
        ime,
      )
    assertEquals(
      listOf(
        Message.TextInput(
          listOf(
            FlatImeOp.ReplaceSelection("a\nb"),
            FlatImeOp.SetSelection(1, 1),
            FlatImeOp.ReplaceSelection("X"),
          )
        )
      ),
      messages,
    )
  }

  @Test
  fun `selection after marked text uses the new text length`() {
    val ime = Ime(text = "", windowStart = 10, selection = ImeRange(10, 10), composing = null)

    val messages =
      EditorImeCommandNormalizer.normalize(
        listOf(SetComposingTextCommand("にほん", 1), SetSelectionCommand(2, 2)),
        ime,
      )

    assertEquals(
      listOf(Message.TextInput(listOf(FlatImeOp.Compose("にほん"), FlatImeOp.SetSelection(12, 12)))),
      messages,
    )
  }

  @Test
  fun `selection and composing region follow changed surrogate pairs within a batch`() {
    val ime =
      Ime(
        text = "a😀Z",
        windowStart = 10,
        selection = ImeRange(12, 12),
        composing = ImeRange(11, 12),
      )

    val messages =
      EditorImeCommandNormalizer.normalize(
        listOf(
          SetComposingTextCommand("にほん", 1),
          SetSelectionCommand(2, 3),
          SetComposingTextCommand("😀ほ", 1),
          SetSelectionCommand(3, 4),
          FinishComposingTextCommand(),
          SetComposingRegionCommand(1, 3),
        ),
        ime,
      )

    assertEquals(
      listOf(
        Message.TextInput(
          listOf(
            FlatImeOp.Compose("にほん"),
            FlatImeOp.SetSelection(12, 13),
            FlatImeOp.Compose("😀ほ"),
            FlatImeOp.SetSelection(12, 13),
            FlatImeOp.CommitAsIs,
            FlatImeOp.SetComposition(11, 12),
          )
        )
      ),
      messages,
    )
  }

  @Test
  fun `commit text without active preedit replaces selection`() {
    val messages =
      EditorImeCommandNormalizer.normalize(listOf(CommitTextCommand("a", 1)), ime = null)

    assertEquals(listOf(Message.TextInput(listOf(FlatImeOp.ReplaceSelection("a")))), messages)
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
            FlatImeOp.ReplaceSelection(" "),
          )
        )
      ),
      messages,
    )
  }

  @Test
  fun `newline commit remains inside the input batch`() {
    val messages =
      EditorImeCommandNormalizer.normalize(listOf(CommitTextCommand("\n", 1)), ime = null)

    assertEquals(listOf(Message.TextInput(listOf(FlatImeOp.ReplaceSelection("\n")))), messages)
  }

  @Test
  fun `multi-line commit preserves the input text for the engine`() {
    val messages =
      EditorImeCommandNormalizer.normalize(listOf(CommitTextCommand("foo\nbar", 1)), ime = null)

    assertEquals(
      listOf(Message.TextInput(listOf(FlatImeOp.ReplaceSelection("foo\nbar")))),
      messages,
    )
  }

  @Test
  fun `multi-line commit keeps carriage returns in the coordinate buffer`() {
    val messages =
      EditorImeCommandNormalizer.normalize(listOf(CommitTextCommand("a\r\n\rb", 1)), ime = null)

    assertEquals(
      listOf(Message.TextInput(listOf(FlatImeOp.ReplaceSelection("a\r\n\rb")))),
      messages,
    )
  }

  @Test
  fun `multi-line commit with leading newline still replaces active preedit`() {
    val ime =
      Ime(text = "가", windowStart = 0, selection = ImeRange(1, 1), composing = ImeRange(0, 1))
    val messages =
      EditorImeCommandNormalizer.normalize(listOf(CommitTextCommand("\nfoo", 1)), ime = ime)

    assertEquals(
      listOf(Message.TextInput(listOf(FlatImeOp.Compose("\nfoo"), FlatImeOp.CommitAsIs))),
      messages,
    )
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
  fun `finish composition stays ordered before following committed text`() {
    val ime =
      Ime(text = "ㅜㅜ", windowStart = 0, selection = ImeRange(2, 2), composing = ImeRange(1, 2))
    val messages =
      EditorImeCommandNormalizer.normalize(
        listOf(FinishComposingTextCommand(), CommitTextCommand(" ", 1)),
        ime = ime,
      )

    assertEquals(
      listOf(Message.TextInput(listOf(FlatImeOp.CommitAsIs, FlatImeOp.ReplaceSelection(" ")))),
      messages,
    )
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
  fun `delete and composing commands stay in one ordered text input batch`() {
    val messages =
      EditorImeCommandNormalizer.normalize(
        listOf(DeleteSurroundingTextInCodePointsCommand(1, 0), SetComposingTextCommand("하", 1)),
        ime = null,
      )

    assertEquals(
      listOf(Message.TextInput(listOf(FlatImeOp.DeleteSurrounding(1, 0), FlatImeOp.Compose("하")))),
      messages,
    )
  }

  @Test
  fun `collapsed selection command preserves its absolute target`() {
    val ime = Ime(text = "hello", windowStart = 10, selection = ImeRange(12, 12), composing = null)

    val messages =
      EditorImeCommandNormalizer.normalize(listOf(SetSelectionCommand(4, 4)), ime = ime)

    assertEquals(listOf(Message.Selection(SelectionOp.SetFlat(start = 14, end = 14))), messages)
  }

  @Test
  fun `document navigation produces one grapheme movement regardless of native offset distance`() {
    val ime =
      Ime(text = "a👩‍💻bc", windowStart = 10, selection = ImeRange(11, 11), composing = null)
    val movement = NavigationOp.Move(Movement.Grapheme(Direction.Forward), false)

    assertEquals(
      listOf(Message.Navigation(movement)),
      EditorImeCommandNormalizer.normalize(
        listOf(SetSelectionCommand(6, 6)),
        ime,
        documentNavigation = movement,
      ),
    )
  }

  @Test
  fun `document navigation preserves shift direction instead of replacing ordered endpoints`() {
    val ime = Ime(text = "ab\n\ncd", windowStart = 0, selection = ImeRange(2, 6), composing = null)
    for (extend in listOf(true, false)) {
      val movement = NavigationOp.Move(Movement.Grapheme(Direction.Forward), extend)
      assertEquals(
        listOf(Message.Navigation(movement)),
        EditorImeCommandNormalizer.normalize(
          listOf(SetSelectionCommand(3, 6)),
          ime,
          documentNavigation = movement,
        ),
      )
    }
  }

  @Test
  fun `marked and mixed edits do not become document navigation`() {
    val movement = NavigationOp.Move(Movement.Grapheme(Direction.Backward), false)
    val ime =
      Ime(
        text = "にほん",
        windowStart = 10,
        selection = ImeRange(13, 13),
        composing = ImeRange(10, 13),
      )
    assertEquals(
      listOf(Message.Selection(SelectionOp.SetFlat(12, 12))),
      EditorImeCommandNormalizer.normalize(
        listOf(SetSelectionCommand(2, 2)),
        ime,
        documentNavigation = movement,
      ),
    )
    assertEquals(
      listOf(
        Message.TextInput(listOf(FlatImeOp.ReplaceSelection("a"), FlatImeOp.SetSelection(0, 0)))
      ),
      EditorImeCommandNormalizer.normalize(
        listOf(CommitTextCommand("a", 1), SetSelectionCommand(0, 0)),
        Ime(text = "", windowStart = 0, selection = ImeRange(0, 0), composing = null),
        documentNavigation = movement,
      ),
    )
  }

  @Test
  fun `native caret crossing a multi code point grapheme preserves its target`() {
    data class Case(val text: String, val fromFlat: Int, val toUtf16: Int, val toFlat: Int)

    for ((text, from, to, expected) in
      listOf(
        Case("a👩‍💻bc", 11, 6, 14),
        Case("a👩‍💻bc", 14, 1, 11),
        Case("ae\u0301bc", 11, 3, 13),
      )) {
      val ime =
        Ime(text = text, windowStart = 10, selection = ImeRange(from, from), composing = null)
      assertEquals(
        listOf(Message.Selection(SelectionOp.SetFlat(expected, expected))),
        EditorImeCommandNormalizer.normalize(listOf(SetSelectionCommand(to, to)), ime),
      )
    }
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
      Ime(text = "텐데. ㅎㅇ", windowStart = 4559, selection = ImeRange(4565, 4565), composing = null)

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
            FlatImeOp.ReplaceSelection(""),
            FlatImeOp.ReplaceSelection("ㅎ"),
            FlatImeOp.ReplaceSelection("아"),
          )
        )
      ),
      messages,
    )
  }

  @Test
  fun `mixed batch selection offsets convert utf16 indices to code point offsets`() {
    val ime = Ime(text = "a😀b", windowStart = 10, selection = ImeRange(13, 13), composing = null)

    val messages =
      EditorImeCommandNormalizer.normalize(
        listOf(SetSelectionCommand(1, 3), CommitTextCommand("x", 1)),
        ime = ime,
      )

    assertEquals(
      listOf(
        Message.TextInput(listOf(FlatImeOp.SetSelection(11, 12), FlatImeOp.ReplaceSelection("x")))
      ),
      messages,
    )
  }

  @Test
  fun `mixed batch composing region projects window-relative offsets to absolute`() {
    val ime =
      Ime(text = "텐데. ㅎㅇ", windowStart = 4559, selection = ImeRange(4565, 4565), composing = null)

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

    assertEquals(listOf(Message.TextInput(listOf(FlatImeOp.ReplaceSelection("x")))), messages)
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
