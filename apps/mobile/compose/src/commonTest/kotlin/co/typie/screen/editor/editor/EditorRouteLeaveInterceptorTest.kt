package co.typie.screen.editor.editor

import co.typie.editor.DocumentEditingClose
import co.typie.editor.LocalDurabilityResult
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
        awaitResult = { LocalDurabilityResult.EditFailed(IllegalStateException("edit")) },
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
  fun localDurabilityWaitIsBounded() = runTest {
    val interceptor =
      interceptor(awaitResult = { awaitCancellation() }, localDurabilityWatchdogMillis = 10)

    assertEquals(RouteRemovalPreparation.NeedsDecision, interceptor.prepare())
    assertEquals(10L, currentTime)
  }

  @Test
  fun defaultLocalDurabilityWatchdogExpiresAtThreeSeconds() = runTest {
    val interceptor =
      EditorRouteLeaveInterceptor(
        finalizeInput = {},
        restoreInput = {},
        beginClose = {
          object : DocumentEditingClose {
            override suspend fun awaitLocalDurability(): LocalDurabilityResult = awaitCancellation()

            override suspend fun retryLocalDurability(): LocalDurabilityResult =
              error("A timeout must not start a retry")

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
    val durability = CompletableDeferred<LocalDurabilityResult>()
    var delayed = 0
    var shown = 0
    var hidden = 0
    val interceptor =
      interceptor(
        awaitResult = { durability.await() },
        delayedFeedbackMillis = 10,
        localDurabilityWatchdogMillis = 30,
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

    durability.complete(LocalDurabilityResult.Captured)
    assertEquals(RouteRemovalPreparation.Ready, preparation.await())
    assertEquals(1, hidden)
  }

  @Test
  fun captureRetryContinuesAcrossSoftThresholdWithoutRestarting() = runTest {
    val failure = LocalDurabilityResult.CaptureFailed(IllegalStateException("still unavailable"))
    val retryCompletion = CompletableDeferred<LocalDurabilityResult>()
    var retries = 0
    val interceptor =
      interceptor(
        awaitResult = { failure },
        retryResult = {
          retries++
          retryCompletion.await()
        },
        delayedFeedbackMillis = 10,
        localDurabilityWatchdogMillis = 30,
      )

    val preparation = async { interceptor.prepare(onDelayed = {}) }
    runCurrent()
    assertEquals(1, retries)

    advanceTimeBy(10)
    runCurrent()
    assertEquals(1, retries)

    retryCompletion.complete(failure)
    assertEquals(RouteRemovalPreparation.NeedsDecision, preparation.await())
    assertEquals(1, retries)
  }

  @Test
  fun directPreparationDoesNotShowRouteRemovalFeedback() = runTest {
    var shown = 0
    val interceptor =
      interceptor(
        awaitResult = { awaitCancellation() },
        delayedFeedbackMillis = 10,
        localDurabilityWatchdogMillis = 20,
        showDelayedFeedback = { shown++ },
      )

    assertEquals(RouteRemovalPreparation.NeedsDecision, interceptor.prepare())
    assertEquals(20L, currentTime)
    assertEquals(0, shown)
  }

  @Test
  fun rollbackDismissesVisibleDelayedFeedbackOnce() = runTest {
    val durability = CompletableDeferred<LocalDurabilityResult>()
    var shown = 0
    var hidden = 0
    val interceptor =
      interceptor(
        awaitResult = { durability.await() },
        onCancel = { durability.complete(LocalDurabilityResult.SessionStopped) },
        delayedFeedbackMillis = 10,
        localDurabilityWatchdogMillis = 30,
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
  fun retriesCaptureFailureOnceBeforeAllowingRemoval() = runTest {
    var retries = 0
    val interceptor =
      interceptor(
        awaitResult = { LocalDurabilityResult.CaptureFailed(IllegalStateException("disk")) },
        retryResult = {
          retries++
          LocalDurabilityResult.Captured
        },
      )

    assertEquals(RouteRemovalPreparation.Ready, interceptor.prepare())
    assertEquals(1, retries)
  }

  @Test
  fun twoCaptureFailuresNeedDecision() = runTest {
    val interceptor =
      interceptor(
        awaitResult = { LocalDurabilityResult.CaptureFailed(IllegalStateException("unprotected")) },
        retryResult = { LocalDurabilityResult.CaptureFailed(IllegalStateException("unprotected")) },
      )

    assertEquals(RouteRemovalPreparation.NeedsDecision, interceptor.prepare())
  }
}

private fun interceptor(
  awaitResult: suspend () -> LocalDurabilityResult = { LocalDurabilityResult.Captured },
  retryResult: suspend () -> LocalDurabilityResult = { LocalDurabilityResult.Captured },
  onCancel: () -> Unit = {},
  restoreInput: () -> Unit = {},
  delayedFeedbackMillis: Long = 350,
  localDurabilityWatchdogMillis: Long = 3_000,
  showDelayedFeedback: () -> Unit = {},
  hideDelayedFeedback: () -> Unit = {},
): EditorRouteLeaveInterceptor =
  EditorRouteLeaveInterceptor(
    finalizeInput = {},
    restoreInput = restoreInput,
    beginClose = {
      object : DocumentEditingClose {
        override suspend fun awaitLocalDurability(): LocalDurabilityResult = awaitResult()

        override suspend fun retryLocalDurability(): LocalDurabilityResult = retryResult()

        override fun cancel() {
          onCancel()
        }
      }
    },
    resolveDecision = { RouteRemovalDecision.CancelRemoval },
    delayedFeedbackMillis = delayedFeedbackMillis,
    localDurabilityWatchdogMillis = localDurabilityWatchdogMillis,
    showDelayedFeedback = showDelayedFeedback,
    hideDelayedFeedback = hideDelayedFeedback,
  )
