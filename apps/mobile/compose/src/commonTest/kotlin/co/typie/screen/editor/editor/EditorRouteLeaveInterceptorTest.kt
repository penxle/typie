package co.typie.screen.editor.editor

import co.typie.editor.DocumentEditingStop
import co.typie.editor.EditingCheckpointResult
import co.typie.navigation.RouteRemovalDecision
import co.typie.navigation.RouteRemovalPreparation
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.cancelAndJoin
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
        beginStop = { throw failure },
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
        beginStop = {
          object : DocumentEditingStop {
            override suspend fun awaitCheckpoint(): EditingCheckpointResult = awaitCancellation()

            override suspend fun retryCheckpoint(): EditingCheckpointResult = awaitCancellation()

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

  @Test
  fun routeAcquiresItsStopBeforeSuspendingReload() = runTest {
    var routeOwnsStop = false
    val interceptor =
      EditorRouteLeaveInterceptor(
        finalizeInput = {},
        restoreInput = {},
        beginStop = {
          routeOwnsStop = true
          object : DocumentEditingStop {
            override suspend fun awaitCheckpoint() = EditingCheckpointResult.Protected

            override suspend fun retryCheckpoint() = EditingCheckpointResult.Protected

            override fun cancel() {
              routeOwnsStop = false
            }
          }
        },
        onPreparationStarted = { assertTrue(routeOwnsStop) },
        resolveDecision = { RouteRemovalDecision.CancelRemoval },
      )

    assertEquals(RouteRemovalPreparation.Ready, interceptor.prepare())
    assertTrue(routeOwnsStop)
  }

  @Test
  fun rollbackRestartsPendingReloadBeforeReleasingRouteStop() = runTest {
    var routeOwnsStop = false
    var reloadOwnsStop = false
    var restored = 0
    val interceptor =
      EditorRouteLeaveInterceptor(
        finalizeInput = {},
        restoreInput = { restored += 1 },
        beginStop = {
          routeOwnsStop = true
          object : DocumentEditingStop {
            override suspend fun awaitCheckpoint() =
              EditingCheckpointResult.ProtectionFailed(IllegalStateException("unprotected"))

            override suspend fun retryCheckpoint() = awaitCheckpoint()

            override fun cancel() {
              assertTrue(reloadOwnsStop)
              routeOwnsStop = false
            }
          }
        },
        resumeReloadBeforeRollback = {
          assertTrue(routeOwnsStop)
          reloadOwnsStop = true
          true
        },
        resolveDecision = { RouteRemovalDecision.CancelRemoval },
      )
    assertEquals(RouteRemovalPreparation.NeedsDecision, interceptor.prepare())

    interceptor.rollback()

    assertFalse(routeOwnsStop)
    assertTrue(reloadOwnsStop)
    assertEquals(0, restored)
  }

  @Test
  fun rollbackRestoresInputOnceWhenThereIsNoPendingReload() = runTest {
    var restored = 0
    val interceptor =
      interceptor(
        awaitResult = {
          EditingCheckpointResult.ProtectionFailed(IllegalStateException("unprotected"))
        },
        restoreInput = { restored += 1 },
        resumeReloadBeforeRollback = { false },
      )
    assertEquals(RouteRemovalPreparation.NeedsDecision, interceptor.prepare())

    interceptor.rollback()
    interceptor.rollback()

    assertEquals(1, restored)
  }

  @Test
  fun failedReloadResumeCleansUpRouteOwnershipBeforePropagating() = runTest {
    val checkpoint = CompletableDeferred<EditingCheckpointResult>()
    val failure = IllegalStateException("reload launch failed")
    var cancelled = false
    var hidden = false
    var restored = false
    val interceptor =
      interceptor(
        awaitResult = { checkpoint.await() },
        onCancel = {
          cancelled = true
          checkpoint.complete(EditingCheckpointResult.StopCancelled)
        },
        restoreInput = { restored = true },
        delayedFeedbackMillis = 10,
        checkpointWatchdogMillis = 30,
        showDelayedFeedback = {},
        hideDelayedFeedback = { hidden = true },
        resumeReloadBeforeRollback = { throw failure },
      )
    val preparation = async { interceptor.prepare(onDelayed = {}) }
    advanceTimeBy(10)
    runCurrent()

    assertEquals(failure, assertFailsWith<IllegalStateException> { interceptor.rollback() })

    assertTrue(cancelled)
    assertTrue(hidden)
    assertTrue(restored)
    assertEquals(RouteRemovalPreparation.NeedsDecision, preparation.await())
  }

  @Test
  fun cancelledReloadLaunchReturnsFalseAndRestoresInput() = runTest {
    val reloadAcquired = CompletableDeferred<Boolean>()
    var restored = 0
    val interceptor =
      interceptor(
        awaitResult = {
          EditingCheckpointResult.ProtectionFailed(IllegalStateException("unprotected"))
        },
        restoreInput = { restored += 1 },
        resumeReloadBeforeRollback = { reloadAcquired.await() },
      )
    assertEquals(RouteRemovalPreparation.NeedsDecision, interceptor.prepare())
    val rollback = async { interceptor.rollback() }
    runCurrent()
    assertFalse(rollback.isCompleted)

    reloadAcquired.complete(false)
    rollback.await()

    assertEquals(1, restored)
  }

  @Test
  fun cancelledPreparationRestartsPendingReloadBeforeReleasingRouteStop() = runTest {
    var routeOwnsStop = false
    var reloadOwnsStop = false
    var restored = 0
    val interceptor =
      EditorRouteLeaveInterceptor(
        finalizeInput = {},
        restoreInput = { restored += 1 },
        beginStop = {
          routeOwnsStop = true
          object : DocumentEditingStop {
            override suspend fun awaitCheckpoint(): EditingCheckpointResult = awaitCancellation()

            override suspend fun retryCheckpoint(): EditingCheckpointResult = awaitCancellation()

            override fun cancel() {
              assertTrue(reloadOwnsStop)
              routeOwnsStop = false
            }
          }
        },
        onPreparationStarted = {},
        resumeReloadBeforeRollback = {
          assertTrue(routeOwnsStop)
          reloadOwnsStop = true
          true
        },
        resolveDecision = { RouteRemovalDecision.CancelRemoval },
      )
    val preparation = async(start = CoroutineStart.UNDISPATCHED) { interceptor.prepare() }

    preparation.cancelAndJoin()

    assertFalse(routeOwnsStop)
    assertTrue(reloadOwnsStop)
    assertEquals(0, restored)
  }

  @Test
  fun cancellationDuringReloadHandoffFinishesHandoffBeforeReleasingRouteStop() = runTest {
    val handoffCanFinish = CompletableDeferred<Unit>()
    val ownershipEvents = mutableListOf<String>()
    val interceptor =
      EditorRouteLeaveInterceptor(
        finalizeInput = {},
        restoreInput = { ownershipEvents += "input restored" },
        beginStop = {
          object : DocumentEditingStop {
            override suspend fun awaitCheckpoint(): EditingCheckpointResult = awaitCancellation()

            override suspend fun retryCheckpoint(): EditingCheckpointResult = awaitCancellation()

            override fun cancel() {
              ownershipEvents += "route released"
            }
          }
        },
        onPreparationStarted = { handoffCanFinish.await() },
        resumeReloadBeforeRollback = {
          ownershipEvents += "reload resumed"
          true
        },
        resolveDecision = { RouteRemovalDecision.CancelRemoval },
      )
    val preparation = async(start = CoroutineStart.UNDISPATCHED) { interceptor.prepare() }

    preparation.cancel()
    handoffCanFinish.complete(Unit)
    preparation.join()

    assertEquals(listOf("reload resumed", "route released"), ownershipEvents)
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
  resumeReloadBeforeRollback: suspend () -> Boolean = { false },
): EditorRouteLeaveInterceptor =
  EditorRouteLeaveInterceptor(
    finalizeInput = {},
    restoreInput = restoreInput,
    beginStop = {
      object : DocumentEditingStop {
        override suspend fun awaitCheckpoint(): EditingCheckpointResult = awaitResult()

        override suspend fun retryCheckpoint(): EditingCheckpointResult = awaitResult()

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
    resumeReloadBeforeRollback = resumeReloadBeforeRollback,
  )
