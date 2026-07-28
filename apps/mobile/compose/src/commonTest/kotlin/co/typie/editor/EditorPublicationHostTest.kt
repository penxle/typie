package co.typie.editor

import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.unit.IntSize
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.FrameKey
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.SystemEvent
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.async
import kotlinx.coroutines.supervisorScope
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

class EditorPublicationHostTest {
  private val dispatcher = StandardTestDispatcher()
  private val message = Message.System(SystemEvent.Initialize)

  @Test
  fun publicationWaitsForEveryActiveTarget() =
    runTest(dispatcher) {
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          pageSizesProvider = {
            listOf(Size(width = 100f, height = 100f), Size(width = 100f, height = 100f))
          },
        )
      val editor = Editor(fake, this, dispatcher)
      editor.activateVisualHost(Any())
      val first = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0, wakeDelivery = {})
      val second = editor.attachSurface(1, 11L, 100.0, 100.0, 1.0, wakeDelivery = {})
      advanceUntilIdle()
      val publishedFirstBitmap = editor.deliverFrame(first, editorRevision = 0L, frameKey = 1L)
      editor.deliverFrame(second, editorRevision = 0L, frameKey = 2L)
      advanceUntilIdle()

      val receipt =
        async(start = CoroutineStart.UNDISPATCHED) { editor.update { enqueue(message) } }
      runCurrent()
      val update = requireNotNull(receipt.await())
      val publication = async(start = CoroutineStart.UNDISPATCHED) { update.awaitPublished() }
      assertEquals(listOf(1L, 1L), fake.renderCalls.takeLast(2).map { it.requestedRevision.value })

      val pendingFirstBitmap = editor.deliverFrame(first, editorRevision = 1L, frameKey = 3L)
      runCurrent()
      assertFalse(publication.isCompleted)
      val retained = editor.retainedFrames(page = 0)
      assertEquals(2, retained.size)
      assertSame(publishedFirstBitmap, retained[0])
      assertSame(pendingFirstBitmap, retained[1])

      editor.deliverFrame(second, editorRevision = 1L, frameKey = 4L)
      advanceUntilIdle()

      assertIs<Published>(publication.await())
      val publishedPages =
        listOfNotNull(
          editor.publishedFrameAt(page = 0, revision = 1L)?.let { 0 },
          editor.publishedFrameAt(page = 1, revision = 1L)?.let { 1 },
        )
      assertEquals(setOf(0, 1), publishedPages.toSet())
      assertEquals(1L, editor.publishedRevision)
      val retainedAfterPublication = editor.retainedFrames(page = 0)
      assertEquals(1, retainedAfterPublication.size)
      assertSame(pendingFirstBitmap, retainedAfterPublication.single())
    }

  @Test
  fun deliveredExactFrameKeyIsReusedForANewerRequiredRevision() =
    runTest(dispatcher) {
      val reusedFrameKey = FrameKey(7L)
      val fake =
        FakeFfiEditor(
            onTick = { listOf(EditorEvent.RenderInvalidated) },
            pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) },
          )
          .apply { renderFrameProvider = { _, _ -> reusedFrameKey } }
      val editor = Editor(fake, this, dispatcher)
      editor.activateVisualHost(Any())
      var wakeCount = 0
      val session = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0) { wakeCount += 1 }
      advanceUntilIdle()
      editor.deliverFrame(session, editorRevision = 0L, frameKey = reusedFrameKey.value)
      advanceUntilIdle()

      val update = requireNotNull(editor.update { enqueue(message) })
      val publication = async(start = CoroutineStart.UNDISPATCHED) { update.awaitPublished() }
      advanceUntilIdle()

      assertTrue(publication.isCompleted)
      assertIs<Published>(publication.await())
      assertEquals(1, wakeCount)
      assertEquals(reusedFrameKey, editor.publishedFrameAt(page = 0, revision = 1L)?.frameKey)
      assertEquals(1L, editor.publishedFrameAt(page = 0, revision = 1L)?.editorRevision)
      assertEquals(1L, editor.publishedRevision)
    }

  @Test
  fun newerRevisionsDoNotReplaceActualInFlightOperation() =
    runTest(dispatcher) {
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) },
        )
      val editor = Editor(fake, this, dispatcher)
      editor.activateVisualHost(Any())
      var wakeCount = 0
      val session = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0) { wakeCount += 1 }
      advanceUntilIdle()

      assertEquals(listOf(0L), fake.renderCalls.map { it.requestedRevision.value })
      assertEquals(1, wakeCount)

      requireNotNull(editor.updateNow { enqueue(message) })
      requireNotNull(editor.updateNow { enqueue(message) })
      advanceUntilIdle()

      assertEquals(
        listOf(0L),
        fake.renderCalls.map { it.requestedRevision.value },
        "new requirements must coalesce behind the one actual in-flight operation",
      )

      editor.deliverFrame(session, editorRevision = 0L, frameKey = 1L)
      advanceUntilIdle()

      assertEquals(listOf(0L, 2L), fake.renderCalls.map { it.requestedRevision.value })
      assertEquals(2, wakeCount)

      editor.deliverFrame(session, editorRevision = 2L, frameKey = 2L)
      advanceUntilIdle()

      assertEquals(2L, editor.publishedRevision)
    }

  @Test
  fun supersededNullRenderKeepsEarlierWaitersUntilNewerRevisionPublishes() =
    runTest(dispatcher) {
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) },
        )
      val editor = Editor(fake, this, dispatcher)
      editor.activateVisualHost(Any())
      val session = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0, wakeDelivery = {})
      advanceUntilIdle()
      editor.deliverFrame(session, editorRevision = 0L, frameKey = 1L)
      advanceUntilIdle()

      fake.renderFrameProvider = { _, revision -> if (revision.value == 1L) null else FrameKey(2L) }
      val earlierUpdate = requireNotNull(editor.updateNow { enqueue(message) })
      val publication =
        async(start = CoroutineStart.UNDISPATCHED) { earlierUpdate.awaitPublished() }
      val newerUpdate = requireNotNull(editor.updateNow { enqueue(message) })
      advanceUntilIdle()

      assertEquals(listOf(0L, 1L, 2L), fake.renderCalls.map { it.requestedRevision.value })
      assertFalse(publication.isCompleted)

      editor.deliverFrame(session, editorRevision = newerUpdate.revision, frameKey = 2L)
      advanceUntilIdle()

      assertEquals(Published(newerUpdate.revision), publication.await())
    }

  @Test
  fun publishedBundleKeepsSnapshotAndFrameProofTogether() =
    runTest(dispatcher) {
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) },
        )
      val editor = Editor(fake, this, dispatcher)
      editor.activateVisualHost(Any())
      val session = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0, wakeDelivery = {})
      advanceUntilIdle()
      val firstBitmap = editor.deliverFrame(session, editorRevision = 0L, frameKey = 1L)
      advanceUntilIdle()
      val first = requireNotNull(editor.publishedBundle)
      assertEquals(0L, first.snapshot.version)
      assertEquals(FrameKey(1L), first.frames[0]?.proof?.frameKey)
      assertSame(firstBitmap, first.frames[0]?.bitmap)

      requireNotNull(editor.update { enqueue(message) })
      advanceUntilIdle()
      val secondBitmap = editor.deliverFrame(session, editorRevision = 1L, frameKey = 2L)
      advanceUntilIdle()

      val second = requireNotNull(editor.publishedBundle)
      assertEquals(1L, second.snapshot.version)
      assertEquals(FrameKey(2L), second.frames[0]?.proof?.frameKey)
      assertEquals(FrameKey(1L), first.frames[0]?.proof?.frameKey)
      assertSame(secondBitmap, second.frames[0]?.bitmap)
      assertSame(firstBitmap, first.frames[0]?.bitmap)
    }

  @Test
  fun targetReplacementsCoalesceBehindActualInFlightOperation() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      editor.activateVisualHost(Any())
      var wakeCount = 0
      val session = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0) { wakeCount += 1 }
      advanceUntilIdle()

      session.requestResize(SurfaceConfiguration(200.0, 100.0, 1.0))
      session.requestResize(SurfaceConfiguration(300.0, 100.0, 1.0))
      advanceUntilIdle()
      assertEquals(
        listOf(0L),
        fake.renderCalls.map { it.requestedRevision.value },
        "target replacement must wait for the one actual in-flight operation",
      )
      assertEquals(1, wakeCount)

      editor.deliverFrame(session, editorRevision = 0L, frameKey = 1L)
      advanceUntilIdle()
      assertNull(editor.publishedFrameAt(page = 0, revision = 0L))
      assertEquals(listOf(0L, 0L), fake.renderCalls.map { it.requestedRevision.value })
      assertEquals(2, wakeCount)

      editor.deliverFrame(session, editorRevision = 0L, frameKey = 2L)
      advanceUntilIdle()
      assertEquals(FrameKey(2L), editor.publishedFrameAt(page = 0, revision = 0L)?.frameKey)
    }

  @Test
  fun sameRevisionReplacementMustPublishItsCurrentSurfaceBeforeAwaitCompletes() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor(pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) })
      val editor = Editor(fake, this, dispatcher)
      editor.activateVisualHost(Any())
      val session = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0, wakeDelivery = {})
      advanceUntilIdle()
      editor.deliverFrame(session, editorRevision = 0L, frameKey = 1L)
      advanceUntilIdle()

      val update = requireNotNull(editor.update { enqueue(message) })
      assertIs<Published>(update.awaitPublished())

      session.requestResize(SurfaceConfiguration(100.0, 100.0, 2.0))
      advanceUntilIdle()
      val publication = async(start = CoroutineStart.UNDISPATCHED) { update.awaitPublished() }

      assertFalse(publication.isCompleted)

      editor.deliverFrame(session, editorRevision = update.revision, frameKey = 2L)
      advanceUntilIdle()

      assertIs<Published>(publication.await())
      assertEquals(
        FrameKey(2L),
        editor.publishedFrameAt(page = 0, revision = update.revision)?.frameKey,
      )
    }

  @Test
  fun pendingWaitCompletesWhenTheLastReplacementTargetDetaches() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor(pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) })
      val editor = Editor(fake, this, dispatcher)
      editor.activateVisualHost(Any())
      val session = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0, wakeDelivery = {})
      advanceUntilIdle()
      editor.deliverFrame(session, editorRevision = 0L, frameKey = 1L)
      advanceUntilIdle()

      val update = requireNotNull(editor.update { enqueue(message) })
      assertEquals(Published(update.revision), update.awaitPublished())

      session.requestResize(SurfaceConfiguration(width = 100.0, height = 100.0, scaleFactor = 2.0))
      advanceUntilIdle()
      val publication = async(start = CoroutineStart.UNDISPATCHED) { update.awaitPublished() }
      assertFalse(publication.isCompleted)

      session.detach()
      advanceUntilIdle()

      try {
        assertTrue(publication.isCompleted)
        assertEquals(Published(update.revision), publication.await())
      } finally {
        publication.cancel()
        advanceUntilIdle()
      }
    }

  @Test
  fun detachingTheLastTargetRetainsPublishedBundleUntilReplacementProofArrives() =
    runTest(dispatcher) {
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) },
        )
      val editor = Editor(fake, this, dispatcher)
      editor.activateVisualHost(Any())
      val session = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0, wakeDelivery = {})
      advanceUntilIdle()
      editor.deliverFrame(session, editorRevision = 0L, frameKey = 1L)
      advanceUntilIdle()
      val previous = requireNotNull(editor.publishedBundle)
      val update = requireNotNull(editor.update { enqueue(message) })
      val publication = async(start = CoroutineStart.UNDISPATCHED) { update.awaitPublished() }
      advanceUntilIdle()

      session.detach()
      advanceUntilIdle()

      assertSame(previous, editor.publishedBundle)
      assertEquals(0L, editor.publishedRevision)
      assertEquals(FrameKey(1L), editor.publishedFrame(0)?.frameKey)
      assertFalse(publication.isCompleted)

      var replacementFrameKey: FrameKey? = null
      val replacement =
        editor.attachSurface(0, 11L, 100.0, 100.0, 1.0) { frameKey ->
          replacementFrameKey = frameKey
        }
      advanceUntilIdle()

      assertSame(previous, editor.publishedBundle)
      assertFalse(publication.isCompleted)

      val replacementBitmap =
        editor.deliverFrame(
          replacement,
          editorRevision = update.revision,
          frameKey = requireNotNull(replacementFrameKey).value,
        )
      advanceUntilIdle()

      assertEquals(Published(update.revision), publication.await())
      val published = requireNotNull(editor.publishedBundle)
      assertEquals(update.revision, published.snapshot.version)
      assertSame(replacementBitmap, published.frames.getValue(0).bitmap)
    }

  @Test
  fun pageShrinkPreparationCompletesLateWaitAfterReplacementDetaches() =
    runTest(dispatcher) {
      var pageSizes = listOf(Size(width = 100f, height = 100f), Size(width = 100f, height = 100f))
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          pageSizesProvider = { pageSizes },
        )
      val editor = Editor(fake, this, dispatcher)
      editor.activateVisualHost(Any())
      val stale = editor.attachSurface(1, 10L, 100.0, 100.0, 1.0, wakeDelivery = {})
      advanceUntilIdle()
      editor.deliverFrame(stale, editorRevision = 0L, frameKey = 1L)
      advanceUntilIdle()
      assertEquals(0L, editor.publishedRevision)

      pageSizes = listOf(Size(width = 200f, height = 300f))
      val update = requireNotNull(editor.update { enqueue(message) })
      advanceUntilIdle()

      assertEquals(update.revision, editor.appliedRevision)
      assertEquals(0L, editor.publishedRevision)
      assertEquals(0, editor.preparingPage)

      var replacementFrameKey: FrameKey? = null
      val replacement =
        editor.attachSurface(0, 11L, 100.0, 100.0, 1.0) { frameKey ->
          replacementFrameKey = frameKey
        }
      advanceUntilIdle()
      assertEquals(FakeFfiEditor.SurfaceAttachCall(0, 200.0, 300.0, 1.0), fake.attachCalls.last())
      assertEquals(0, editor.preparingPage)
      editor.deliverFrame(
        replacement,
        editorRevision = update.revision,
        frameKey = requireNotNull(replacementFrameKey).value,
      )
      advanceUntilIdle()

      assertEquals(update.revision, editor.publishedRevision)
      assertNull(editor.preparingPage)

      replacement.detach()
      advanceUntilIdle()
      val latePublication = async(start = CoroutineStart.UNDISPATCHED) { update.awaitPublished() }
      try {
        advanceUntilIdle()
        assertTrue(latePublication.isCompleted)
        assertEquals(Published(update.revision), latePublication.await())
      } finally {
        latePublication.cancel()
        advanceUntilIdle()
      }
    }

  @Test
  fun terminalFailureCancelsPreparationWithoutDroppingThePublishedBundle() =
    runTest(dispatcher) {
      var pageSizes = listOf(Size(width = 100f, height = 100f), Size(width = 100f, height = 100f))
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          pageSizesProvider = { pageSizes },
        )
      val editor = Editor(fake, this, dispatcher)
      editor.activateVisualHost(Any())
      val stale = editor.attachSurface(1, 10L, 100.0, 100.0, 1.0, wakeDelivery = {})
      advanceUntilIdle()
      editor.deliverFrame(stale, editorRevision = 0L, frameKey = 1L)
      advanceUntilIdle()
      val publishedBeforeFailure = requireNotNull(editor.publishedBundle)

      pageSizes = listOf(Size(width = 200f, height = 300f))
      requireNotNull(editor.update { enqueue(message) })
      advanceUntilIdle()
      assertEquals(0, editor.preparingPage)

      editor.fail(IllegalStateException("test failure"))
      advanceUntilIdle()

      assertNull(editor.preparingPage)
      assertSame(publishedBeforeFailure, editor.publishedBundle)
    }

  @Test
  fun stalePublishedSizeCallbackDoesNotReplaceLatestAppliedTarget() =
    runTest(dispatcher) {
      var pageHeight = 100f
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          pageSizesProvider = { listOf(Size(width = 100f, height = pageHeight)) },
        )
      val editor = Editor(fake, this, dispatcher)
      editor.activateVisualHost(Any())
      val session = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0, wakeDelivery = {})
      advanceUntilIdle()
      editor.deliverFrame(session, editorRevision = 0L, frameKey = 1L)
      advanceUntilIdle()

      pageHeight = 200f
      requireNotNull(editor.updateNow { enqueue(message) })
      advanceUntilIdle()
      fake.resizeCalls.clear()

      // A Compose effect from the previously published geometry can run after the
      // Host has already reconciled the target from the latest applied snapshot.
      session.requestResize(SurfaceConfiguration(100.0, 100.0, 1.0))
      advanceUntilIdle()

      assertTrue(fake.resizeCalls.isEmpty())

      session.requestResize(SurfaceConfiguration(100.0, 100.0, 2.0))
      advanceUntilIdle()

      assertEquals(listOf(FakeFfiEditor.SurfaceResizeCall(0, 100.0, 200.0, 2.0)), fake.resizeCalls)
    }

  @Test
  fun stalePublishedSizeDoesNotConfigureANewSurfaceTarget() =
    runTest(dispatcher) {
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          pageSizesProvider = { listOf(Size(width = 100f, height = 200f)) },
        )
      val editor = Editor(fake, this, dispatcher)
      requireNotNull(editor.updateNow { enqueue(message) })
      editor.activateVisualHost(Any())

      val session = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0, wakeDelivery = {})
      advanceUntilIdle()
      fake.resizeCalls.clear()

      session.requestResize(SurfaceConfiguration(100.0, 100.0, 1.0))
      advanceUntilIdle()

      assertTrue(fake.resizeCalls.isEmpty())
    }

  @Test
  fun nullPresentationWaitsForExplicitReplacementBeforeRetry() =
    runTest(dispatcher) {
      var available = false
      val fake =
        FakeFfiEditor(pageSizesProvider = { listOf(Size(100f, 100f)) }).apply {
          renderFrameProvider = { _, _ -> if (available) FrameKey(7L) else null }
        }
      val editor = Editor(fake, this, dispatcher)
      editor.activateVisualHost(Any())
      var wakeCount = 0
      val session = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0) { wakeCount += 1 }
      advanceUntilIdle()
      assertEquals(1, fake.renderCount)
      assertEquals(0, wakeCount)
      assertNull(editor.publishedFrameAt(page = 0, revision = 0L))

      available = true
      session.requestResize(SurfaceConfiguration(200.0, 100.0, 1.0))
      advanceUntilIdle()
      assertEquals(2, fake.renderCount)
      assertEquals(1, wakeCount)

      editor.deliverFrame(session, editorRevision = 0L, frameKey = 7L)
      advanceUntilIdle()
      assertEquals(FrameKey(7L), editor.publishedFrameAt(page = 0, revision = 0L)?.frameKey)
      assertTrue(editor.publishedFrameAt(page = 0, revision = 0L) != null)
    }

  @Test
  fun newerAppliedRevisionRetriesANullPresentation() =
    runTest(dispatcher) {
      var renderAttempt = 0
      val fake =
        FakeFfiEditor(pageSizesProvider = { listOf(Size(100f, 100f)) }).apply {
          renderFrameProvider = { _, _ ->
            renderAttempt += 1
            if (renderAttempt == 1) null else FrameKey(8L)
          }
        }
      val editor = Editor(fake, this, dispatcher)
      editor.activateVisualHost(Any())
      var wakeCount = 0
      val session = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0) { wakeCount += 1 }
      advanceUntilIdle()
      assertEquals(1, fake.renderCount)
      assertEquals(0, wakeCount)

      requireNotNull(editor.update { enqueue(message) })
      advanceUntilIdle()

      assertEquals(listOf(0L, 1L), fake.renderCalls.map { it.requestedRevision.value })
      assertEquals(1, wakeCount)
      editor.deliverFrame(session, editorRevision = 1L, frameKey = 8L)
      advanceUntilIdle()
      assertEquals(FrameKey(8L), editor.publishedFrameAt(page = 0, revision = 1L)?.frameKey)
      assertEquals(1L, editor.publishedRevision)
    }

  @Test
  fun mismatchedDeliveredFrameDoesNotLeaveThePageInFlight() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor(pageSizesProvider = { listOf(Size(100f, 100f)) })
      val editor = Editor(fake, this, dispatcher)
      editor.activateVisualHost(Any())
      val prepared = mutableListOf<FrameKey>()
      val session =
        editor.attachSurface(0, 10L, 100.0, 100.0, 1.0) { frameKey -> prepared += frameKey }
      advanceUntilIdle()

      editor.deliverFrame(session, editorRevision = 0L, frameKey = 999L)
      advanceUntilIdle()

      assertEquals(listOf(0L, 0L), fake.renderCalls.map { it.requestedRevision.value })
      assertEquals(listOf(FrameKey(1L), FrameKey(2L)), prepared)
    }

  @Test
  fun unavailablePlatformCopyRejectsItsRevisionAndRetriesANewerRevision() =
    runTest(dispatcher) {
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) },
        )
      val editor = Editor(fake, this, dispatcher)
      editor.activateVisualHost(Any())
      var pendingFrameKey: FrameKey? = null
      val session =
        editor.attachSurface(0, 10L, 100.0, 100.0, 1.0) { frameKey -> pendingFrameKey = frameKey }
      advanceUntilIdle()
      editor.deliverFrame(session, editorRevision = 0L, frameKey = 1L)
      advanceUntilIdle()

      supervisorScope {
        val failedUpdate = requireNotNull(editor.update { enqueue(message) })
        val failedPublication =
          async(start = CoroutineStart.UNDISPATCHED) { failedUpdate.awaitPublished() }
        advanceUntilIdle()

        editor.surfaceUnavailable(session, requireNotNull(pendingFrameKey))
        advanceUntilIdle()

        assertTrue(failedPublication.isCompleted)
        assertIs<EditorSurfaceUnavailableException>(
          runCatching { failedPublication.await() }.exceptionOrNull()
        )
        assertEquals(listOf(0L, 1L), fake.renderCalls.map { it.requestedRevision.value })
        assertEquals(0L, editor.publishedRevision)
        assertEquals(FrameKey(1L), editor.publishedFrame(0)?.frameKey)
      }

      val recoveredUpdate = requireNotNull(editor.update { enqueue(message) })
      advanceUntilIdle()
      assertEquals(listOf(0L, 1L, 2L), fake.renderCalls.map { it.requestedRevision.value })

      editor.deliverFrame(session, editorRevision = 2L, frameKey = 3L)
      advanceUntilIdle()

      assertIs<Published>(recoveredUpdate.awaitPublished())
      assertEquals(2L, editor.publishedRevision)
      assertEquals(FrameKey(3L), editor.publishedFrame(0)?.frameKey)
    }

  @Test
  fun lateAwaitReturnsAnAlreadyPublishedRevisionAfterANewerRenderFails() =
    runTest(dispatcher) {
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) },
        )
      val editor = Editor(fake, this, dispatcher)
      editor.activateVisualHost(Any())
      var pendingFrameKey: FrameKey? = null
      val session =
        editor.attachSurface(0, 10L, 100.0, 100.0, 1.0) { frameKey -> pendingFrameKey = frameKey }
      advanceUntilIdle()
      editor.deliverFrame(session, editorRevision = 0L, frameKey = 1L)
      advanceUntilIdle()

      val publishedUpdate = requireNotNull(editor.update { enqueue(message) })
      advanceUntilIdle()
      editor.deliverFrame(session, editorRevision = 1L, frameKey = 2L)
      advanceUntilIdle()

      requireNotNull(editor.update { enqueue(message) })
      advanceUntilIdle()
      editor.surfaceUnavailable(session, requireNotNull(pendingFrameKey))
      advanceUntilIdle()

      assertEquals(Published(1L), publishedUpdate.awaitPublished())
    }

  @Test
  fun unavailableTargetFailsCurrentWaitersAndAReplacementCanRecover() =
    runTest(dispatcher) {
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) },
        )
      val editor = Editor(fake, this, dispatcher)
      editor.activateVisualHost(Any())
      var pendingFrameKey: FrameKey? = null
      val failedSession = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0) { pendingFrameKey = it }
      advanceUntilIdle()
      editor.deliverFrame(failedSession, editorRevision = 0L, frameKey = 1L)
      advanceUntilIdle()

      supervisorScope {
        val update = requireNotNull(editor.update { enqueue(message) })
        val publication = async(start = CoroutineStart.UNDISPATCHED) { update.awaitPublished() }
        advanceUntilIdle()

        editor.surfaceTargetUnavailable(failedSession, requireNotNull(pendingFrameKey))
        advanceUntilIdle()

        assertIs<EditorSurfaceUnavailableException>(
          runCatching { publication.await() }.exceptionOrNull()
        )
        assertEquals(0L, editor.publishedRevision)
        assertEquals(FrameKey(1L), editor.publishedFrame(0)?.frameKey)

        editor.deliverFrame(failedSession, editorRevision = 1L, frameKey = 2L)
        advanceUntilIdle()
        assertEquals(0L, editor.publishedRevision)
      }

      val replacement = editor.attachSurface(0, 11L, 100.0, 100.0, 1.0, wakeDelivery = {})
      advanceUntilIdle()
      editor.deliverFrame(replacement, editorRevision = 1L, frameKey = 3L)
      advanceUntilIdle()

      assertEquals(1L, editor.publishedRevision)
      assertEquals(FrameKey(3L), editor.publishedFrame(0)?.frameKey)
    }

  @Test
  fun unexpectedCurrentSurfaceDeliveryFailureIsTerminalWithoutReplacingPublishedFrame() =
    runTest(dispatcher) {
      val failure = IllegalStateException("platform copy failed")
      val reported = mutableListOf<Throwable>()
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) },
        )
      val editor = Editor(fake, this, dispatcher, onError = { _, error -> reported += error })
      editor.activateVisualHost(Any())
      val session = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0, wakeDelivery = {})
      advanceUntilIdle()
      editor.deliverFrame(session, editorRevision = 0L, frameKey = 1L)
      advanceUntilIdle()
      val published = requireNotNull(editor.publishedBundle)

      supervisorScope {
        val update = requireNotNull(editor.update { enqueue(message) })
        val publication = async(start = CoroutineStart.UNDISPATCHED) { update.awaitPublished() }
        advanceUntilIdle()

        editor.surfaceDeliveryFailed(page = 0, session = session, error = failure)
        advanceUntilIdle()

        assertTrue(editor.terminal)
        assertEquals(listOf<Throwable>(failure), reported)
        assertSame(published, editor.publishedBundle)
        val publicationFailure =
          assertIs<IllegalStateException>(runCatching { publication.await() }.exceptionOrNull())
        assertEquals(failure.message, publicationFailure.message)

        editor.surfaceDeliveryFailed(
          page = 0,
          session = session,
          error = IllegalStateException("late failure"),
        )
        advanceUntilIdle()
        assertEquals(listOf<Throwable>(failure), reported)
      }
    }

  @Test
  fun staleSurfaceDeliveryFailureDoesNotFailTheEditor() =
    runTest(dispatcher) {
      val reported = mutableListOf<Throwable>()
      val fake = FakeFfiEditor(pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) })
      val editor = Editor(fake, this, dispatcher, onError = { _, error -> reported += error })
      fake.applySnapshot(editor)
      val host = Any()
      editor.activateVisualHost(host)
      val stale = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0, wakeDelivery = {})
      advanceUntilIdle()
      editor.attachSurface(0, 11L, 100.0, 100.0, 1.0, wakeDelivery = {})
      advanceUntilIdle()

      editor.surfaceDeliveryFailed(
        page = 0,
        session = stale,
        error = IllegalStateException("stale platform copy failed"),
      )
      advanceUntilIdle()

      assertFalse(editor.terminal)
      assertTrue(reported.isEmpty())

      editor.deactivateVisualHost(host)
      editor.surfaceDeliveryFailed(
        page = 0,
        session = null,
        error = IllegalStateException("detached platform allocation failed"),
      )
      advanceUntilIdle()

      assertFalse(editor.terminal)
      assertTrue(reported.isEmpty())
    }

  @Test
  fun lateSurfaceCleanupFailureIsNonTerminalAfterTheSurfaceIsRemoved() =
    runTest(dispatcher) {
      val reported = mutableListOf<Throwable>()
      val fake = FakeFfiEditor(pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) })
      val editor = Editor(fake, this, dispatcher, onError = { _, error -> reported += error })
      editor.activateVisualHost(Any())
      val session = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0, wakeDelivery = {})
      advanceUntilIdle()

      session.detach { error("late buffer release failed") }
      advanceUntilIdle()

      assertFalse(editor.terminal)
      assertTrue(reported.isEmpty())
    }

  @Test
  fun lateSurfaceCleanupFailureIsNonTerminalAfterAReplacementIsAttached() =
    runTest(dispatcher) {
      val reported = mutableListOf<Throwable>()
      val fake = FakeFfiEditor(pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) })
      val editor = Editor(fake, this, dispatcher, onError = { _, error -> reported += error })
      editor.activateVisualHost(Any())
      val stale = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0, wakeDelivery = {})
      advanceUntilIdle()

      stale.detach { error("late stale buffer release failed") }
      editor.attachSurface(0, 11L, 100.0, 100.0, 1.0, wakeDelivery = {})
      advanceUntilIdle()

      assertFalse(editor.terminal)
      assertTrue(reported.isEmpty())
    }
}

private fun Editor.publishedFrameAt(page: Int, revision: Long): FrameProof? =
  publishedBundle?.takeIf { it.snapshot.version == revision }?.frames?.get(page)?.proof

private fun Editor.publishedFrame(page: Int): FrameProof? =
  publishedBundle?.frames?.get(page)?.proof

private fun Editor.deliverFrame(
  session: SurfaceSessionHandle,
  editorRevision: Long,
  frameKey: Long,
): ImageBitmap {
  val bitmap = ImageBitmap(width = 100, height = 100)
  deliverFrame(
    session = session,
    bitmap = bitmap,
    pixelSize = IntSize(width = 100, height = 100),
    editorRevision = editorRevision,
    frameKey = frameKey,
  )
  return bitmap
}
