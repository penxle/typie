package co.typie.editor.input

import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Key
import co.typie.editor.ffi.KeyEvent
import co.typie.editor.ffi.Message
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorDesktopTextEditingBatchTest {
  @Test
  fun `delete and compose are batched as one flat ime message`() {
    val batch = EditorDesktopTextEditingBatch()

    batch.deleteSurroundingTextInCodePoints(1, 0)
    batch.setComposingText("하", 1)

    assertEquals(
      listOf(Message.TextInput(listOf(FlatImeOp.DeleteSurrounding(1, 0), FlatImeOp.Compose("하")))),
      batch.drainMessages(),
    )
  }

  @Test
  fun `newline commit flushes pending flat ops before enter`() {
    val batch = EditorDesktopTextEditingBatch()

    batch.setComposingText("ㅎ", 1)
    batch.commitText("\n", 1)

    assertEquals(
      listOf(Message.TextInput(listOf(FlatImeOp.Compose("ㅎ"))), Message.Key(KeyEvent(Key.Enter))),
      batch.drainMessages(),
    )
  }

  @Test
  fun `text commit uses composition replacement and commits composition`() {
    val batch = EditorDesktopTextEditingBatch()

    batch.commitText("하", 1)

    assertEquals(
      listOf(Message.TextInput(listOf(FlatImeOp.Compose("하"), FlatImeOp.CommitAsIs))),
      batch.drainMessages(),
    )
  }

  @Test
  fun `finish composing text clears composition without active preedit`() {
    val batch = EditorDesktopTextEditingBatch()

    batch.finishComposingText()

    assertEquals(
      listOf(Message.TextInput(listOf(FlatImeOp.ClearComposition))),
      batch.drainMessages(),
    )
  }

  @Test
  fun `finish composing text commits initial active preedit as-is`() {
    val batch = EditorDesktopTextEditingBatch(initialHasActiveComposition = true)

    batch.finishComposingText()

    assertEquals(listOf(Message.TextInput(listOf(FlatImeOp.CommitAsIs))), batch.drainMessages())
  }

  @Test
  fun `finish composing text commits preedit started in same batch`() {
    val batch = EditorDesktopTextEditingBatch()

    batch.setComposingText("ㅎ", 1)
    batch.finishComposingText()

    assertEquals(
      listOf(Message.TextInput(listOf(FlatImeOp.Compose("ㅎ"), FlatImeOp.CommitAsIs))),
      batch.drainMessages(),
    )
  }
}
