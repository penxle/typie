package co.typie.editor.scroll

import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SystemEvent
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest

class EditorUpdateWithBringIntoViewTest {
  private val dispatcher = StandardTestDispatcher()

  @Test
  fun `update bringIntoView attaches to applied editor version`() =
    runTest(dispatcher) {
      val requests = EditorBringIntoViewRequests()
      val editor = Editor(FakeFfiEditor(), this, dispatcher)

      val update =
        assertNotNull(
          editor.updateWithBringIntoView(requests) {
            enqueue(Message.System(SystemEvent.Initialize))
            bringIntoView(EditorBringIntoViewTarget.CurrentSelectionHead)
          }
        )

      assertEquals(1L, update.revision)
      assertEquals(1L, update.snapshot.version)
      assertNull(requests.activateForVersion(version = 0L))
      assertEquals(
        request(EditorBringIntoViewTarget.CurrentSelectionHead),
        requests.activateForVersion(version = 1L),
      )
    }

  @Test
  fun `updateNow bringIntoView attaches to applied editor version`() =
    runTest(dispatcher) {
      val requests = EditorBringIntoViewRequests()
      val editor = Editor(FakeFfiEditor(), this, dispatcher)

      val update =
        assertNotNull(
          editor.updateNowWithBringIntoView(requests) {
            enqueue(Message.System(SystemEvent.Initialize))
            bringIntoView(EditorBringIntoViewTarget.CurrentSelectionHead)
          }
        )

      assertEquals(1L, update.revision)
      assertEquals(1L, update.snapshot.version)
      assertNull(requests.activateForVersion(version = 0L))
      assertEquals(
        request(EditorBringIntoViewTarget.CurrentSelectionHead),
        requests.activateForVersion(version = 1L),
      )
    }

  @Test
  fun `update bringIntoView survives caller cancellation after admission`() =
    runTest(dispatcher) {
      val requests = EditorBringIntoViewRequests()
      lateinit var caller: Job
      val fake = FakeFfiEditor(beforeEnqueueRequest = { caller.cancel() })
      val editor = Editor(fake, this, dispatcher)
      caller =
        launch(start = CoroutineStart.LAZY) {
          editor.updateWithBringIntoView(requests) {
            enqueue(Message.System(SystemEvent.Initialize))
            bringIntoView(EditorBringIntoViewTarget.CurrentSelectionHead)
          }
        }
      caller.start()
      dispatcher.scheduler.advanceUntilIdle()
      caller.join()

      assertTrue(caller.isCancelled)
      assertEquals(1L, editor.appliedState.version)
      assertEquals(
        request(EditorBringIntoViewTarget.CurrentSelectionHead),
        requests.activateForVersion(version = 1L),
      )
    }

  private fun request(
    target: EditorBringIntoViewTarget,
    behavior: EditorBringIntoViewBehavior = EditorBringIntoViewBehavior.Instant,
  ): EditorBringIntoViewRequests.Request =
    EditorBringIntoViewRequests.Request(target = target, behavior = behavior)
}
