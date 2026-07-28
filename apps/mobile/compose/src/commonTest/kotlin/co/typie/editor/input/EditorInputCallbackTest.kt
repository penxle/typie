package co.typie.editor.input

import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SystemEvent
import kotlin.test.Test
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.test.runTest

class EditorInputCallbackTest {
  private val message = Message.System(SystemEvent.Initialize)

  @Test
  fun `input callback does not invoke the block after the editor is terminal`() = runTest {
    val editor = Editor(FakeFfiEditor(), this)
    var invoked = false
    editor.dispose()

    val result =
      editor.runInputCallback<Unit> {
        invoked = true
        error("must not run")
      }

    assertNull(result)
    assertFalse(invoked)
  }

  @Test
  fun `input callback contains a failure already owned by the editor`() = runTest {
    val failure = IllegalStateException("tick failed")
    val reported = mutableListOf<Throwable>()
    val editor =
      Editor(
        inner = FakeFfiEditor(onTick = { throw failure }),
        scope = this,
        onError = { _, error -> reported += error },
      )

    val result = editor.runInputCallback { editor.updateNow { enqueue(message) } }

    assertNull(result)
    assertTrue(editor.terminal)
    assertSame(failure, reported.single())
  }

  @Test
  fun `input callback rethrows a nonterminal programming error`() = runTest {
    val failure = IllegalStateException("programming error")
    val editor = Editor(FakeFfiEditor(), this)

    val thrown =
      assertFailsWith<IllegalStateException> { editor.runInputCallback<Unit> { throw failure } }

    assertSame(failure, thrown)
    assertFalse(editor.terminal)
  }
}
