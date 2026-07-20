package co.typie.screen.editor.editor

import co.typie.editor.DocumentEditingStop
import co.typie.editor.EditingCheckpointResult
import co.typie.navigation.RouteRemovalDecision
import co.typie.navigation.RouteRemovalInterceptor
import co.typie.navigation.RouteRemovalPreparation
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull

internal class EditorRouteLeaveInterceptor(
  private val finalizeInput: () -> Unit,
  private val restoreInput: () -> Unit,
  private val beginStop: () -> DocumentEditingStop,
  private val onPreparationStarted: suspend () -> Unit = {},
  private val resumeReloadBeforeRollback: suspend () -> Boolean = { false },
  private val resolveDecision: suspend () -> RouteRemovalDecision,
  private val delayedFeedbackMillis: Long = DEFAULT_DELAYED_FEEDBACK_MILLIS,
  private val checkpointWatchdogMillis: Long = DEFAULT_CHECKPOINT_WATCHDOG_MILLIS,
  private val showDelayedFeedback: () -> Unit = {},
  private val hideDelayedFeedback: () -> Unit = {},
) : RouteRemovalInterceptor {
  private var stop: DocumentEditingStop? = null
  private var reloadPaused = false
  private var delayedFeedbackVisible = false

  override suspend fun prepare(onDelayed: (suspend () -> Unit)?): RouteRemovalPreparation {
    check(stop == null) { "Editor leave preparation is already active" }
    val currentStop =
      try {
        finalizeInput()
        beginStop()
      } catch (throwable: Throwable) {
        try {
          restoreInput()
        } catch (cleanupFailure: Throwable) {
          if (cleanupFailure !== throwable) throwable.addSuppressed(cleanupFailure)
        }
        throw throwable
      }
    stop = currentStop

    try {
      withContext(NonCancellable) {
        onPreparationStarted()
        reloadPaused = true
      }
      return if (awaitCheckpoint(currentStop, onDelayed)) {
        RouteRemovalPreparation.Ready
      } else {
        RouteRemovalPreparation.NeedsDecision
      }
    } catch (throwable: Throwable) {
      if (stop === currentStop) stop = null
      val shouldResumeReload = reloadPaused
      reloadPaused = false
      val failure =
        withContext(NonCancellable) { releaseStop(currentStop, shouldResumeReload, throwable) }
      throw failure ?: throwable
    }
  }

  override suspend fun resolveDecision(): RouteRemovalDecision = resolveDecision.invoke()

  override suspend fun rollback() {
    val currentStop = stop ?: return
    stop = null
    val shouldResumeReload = reloadPaused
    reloadPaused = false
    releaseStop(currentStop, shouldResumeReload, initialFailure = null)?.let { throw it }
  }

  private suspend fun releaseStop(
    currentStop: DocumentEditingStop,
    shouldResumeReload: Boolean,
    initialFailure: Throwable?,
  ): Throwable? {
    var reloadResumed = false
    var failure = initialFailure
    if (shouldResumeReload) {
      try {
        reloadResumed = resumeReloadBeforeRollback()
      } catch (throwable: Throwable) {
        failure = recordFailure(failure, throwable)
      }
    }
    return cleanup(currentStop, restore = !reloadResumed, failure)
  }

  private suspend fun awaitCheckpoint(
    stop: DocumentEditingStop,
    onDelayed: (suspend () -> Unit)? = null,
  ): Boolean {
    suspend fun awaitResult(): Boolean = stop.awaitCheckpoint() == EditingCheckpointResult.Protected

    return try {
      withTimeoutOrNull(checkpointWatchdogMillis) {
        if (onDelayed == null) return@withTimeoutOrNull awaitResult()

        coroutineScope {
          val result = async(start = CoroutineStart.UNDISPATCHED) { awaitResult() }
          val earlyResult = withTimeoutOrNull(delayedFeedbackMillis) { result.await() }
          if (earlyResult != null) return@coroutineScope earlyResult

          onDelayed()
          showDelayedFeedback()
          delayedFeedbackVisible = true
          result.await()
        }
      } == true
    } finally {
      hideDelayedFeedbackIfVisible()
    }
  }

  private fun hideDelayedFeedbackIfVisible() {
    if (!delayedFeedbackVisible) return
    delayedFeedbackVisible = false
    hideDelayedFeedback()
  }

  private fun cleanup(
    currentStop: DocumentEditingStop,
    restore: Boolean,
    initialFailure: Throwable?,
  ): Throwable? {
    var failure = initialFailure
    try {
      currentStop.cancel()
    } catch (throwable: Throwable) {
      failure = recordFailure(failure, throwable)
    }
    try {
      hideDelayedFeedbackIfVisible()
    } catch (throwable: Throwable) {
      failure = recordFailure(failure, throwable)
    }
    if (restore) {
      try {
        restoreInput()
      } catch (throwable: Throwable) {
        failure = recordFailure(failure, throwable)
      }
    }
    return failure
  }

  private fun recordFailure(primary: Throwable?, cleanupFailure: Throwable): Throwable {
    if (primary == null) return cleanupFailure
    if (cleanupFailure !== primary) primary.addSuppressed(cleanupFailure)
    return primary
  }

  private companion object {
    const val DEFAULT_DELAYED_FEEDBACK_MILLIS = 350L
    const val DEFAULT_CHECKPOINT_WATCHDOG_MILLIS = 3_000L
  }
}
