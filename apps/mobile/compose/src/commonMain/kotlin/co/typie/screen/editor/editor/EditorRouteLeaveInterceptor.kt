package co.typie.screen.editor.editor

import co.typie.editor.DocumentEditingClose
import co.typie.editor.LocalDurabilityResult
import co.typie.navigation.RouteRemovalDecision
import co.typie.navigation.RouteRemovalInterceptor
import co.typie.navigation.RouteRemovalPreparation
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.withTimeoutOrNull

internal class EditorRouteLeaveInterceptor(
  private val finalizeInput: () -> Unit,
  private val restoreInput: () -> Unit,
  private val beginClose: () -> DocumentEditingClose,
  private val resolveDecision: suspend () -> RouteRemovalDecision,
  private val delayedFeedbackMillis: Long = DEFAULT_DELAYED_FEEDBACK_MILLIS,
  private val localDurabilityWatchdogMillis: Long = DEFAULT_LOCAL_DURABILITY_WATCHDOG_MILLIS,
  private val showDelayedFeedback: () -> Unit = {},
  private val hideDelayedFeedback: () -> Unit = {},
) : RouteRemovalInterceptor {
  private var close: DocumentEditingClose? = null
  private var delayedFeedbackVisible = false

  override suspend fun prepare(onDelayed: (suspend () -> Unit)?): RouteRemovalPreparation {
    check(close == null) { "Editor leave preparation is already active" }
    val currentClose =
      try {
        finalizeInput()
        beginClose()
      } catch (throwable: Throwable) {
        try {
          restoreInput()
        } catch (cleanupFailure: Throwable) {
          if (cleanupFailure !== throwable) throwable.addSuppressed(cleanupFailure)
        }
        throw throwable
      }
    close = currentClose

    return if (awaitLocalDurability(currentClose, onDelayed)) {
      RouteRemovalPreparation.Ready
    } else {
      RouteRemovalPreparation.NeedsDecision
    }
  }

  override suspend fun resolveDecision(): RouteRemovalDecision = resolveDecision.invoke()

  override suspend fun rollback() {
    val currentClose = close ?: return
    close = null
    try {
      currentClose.cancel()
    } finally {
      try {
        hideDelayedFeedbackIfVisible()
      } finally {
        restoreInput()
      }
    }
  }

  private suspend fun awaitLocalDurability(
    close: DocumentEditingClose,
    onDelayed: (suspend () -> Unit)? = null,
  ): Boolean {
    suspend fun awaitResult(): Boolean = awaitLocalDurabilityWithOneRetry(close)

    return try {
      withTimeoutOrNull(localDurabilityWatchdogMillis) {
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

  private suspend fun awaitLocalDurabilityWithOneRetry(close: DocumentEditingClose): Boolean {
    val first = close.awaitLocalDurability()
    return when {
      first == LocalDurabilityResult.Captured -> true
      first is LocalDurabilityResult.CaptureFailed ->
        close.retryLocalDurability() == LocalDurabilityResult.Captured
      else -> false
    }
  }

  private fun hideDelayedFeedbackIfVisible() {
    if (!delayedFeedbackVisible) return
    delayedFeedbackVisible = false
    hideDelayedFeedback()
  }

  private companion object {
    const val DEFAULT_DELAYED_FEEDBACK_MILLIS = 350L
    const val DEFAULT_LOCAL_DURABILITY_WATCHDOG_MILLIS = 3_000L
  }
}
