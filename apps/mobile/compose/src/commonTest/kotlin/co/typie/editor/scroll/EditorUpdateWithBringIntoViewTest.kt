package co.typie.editor.scroll

import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.unit.IntSize
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.SurfaceSessionHandle
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.FrameKey
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.ffi.ViewOp
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest

class EditorUpdateWithBringIntoViewTest {
  private val dispatcher = StandardTestDispatcher()

  @Test
  fun `tracked item reveal expands folds and binds semantic target to the same version`() =
    runTest(dispatcher) {
      val requests = EditorBringIntoViewRequests()
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      val update = assertNotNull(editor.revealTrackedItem(requests, id = "match-1"))

      assertEquals(
        listOf<Message>(Message.View(ViewOp.ExpandFoldsForTrackedRange(id = "match-1"))),
        fake.enqueued,
      )
      assertEquals(1L, update.revision)
      val request = assertNotNull(requests.activateForVersion(version = update.revision))
      assertEquals(EditorBringIntoViewTarget.TrackedItem("match-1"), request.target)
      assertEquals(EditorBringIntoViewPolicy.Reveal, request.policy)
      assertEquals(EditorBringIntoViewBehavior.Smooth, request.behavior)
    }

  @Test
  fun `update bringIntoView attaches to applied editor version`() =
    runTest(dispatcher) {
      val requests = EditorBringIntoViewRequests()
      val editor = Editor(FakeFfiEditor(), this, dispatcher)

      val updateDeferred = async {
        editor.updateWithBringIntoView(requests) {
          enqueue(Message.System(SystemEvent.Initialize))
          bringIntoView(
            EditorBringIntoViewTarget.CurrentSelectionHead,
            policy = EditorBringIntoViewPolicy.CursorGuard,
          )
        }
      }
      testScheduler.runCurrent()
      val request = assertNotNull(requests.activateForVersion(version = 1L))
      assertEquals(EditorBringIntoViewPolicy.CursorGuard, request.policy)
      assertTrue(requests.markPresented(version = 1L, request = request))
      val update = assertNotNull(updateDeferred.await())

      assertEquals(1L, update.revision)
      assertEquals(1L, update.snapshot.version)
      assertNull(requests.activateForVersion(version = 0L))
      assertNull(requests.activateForVersion(version = 1L))
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
            bringIntoView(
              EditorBringIntoViewTarget.CurrentSelectionHead,
              policy = EditorBringIntoViewPolicy.Typewriter,
              behavior = EditorBringIntoViewBehavior.Smooth,
            )
          }
        )

      assertEquals(1L, update.revision)
      assertEquals(1L, update.snapshot.version)
      assertNull(requests.activateForVersion(version = 0L))
      val request = assertNotNull(requests.activateForVersion(version = 1L))
      assertTrue(request.target == EditorBringIntoViewTarget.CurrentSelectionHead)
      assertEquals(EditorBringIntoViewPolicy.Typewriter, request.policy)
      assertEquals(EditorBringIntoViewBehavior.Smooth, request.behavior)
    }

  @Test
  fun `update bringIntoView discards an unadmitted reveal when admission fails`() =
    runTest(dispatcher) {
      var presentationRequests = 0
      val requests = EditorBringIntoViewRequests { presentationRequests += 1 }
      val editor = Editor(FakeFfiEditor(), this, dispatcher)

      assertFailsWith<kotlinx.coroutines.CancellationException> {
        editor.updateWithBringIntoView(
          bringIntoViewRequests = requests,
          admit = { throw kotlinx.coroutines.CancellationException("not admitted") },
        ) {
          enqueue(Message.System(SystemEvent.Initialize))
          bringIntoView(
            EditorBringIntoViewTarget.CurrentSelectionHead,
            policy = EditorBringIntoViewPolicy.CursorGuard,
          )
        }
      }

      assertEquals(2, presentationRequests)
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
            bringIntoView(
              EditorBringIntoViewTarget.CurrentSelectionHead,
              policy = EditorBringIntoViewPolicy.CursorGuard,
            )
          }
        }
      caller.start()
      dispatcher.scheduler.advanceUntilIdle()
      caller.join()

      assertTrue(caller.isCancelled)
      assertEquals(1L, editor.appliedState.version)
      val request = assertNotNull(requests.activateForVersion(version = 1L))
      assertSame(EditorBringIntoViewTarget.CurrentSelectionHead, request.target)
    }

  @Test
  fun `required surface failure releases ordered bringIntoView updates`() =
    runTest(dispatcher) {
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) },
        )
      val editor = Editor(fake, this, dispatcher)
      val requests = EditorBringIntoViewRequests()
      activateVisualHost(editor, requests)
      var preparedFrameKey: FrameKey? = null
      val session =
        editor.attachSurface(0, 10L, 100.0, 100.0, 1.0) { frameKey -> preparedFrameKey = frameKey }
      editor.requestSurfacePages(setOf(0))
      advanceUntilIdle()
      editor.deliverPreparedFrame(
        session,
        revision = 0L,
        frameKey = requireNotNull(preparedFrameKey),
      )
      advanceUntilIdle()

      val first = async {
        editor.updateWithBringIntoView(requests) {
          enqueue(Message.System(SystemEvent.Initialize))
          bringIntoView(
            EditorBringIntoViewTarget.CurrentSelectionHead,
            policy = EditorBringIntoViewPolicy.CursorGuard,
          )
        }
      }
      testScheduler.runCurrent()
      assertFalse(first.isCompleted)

      editor.surfaceUnavailable(session, requireNotNull(preparedFrameKey))
      advanceUntilIdle()

      assertTrue(first.isCompleted, "surface failure must not leave the ordered consumer suspended")
      assertEquals(1L, assertNotNull(first.await()).revision)

      val second = async {
        editor.updateWithBringIntoView(requests) {
          enqueue(Message.System(SystemEvent.Initialize))
          bringIntoView(
            EditorBringIntoViewTarget.CurrentSelectionHead,
            policy = EditorBringIntoViewPolicy.CursorGuard,
          )
        }
      }
      testScheduler.runCurrent()
      val nextRequest = assertNotNull(requests.activateForVersion(version = 2L))
      assertTrue(requests.markPresented(version = 2L, request = nextRequest))
      assertEquals(2L, assertNotNull(second.await()).revision)
    }

  @Test
  fun `obsolete platform copy failure preserves a semantic reveal for the newer version`() =
    runTest(dispatcher) {
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) },
        )
      val editor = Editor(fake, this, dispatcher)
      val requests = EditorBringIntoViewRequests()
      activateVisualHost(editor, requests)
      var preparedFrameKey: FrameKey? = null
      val session =
        editor.attachSurface(0, 10L, 100.0, 100.0, 1.0) { frameKey -> preparedFrameKey = frameKey }
      editor.requestSurfacePages(setOf(0))
      advanceUntilIdle()
      editor.deliverPreparedFrame(
        session,
        revision = 0L,
        frameKey = requireNotNull(preparedFrameKey),
      )
      advanceUntilIdle()

      val first = async {
        editor.updateWithBringIntoView(requests) {
          enqueue(Message.System(SystemEvent.Initialize))
          bringIntoView(
            EditorBringIntoViewTarget.CurrentSelectionHead,
            policy = EditorBringIntoViewPolicy.CursorGuard,
          )
        }
      }
      testScheduler.runCurrent()
      val request = assertNotNull(requests.activateForVersion(version = 1L))
      val obsoleteFrameKey = requireNotNull(preparedFrameKey)

      val newer = assertNotNull(editor.update { enqueue(Message.System(SystemEvent.Initialize)) })
      advanceUntilIdle()
      assertEquals(2L, newer.revision)

      editor.surfaceUnavailable(session, obsoleteFrameKey)
      advanceUntilIdle()

      assertFalse(
        first.isCompleted,
        "an obsolete copy failure must not terminate a semantic reveal",
      )
      assertSame(request, requests.activateForVersion(version = 2L))
      val currentFrameKey = requireNotNull(preparedFrameKey)
      assertTrue(currentFrameKey != obsoleteFrameKey)

      editor.deliverPreparedFrame(session, revision = 2L, frameKey = currentFrameKey)
      advanceUntilIdle()
      assertTrue(requests.markPresented(version = 2L, request = request))
      assertEquals(1L, assertNotNull(first.await()).revision)
    }
}

private fun activateVisualHost(editor: Editor, requests: EditorBringIntoViewRequests) {
  editor.activateVisualHost(Any(), requests::discardFailedForVersion)
}

private fun Editor.deliverPreparedFrame(
  session: SurfaceSessionHandle,
  revision: Long,
  frameKey: FrameKey,
) {
  deliverFrame(
    session = session,
    bitmap = ImageBitmap(width = 100, height = 100),
    pixelSize = IntSize(width = 100, height = 100),
    editorRevision = revision,
    frameKey = frameKey.value,
  )
}
