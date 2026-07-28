package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.runtime.BroadcastFrameClock
import co.typie.screen.editor.editor.toolbar.EditorToolbarSessionState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineExceptionHandler
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.async
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class AttachmentOperationTest {
  @Test
  fun imageAndFilePendingRemainUntilLocalCommitCompletes() = runTest {
    for (kind in listOf(AttachmentKind.Image, AttachmentKind.File)) {
      var pending = true
      var cached = false
      val allowCommit = CompletableDeferred<Unit>()

      val operation = async {
        completeAttachmentOperation(
          persist = { "$kind-asset" },
          cache = { cached = true },
          commit = { allowCommit.await() },
          clearPending = { pending = false },
        )
      }
      runCurrent()

      assertTrue(cached, "$kind metadata should be cached before commit")
      assertTrue(pending, "$kind pending should remain during commit")
      assertFalse(operation.isCompleted)

      allowCommit.complete(Unit)
      assertEquals("$kind-asset", operation.await())
      assertFalse(pending, "$kind pending should clear after commit")
    }
  }

  @Test
  fun imageAndFileCommitFailureDoesNotReportSuccessAndClearsPending() = runTest {
    for (kind in listOf(AttachmentKind.Image, AttachmentKind.File)) {
      var pending = true
      val commitFailure = IllegalStateException("$kind commit failed")

      val failure =
        assertFailsWith<AttachmentException> {
          completeAttachmentOperation(
            persist = { "$kind-asset" },
            cache = {},
            commit = { throw commitFailure },
            clearPending = { pending = false },
          )
        }

      assertEquals(AttachmentFailureStage.CommitDocument, failure.stage)
      assertSame(commitFailure, failure.cause)
      assertFalse(pending)
    }
  }

  @Test
  fun persistenceFailureIsClassifiedAndClearsPending() = runTest {
    var pending = true
    val persistFailure = IllegalStateException("persist failed")

    val failure =
      assertFailsWith<AttachmentException> {
        completeAttachmentOperation<String>(
          persist = { throw persistFailure },
          cache = {},
          commit = {},
          clearPending = { pending = false },
        )
      }

    assertEquals(AttachmentFailureStage.PersistAsset, failure.stage)
    assertSame(persistFailure, failure.cause)
    assertFalse(pending)
  }

  @Test
  fun cancellationPassesThroughWithoutBecomingAttachmentFailure() = runTest {
    var pending = true
    val cancellation = CancellationException("cancelled")

    val thrown =
      assertFailsWith<CancellationException> {
        completeAttachmentOperation<String>(
          persist = { throw cancellation },
          cache = {},
          commit = {},
          clearPending = { pending = false },
        )
      }

    assertSame(cancellation, thrown)
    assertTrue(pending)
  }

  @Test
  fun pickerLaunchWaitsForBlockedInputFrame() = runTest {
    val frameClock = BroadcastFrameClock()
    val sessionState = EditorToolbarSessionState()
    var launched = false

    launchAttachmentPicker(
      scope = CoroutineScope(coroutineContext + frameClock),
      sessionState = sessionState,
      requestIsCurrent = { true },
      clearRequest = {},
      launchPicker = { launched = true },
      onFailure = {},
    )

    assertTrue(sessionState.pickerInputActive)
    runCurrent()
    assertFalse(launched)

    frameClock.sendFrame(0L)
    runCurrent()

    assertTrue(launched)
  }

  @Test
  fun pickerLaunchCancellationDoesNotReportFailure() = runTest {
    val frameClock = BroadcastFrameClock()
    val pickerJob = SupervisorJob()
    var failureCount = 0

    launchAttachmentPicker(
      scope = CoroutineScope(coroutineContext + pickerJob + frameClock),
      sessionState = EditorToolbarSessionState(),
      requestIsCurrent = { true },
      clearRequest = {},
      launchPicker = {},
      onFailure = { failureCount += 1 },
    )
    runCurrent()

    pickerJob.cancel(CancellationException("picker owner disposed"))
    runCurrent()

    assertEquals(0, failureCount)
  }

  @Test
  fun pickerLaunchFailureReleasesInputAndRequest() = runTest {
    val frameClock = BroadcastFrameClock()
    val pickerJob = SupervisorJob()
    val uncaught = mutableListOf<Throwable>()
    val exceptionHandler = CoroutineExceptionHandler { _, error -> uncaught += error }
    val sessionState = EditorToolbarSessionState()
    var requestCurrent = true
    var failureCount = 0

    launchAttachmentPicker(
      scope = CoroutineScope(coroutineContext + pickerJob + frameClock + exceptionHandler),
      sessionState = sessionState,
      requestIsCurrent = { requestCurrent },
      clearRequest = { requestCurrent = false },
      launchPicker = { throw IllegalArgumentException("picker launch failed") },
      onFailure = { failureCount += 1 },
    )
    runCurrent()

    frameClock.sendFrame(0L)
    runCurrent()

    assertFalse(sessionState.pickerInputActive)
    assertFalse(requestCurrent)
    assertEquals(1, failureCount)
    assertTrue(uncaught.isEmpty())
    pickerJob.cancel()
  }
}
