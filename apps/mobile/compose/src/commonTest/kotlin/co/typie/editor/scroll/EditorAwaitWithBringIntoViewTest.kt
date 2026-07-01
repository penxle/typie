package co.typie.editor.scroll

import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SystemEvent
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
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
        beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentSelectionHead) }
      }

      assertNull(requests.activateForVersion(version = 0L))
      assertEquals(
        request(EditorBringIntoViewTarget.CurrentSelectionHead),
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
        beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentSelectionHead) }
      }

      assertNull(requests.activateForVersion(version = 0L))
      assertEquals(
        request(EditorBringIntoViewTarget.CurrentSelectionHead),
        requests.activateForVersion(version = 1L),
      )
    }

  @Test
  fun `await bringIntoView request survives cancel before commit`() =
    runTest(dispatcher) {
      val requests = EditorBringIntoViewRequests()
      val editor =
        Editor(FakeFfiEditor(onTick = { listOf(EditorEvent.RenderInvalidated) }), this, dispatcher)
      editor.attachSurface(page = 0, handle = 0L, width = 0.0, height = 0.0, scaleFactor = 1.0)

      val job =
        launch(dispatcher) {
          editor.awaitWithBringIntoView(requests) {
            enqueue(Message.System(SystemEvent.Initialize))
            beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentSelectionHead) }
          }
        }
      dispatcher.scheduler.advanceUntilIdle()

      assertNull(requests.activateForVersion(version = 1L))
      requests.cancel()

      editor.onPageSettled(page = 0, version = 1L)
      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(
        request(EditorBringIntoViewTarget.CurrentSelectionHead),
        requests.activateForVersion(version = 1L),
      )
      job.join()
    }

  private fun request(
    target: EditorBringIntoViewTarget,
    behavior: EditorBringIntoViewBehavior = EditorBringIntoViewBehavior.Instant,
  ): EditorBringIntoViewRequests.Request =
    EditorBringIntoViewRequests.Request(target = target, behavior = behavior)
}
