package co.typie.editor.scroll

import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SystemEvent
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorAwaitWithBringIntoViewTest {
  private val dispatcher = StandardTestDispatcher()

  @Test
  fun `bringIntoView attaches to committed editor version`() =
    runTest(dispatcher) {
      val requests = EditorBringIntoViewRequests()
      val editor = Editor(FakeFfiEditor(), this, dispatcher)

      editor.awaitWithBringIntoView(requests) {
        enqueue(Message.System(SystemEvent.Initialize))
        beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentCursorLine) }
      }

      assertNull(requests.activateForVersion(version = 0L))
      assertEquals(
        request(EditorBringIntoViewTarget.CurrentCursorLine),
        requests.activateForVersion(version = 1L),
      )
    }

  @Test
  fun `sync bringIntoView attaches to committed editor version`() =
    runTest(dispatcher) {
      val requests = EditorBringIntoViewRequests()
      val editor = Editor(FakeFfiEditor(), this, dispatcher)

      editor.syncWithBringIntoView(requests) {
        enqueue(Message.System(SystemEvent.Initialize))
        beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentCursorLine) }
      }

      assertNull(requests.activateForVersion(version = 0L))
      assertEquals(
        request(EditorBringIntoViewTarget.CurrentCursorLine),
        requests.activateForVersion(version = 1L),
      )
    }

  private fun request(
    target: EditorBringIntoViewTarget,
    behavior: EditorBringIntoViewBehavior = EditorBringIntoViewBehavior.Instant,
  ): EditorBringIntoViewRequests.Request =
    EditorBringIntoViewRequests.Request(target = target, behavior = behavior)
}
