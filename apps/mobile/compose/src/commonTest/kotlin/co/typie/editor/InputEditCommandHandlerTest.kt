package co.typie.editor

import androidx.compose.ui.text.input.CommitTextCommand
import co.typie.editor.ffi.CompositionOp
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Message
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class InputEditCommandHandlerTest {
  private val dispatcher = StandardTestDispatcher()

  @Test
  fun `IME edit commands attach bringIntoView to committed editor version`() =
    runTest(dispatcher) {
      val requests = EditorBringIntoViewRequests()
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      InputEditCommandHandler.handle(
        editor = editor,
        bringIntoViewRequests = requests,
        commands = listOf(CommitTextCommand("a", 1)),
      )

      val expectedMessages: List<Message> =
        listOf(Message.Composition(CompositionOp.Flat(listOf(FlatImeOp.ReplaceSelection("a")))))
      assertEquals(expectedMessages, fake.enqueued)
      assertNull(requests.activateForVersion(version = 0L))
      assertEquals(
        EditorBringIntoViewTarget.CurrentCursorLine,
        requests.activateForVersion(version = 1L),
      )
    }
}
