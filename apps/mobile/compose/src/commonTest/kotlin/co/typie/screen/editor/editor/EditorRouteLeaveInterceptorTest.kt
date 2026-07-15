package co.typie.screen.editor.editor

import co.typie.editor.DocumentEditingClose
import co.typie.editor.EditingCheckpointResult
import co.typie.navigation.RouteRemovalDecision
import co.typie.navigation.RouteRemovalPreparation
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.currentTime
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorRouteLeaveInterceptorTest {
  @Test
  fun editFailureNeedsDecisionAndRollbackRestoresInput() = runTest {
    var cancelled = 0
    var restored = 0
    val interceptor =
      interceptor(
        awaitResult = { EditingCheckpointResult.EditFailed(IllegalStateException("edit")) },
        onCancel = { cancelled++ },
        restoreInput = { restored++ },
      )

    assertEquals(RouteRemovalPreparation.NeedsDecision, interceptor.prepare())
    assertEquals(RouteRemovalDecision.CancelRemoval, interceptor.resolveDecision())
    interceptor.rollback()
    assertEquals(1, cancelled)
    assertEquals(1, restored)
  }

  @Test
  fun closeStartFailureRestoresInput() = runTest {
    var restored = 0
    val failure = IllegalStateException("stale session")
    val interceptor =
      EditorRouteLeaveInterceptor(
        finalizeInput = {},
        restoreInput = { restored++ },
        beginClose = { throw failure },
        resolveDecision = { RouteRemovalDecision.CancelRemoval },
      )

    assertEquals(failure, assertFailsWith<IllegalStateException> { interceptor.prepare() })
    assertEquals(1, restored)
  }

  @Test
  fun checkpointWaitIsBounded() = runTest {
    val interceptor =
      interceptor(awaitResult = { awaitCancellation() }, checkpointWatchdogMillis = 10)

    assertEquals(RouteRemovalPreparation.NeedsDecision, interceptor.prepare())
    assertEquals(10L, currentTime)
  }

  @Test
  fun defaultCheckpointWatchdogExpiresAtThreeSeconds() = runTest {
    val interceptor =
      EditorRouteLeaveInterceptor(
        finalizeInput = {},
        restoreInput = {},
        beginClose = {
          object : DocumentEditingClose {
            override suspend fun awaitCheckpoint(): EditingCheckpointResult = awaitCancellation()

            override fun cancel() = Unit
          }
        },
        resolveDecision = { RouteRemovalDecision.CancelRemoval },
      )

    assertEquals(RouteRemovalPreparation.NeedsDecision, interceptor.prepare())
    assertEquals(3_000L, currentTime)
  }

  @Test
  fun delayedFeedbackStartsAtSoftThresholdAndSuccessStillAllowsRemoval() = runTest {
    val checkpoint = CompletableDeferred<EditingCheckpointResult>()
    var delayed = 0
    var shown = 0
    var hidden = 0
    val interceptor =
      interceptor(
        awaitResult = { checkpoint.await() },
        delayedFeedbackMillis = 10,
        checkpointWatchdogMillis = 30,
        showDelayedFeedback = { shown++ },
        hideDelayedFeedback = { hidden++ },
      )

    val preparation = async { interceptor.prepare(onDelayed = { delayed++ }) }
    runCurrent()
    advanceTimeBy(9)
    runCurrent()
    assertEquals(0, delayed)
    assertEquals(0, shown)
    assertFalse(preparation.isCompleted)

    advanceTimeBy(1)
    runCurrent()
    assertEquals(1, delayed)
    assertEquals(1, shown)
    assertFalse(preparation.isCompleted)

    checkpoint.complete(EditingCheckpointResult.Protected)
    assertEquals(RouteRemovalPreparation.Ready, preparation.await())
    assertEquals(1, hidden)
  }

  @Test
  fun checkpointWaitContinuesAcrossSoftThresholdWithoutRestarting() = runTest {
    val checkpoint = CompletableDeferred<EditingCheckpointResult>()
    var waits = 0
    val interceptor =
      interceptor(
        awaitResult = {
          waits++
          checkpoint.await()
        },
        delayedFeedbackMillis = 10,
        checkpointWatchdogMillis = 30,
      )

    val preparation = async { interceptor.prepare(onDelayed = {}) }
    advanceTimeBy(10)
    runCurrent()

    assertEquals(1, waits)
    checkpoint.complete(EditingCheckpointResult.Protected)
    assertEquals(RouteRemovalPreparation.Ready, preparation.await())
  }

  @Test
  fun directPreparationDoesNotShowRouteRemovalFeedback() = runTest {
    var shown = 0
    val interceptor =
      interceptor(
        awaitResult = { awaitCancellation() },
        delayedFeedbackMillis = 10,
        checkpointWatchdogMillis = 20,
        showDelayedFeedback = { shown++ },
      )

    assertEquals(RouteRemovalPreparation.NeedsDecision, interceptor.prepare())
    assertEquals(20L, currentTime)
    assertEquals(0, shown)
  }

  @Test
  fun rollbackDismissesVisibleDelayedFeedbackOnce() = runTest {
    val checkpoint = CompletableDeferred<EditingCheckpointResult>()
    var shown = 0
    var hidden = 0
    val interceptor =
      interceptor(
        awaitResult = { checkpoint.await() },
        onCancel = { checkpoint.complete(EditingCheckpointResult.SessionStopped) },
        delayedFeedbackMillis = 10,
        checkpointWatchdogMillis = 30,
        showDelayedFeedback = { shown++ },
        hideDelayedFeedback = { hidden++ },
      )

    val preparation = async { interceptor.prepare(onDelayed = {}) }
    advanceTimeBy(10)
    runCurrent()
    assertEquals(1, shown)

    interceptor.rollback()
    assertEquals(RouteRemovalPreparation.NeedsDecision, preparation.await())
    assertEquals(1, hidden)
  }

  @Test
  fun protectionFailureNeedsDecision() = runTest {
    val interceptor =
      interceptor(
        awaitResult = {
          EditingCheckpointResult.ProtectionFailed(IllegalStateException("unprotected"))
        }
      )

    assertEquals(RouteRemovalPreparation.NeedsDecision, interceptor.prepare())
  }
}

private fun interceptor(
  awaitResult: suspend () -> EditingCheckpointResult = { EditingCheckpointResult.Protected },
  onCancel: () -> Unit = {},
  restoreInput: () -> Unit = {},
  delayedFeedbackMillis: Long = 350,
  checkpointWatchdogMillis: Long = 3_000,
  showDelayedFeedback: () -> Unit = {},
  hideDelayedFeedback: () -> Unit = {},
): EditorRouteLeaveInterceptor =
  EditorRouteLeaveInterceptor(
    finalizeInput = {},
    restoreInput = restoreInput,
    beginClose = {
      object : DocumentEditingClose {
        override suspend fun awaitCheckpoint(): EditingCheckpointResult = awaitResult()

        override fun cancel() {
          onCancel()
        }
      }
    },
    resolveDecision = { RouteRemovalDecision.CancelRemoval },
    delayedFeedbackMillis = delayedFeedbackMillis,
    checkpointWatchdogMillis = checkpointWatchdogMillis,
    showDelayedFeedback = showDelayedFeedback,
    hideDelayedFeedback = hideDelayedFeedback,
  )
