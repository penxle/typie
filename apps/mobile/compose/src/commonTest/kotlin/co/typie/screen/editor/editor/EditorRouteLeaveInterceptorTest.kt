package co.typie.screen.editor.editor

import androidx.compose.runtime.State
import co.typie.editor.DocumentEditingStop
import co.typie.editor.DocumentSaveState
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
  fun recoveryKeepsTheDialogOpenUntilItsCompletionAction() = runTest {
    val recovered = CompletableDeferred<EditingCheckpointResult>()
    val choice = CompletableDeferred<RouteRemovalDecision>()
    lateinit var protection: State<DocumentSaveState>
    val interceptor =
      interceptor(
        awaitResult = {
          EditingCheckpointResult.ProtectionFailed(IllegalStateException("storage"))
        },
        awaitProtection = { _, _ -> recovered.await() },
        resolveDecision = {
          protection = it
          choice.await()
        },
      )
    assertEquals(RouteRemovalPreparation.NeedsDecision, interceptor.prepare())
    val decision = async { interceptor.resolveDecision() }
    runCurrent()
    assertEquals(DocumentSaveState.Pending, protection.value)

    recovered.complete(EditingCheckpointResult.Protected)
    runCurrent()
    assertEquals(DocumentSaveState.Protected, protection.value)
    assertFalse(decision.isCompleted)

    choice.complete(RouteRemovalDecision.ProceedWithRemoval)
    assertEquals(RouteRemovalDecision.ProceedWithRemoval, decision.await())
  }

  @Test
  fun completedDialogCanStillCancelRemoval() = runTest {
    val recovered = CompletableDeferred<EditingCheckpointResult>()
    val choice = CompletableDeferred<RouteRemovalDecision>()
    lateinit var protection: State<DocumentSaveState>
    val interceptor =
      interceptor(
        awaitResult = {
          EditingCheckpointResult.ProtectionFailed(IllegalStateException("storage"))
        },
        awaitProtection = { _, _ -> recovered.await() },
        resolveDecision = {
          protection = it
          choice.await()
        },
      )
    assertEquals(RouteRemovalPreparation.NeedsDecision, interceptor.prepare())
    val decision = async { interceptor.resolveDecision() }
    runCurrent()
    recovered.complete(EditingCheckpointResult.Protected)
    runCurrent()
    assertEquals(DocumentSaveState.Protected, protection.value)
    choice.complete(RouteRemovalDecision.CancelRemoval)
    assertEquals(RouteRemovalDecision.CancelRemoval, decision.await())
    interceptor.rollback()
  }

  @Test
  fun cancellingDecisionStopsRecoveryAndDoesNotReplayRemoval() = runTest {
    val choice = CompletableDeferred<RouteRemovalDecision>()
    val recovered = CompletableDeferred<EditingCheckpointResult>()
    var watching = false
    val interceptor =
      interceptor(
        awaitResult = {
          EditingCheckpointResult.ProtectionFailed(IllegalStateException("storage"))
        },
        awaitProtection = { _, _ ->
          watching = true
          try {
            recovered.await()
          } finally {
            watching = false
          }
        },
        resolveDecision = { choice.await() },
      )
    assertEquals(RouteRemovalPreparation.NeedsDecision, interceptor.prepare())
    val decision = async { interceptor.resolveDecision() }
    runCurrent()
    assertTrue(watching)
    choice.complete(RouteRemovalDecision.CancelRemoval)
    assertEquals(RouteRemovalDecision.CancelRemoval, decision.await())
    interceptor.rollback()
    recovered.complete(EditingCheckpointResult.Protected)
    runCurrent()
    assertFalse(watching)
    assertEquals(RouteRemovalDecision.CancelRemoval, decision.await())
  }

  @Test
  fun recoveredBodyCannotBypassFailedPendingSubPaneSave() = runTest {
    val choice = CompletableDeferred<RouteRemovalDecision>()
    lateinit var protection: State<DocumentSaveState>
    val interceptor =
      interceptor(
        awaitResult = {
          EditingCheckpointResult.ProtectionFailed(IllegalStateException("storage"))
        },
        awaitProtection = { _, _ -> EditingCheckpointResult.Protected },
        savePendingChanges = { false },
        resolveDecision = {
          protection = it
          choice.await()
        },
      )
    assertEquals(RouteRemovalPreparation.NeedsDecision, interceptor.prepare())
    val decision = async { interceptor.resolveDecision() }
    runCurrent()
    assertEquals(DocumentSaveState.Failed, protection.value)
    assertFalse(decision.isCompleted)
    choice.complete(RouteRemovalDecision.CancelRemoval)
    assertEquals(RouteRemovalDecision.CancelRemoval, decision.await())
    interceptor.rollback()
  }

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
  fun pendingSubPaneChangesMustBeSavedBeforeTheEditorRouteCanLeave() = runTest {
    var saveAttempts = 0
    val interceptor =
      interceptor(
        awaitResult = { EditingCheckpointResult.Protected },
        savePendingChanges = {
          saveAttempts += 1
          false
        },
      )

    assertEquals(RouteRemovalPreparation.NeedsDecision, interceptor.prepare())
    assertEquals(1, saveAttempts)
  }

  @Test
  fun rollbackClearsPresentationPriorityAndRestoresInputOnce() = runTest {
    var ownsStop = false
    var preparing = false
    var restored = 0
    val interceptor =
      EditorRouteLeaveInterceptor(
        finalizeInput = {},
        restoreInput = {
          assertFalse(preparing)
          assertFalse(ownsStop)
          restored += 1
        },
        beginStop = {
          ownsStop = true
          object : DocumentEditingStop {
            override suspend fun awaitCheckpoint() = EditingCheckpointResult.Protected

            override suspend fun retryCheckpoint() = awaitCheckpoint()

            override fun cancel() {
              ownsStop = false
            }
          }
        },
        onPreparationChanged = {
          if (it) assertTrue(ownsStop)
          preparing = it
        },
        resolveDecision = { RouteRemovalDecision.CancelRemoval },
      )
    assertEquals(RouteRemovalPreparation.Ready, interceptor.prepare())
    assertTrue(preparing)

    interceptor.rollback()
    interceptor.rollback()

    assertFalse(preparing)
    assertEquals(1, restored)
  }

  @Test
  fun presentationCleanupFailureStillReleasesStopAndRestoresInput() = runTest {
    val failure = IllegalStateException("presentation cleanup failed")
    var cancelled = false
    var restored = false
    val interceptor =
      interceptor(
        onCancel = { cancelled = true },
        restoreInput = { restored = true },
        onPreparationChanged = { if (!it) throw failure },
      )
    interceptor.prepare()

    assertEquals(failure, assertFailsWith<IllegalStateException> { interceptor.rollback() })

    assertTrue(cancelled)
    assertTrue(restored)
  }

  @Test
  fun cancelledPreparationClearsPresentationPriorityAndReleasesStop() = runTest {
    var preparing = false
    var cancelled = false
    var restored = false
    val interceptor =
      interceptor(
        awaitResult = { awaitCancellation() },
        onCancel = { cancelled = true },
        restoreInput = { restored = true },
        onPreparationChanged = { preparing = it },
      )
    val preparation = async(start = CoroutineStart.UNDISPATCHED) { interceptor.prepare() }
    assertTrue(preparing)

    preparation.cancelAndJoin()

    assertFalse(preparing)
    assertTrue(cancelled)
    assertTrue(restored)
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
  onPreparationChanged: (Boolean) -> Unit = {},
  savePendingChanges: suspend () -> Boolean = { true },
  awaitProtection:
    suspend (DocumentEditingStop, (DocumentSaveState) -> Unit) -> EditingCheckpointResult =
    { _, _ ->
      awaitCancellation()
    },
  resolveDecision: suspend (State<DocumentSaveState>) -> RouteRemovalDecision = {
    RouteRemovalDecision.CancelRemoval
  },
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
    resolveDecision = resolveDecision,
    awaitProtection = awaitProtection,
    delayedFeedbackMillis = delayedFeedbackMillis,
    checkpointWatchdogMillis = checkpointWatchdogMillis,
    showDelayedFeedback = showDelayedFeedback,
    hideDelayedFeedback = hideDelayedFeedback,
    onPreparationChanged = onPreparationChanged,
    savePendingChanges = savePendingChanges,
  )
