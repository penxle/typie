package co.typie.editor.input

import androidx.compose.ui.text.input.CommitTextCommand
import androidx.compose.ui.text.input.DeleteSurroundingTextInCodePointsCommand
import androidx.compose.ui.text.input.FinishComposingTextCommand
import androidx.compose.ui.text.input.SetComposingTextCommand
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorDesktopTextEditingBatchTest {
  @Test
  fun `text editing scope calls are forwarded as edit commands in order`() {
    val batch = EditorDesktopTextEditingBatch()

    batch.deleteSurroundingTextInCodePoints(1, 2)
    batch.setComposingText("하", 3)
    batch.finishComposingText()
    batch.commitText("foo\r\nbar", 4)

    assertEquals(
      listOf(
        DeleteSurroundingTextInCodePointsCommand(1, 2),
        SetComposingTextCommand("하", 3),
        FinishComposingTextCommand(),
        CommitTextCommand("foo\r\nbar", 4),
      ),
      batch.drainCommands(),
    )
  }

  @Test
  fun `draining commands clears the batch`() {
    val batch = EditorDesktopTextEditingBatch()

    batch.commitText("하", 1)

    assertEquals(listOf(CommitTextCommand("하", 1)), batch.drainCommands())
    assertEquals(emptyList(), batch.drainCommands())
  }
}
