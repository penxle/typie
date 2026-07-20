package co.typie.editor

import co.touchlab.kermit.Logger
import io.sentry.kotlin.multiplatform.Sentry
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.selects.select
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull

internal enum class DocumentReloadFailureDecision {
  Retry,
  Discard,
}

internal enum class DocumentProtectedReloadResult {
  Replaced,
  NotCurrent,
  SessionStopped,
}

private sealed interface FailureResolution {
  data class Decision(val decision: DocumentReloadFailureDecision) : FailureResolution

  data class ProtectionAdvanced(val active: Boolean) : FailureResolution
}

private sealed interface RecoveryResult {
  data object Protected : RecoveryResult

  data object SessionStopped : RecoveryResult

  data object StopCancelled : RecoveryResult

  data class TimedOut(val observedGeneration: Long) : RecoveryResult
}

internal suspend fun runProtectedDocumentReload(
  session: DocumentEditingSession,
  finalizeInput: () -> Unit,
  onStopAcquired: () -> Unit = {},
  showDelayedFeedback: () -> Unit = {},
  hideDelayedFeedback: () -> Unit = {},
  resolveFailure: suspend () -> DocumentReloadFailureDecision,
  replaceIfCurrent: (DocumentEditingSession) -> Boolean,
  delayedFeedbackMillis: Long = 350,
  checkpointWatchdogMillis: Long = 3_000,
  retryWindowMillis: Long = 3_000,
): DocumentProtectedReloadResult {
  finalizeInput()
  val stop = session.beginStop()
  try {
    onStopAcquired()
    var observedGeneration = session.protectionGeneration
    val initialResult =
      withDelayedFeedback(
        delayMillis = delayedFeedbackMillis,
        timeoutMillis = checkpointWatchdogMillis,
        show = showDelayedFeedback,
        hide = hideDelayedFeedback,
      ) {
        stop.awaitCheckpoint()
      }

    when (initialResult) {
      EditingCheckpointResult.Protected -> return replaceExact(session, replaceIfCurrent)
      EditingCheckpointResult.SessionStopped -> return DocumentProtectedReloadResult.SessionStopped
      EditingCheckpointResult.StopCancelled -> return DocumentProtectedReloadResult.NotCurrent
      is EditingCheckpointResult.EditFailed,
      is EditingCheckpointResult.ProtectionFailed,
      null -> {}
    }

    while (true) {
      when (
        val resolution =
          awaitFailureResolution(
            session = session,
            observedGeneration = observedGeneration,
            resolveFailure = resolveFailure,
          )
      ) {
        is FailureResolution.Decision ->
          when (resolution.decision) {
            DocumentReloadFailureDecision.Discard -> return replaceExact(session, replaceIfCurrent)
            DocumentReloadFailureDecision.Retry -> {
              // TODO: 저장 실패 상태 인디케이터가 생기면 reload 실패 시 admission을 다시 열고
              // `계속 편집`을 제공한다. 현재는 실패 상태를 숨기지 않기 위해 재시도 modal로 막는다.
            }
          }
        is FailureResolution.ProtectionAdvanced -> {
          if (!resolution.active) return DocumentProtectedReloadResult.SessionStopped
        }
      }

      observedGeneration = session.protectionGeneration
      when (
        val recovery =
          runRecoveryWindow(
            session = session,
            stop = stop,
            initialObservedGeneration = observedGeneration,
            delayedFeedbackMillis = delayedFeedbackMillis,
            retryWindowMillis = retryWindowMillis,
            showDelayedFeedback = showDelayedFeedback,
            hideDelayedFeedback = hideDelayedFeedback,
          )
      ) {
        RecoveryResult.Protected -> return replaceExact(session, replaceIfCurrent)
        RecoveryResult.SessionStopped -> return DocumentProtectedReloadResult.SessionStopped
        RecoveryResult.StopCancelled -> return DocumentProtectedReloadResult.NotCurrent
        is RecoveryResult.TimedOut -> observedGeneration = recovery.observedGeneration
      }
    }
  } finally {
    try {
      runReloadFeedback("hide", hideDelayedFeedback)
    } finally {
      stop.cancel()
    }
  }
}

private suspend fun <T> withDelayedFeedback(
  delayMillis: Long,
  timeoutMillis: Long,
  show: () -> Unit,
  hide: () -> Unit,
  block: suspend () -> T,
): T? = coroutineScope {
  val feedback =
    launch(start = CoroutineStart.UNDISPATCHED) {
      delay(delayMillis)
      runReloadFeedback("show", show)
    }
  try {
    withTimeoutOrNull(timeoutMillis) { block() }
  } finally {
    feedback.cancel()
    try {
      withContext(NonCancellable) { feedback.join() }
    } finally {
      runReloadFeedback("hide", hide)
    }
  }
}

private inline fun runReloadFeedback(stage: String, block: () -> Unit) {
  try {
    block()
  } catch (e: CancellationException) {
    throw e
  } catch (e: Throwable) {
    runCatching { Logger.w(e) { "Document protected reload feedback failed: $stage" } }
    runCatching { Sentry.captureException(e) }
  }
}

private suspend fun awaitFailureResolution(
  session: DocumentEditingSession,
  observedGeneration: Long,
  resolveFailure: suspend () -> DocumentReloadFailureDecision,
): FailureResolution = coroutineScope {
  val decision = async { FailureResolution.Decision(resolveFailure()) }
  val protection = async {
    FailureResolution.ProtectionAdvanced(session.awaitProtectionAfter(observedGeneration))
  }
  try {
    select {
      decision.onAwait { it }
      protection.onAwait { it }
    }
  } finally {
    decision.cancel()
    protection.cancel()
    withContext(NonCancellable) {
      try {
        decision.join()
      } finally {
        protection.join()
      }
    }
  }
}

private suspend fun runRecoveryWindow(
  session: DocumentEditingSession,
  stop: DocumentEditingStop,
  initialObservedGeneration: Long,
  delayedFeedbackMillis: Long,
  retryWindowMillis: Long,
  showDelayedFeedback: () -> Unit,
  hideDelayedFeedback: () -> Unit,
): RecoveryResult {
  var observedGeneration = initialObservedGeneration
  suspend fun retryUntilProtected(): RecoveryResult {
    while (true) {
      when (stop.retryCheckpoint()) {
        EditingCheckpointResult.Protected -> return RecoveryResult.Protected
        EditingCheckpointResult.SessionStopped -> return RecoveryResult.SessionStopped
        EditingCheckpointResult.StopCancelled -> return RecoveryResult.StopCancelled
        is EditingCheckpointResult.EditFailed,
        is EditingCheckpointResult.ProtectionFailed -> {}
      }
      if (!session.awaitProtectionAfter(observedGeneration)) {
        return RecoveryResult.SessionStopped
      }
      observedGeneration = session.protectionGeneration
    }
  }
  return withDelayedFeedback(
    delayMillis = delayedFeedbackMillis,
    timeoutMillis = retryWindowMillis,
    show = showDelayedFeedback,
    hide = hideDelayedFeedback,
  ) {
    retryUntilProtected()
  } ?: RecoveryResult.TimedOut(observedGeneration)
}

private fun replaceExact(
  session: DocumentEditingSession,
  replaceIfCurrent: (DocumentEditingSession) -> Boolean,
): DocumentProtectedReloadResult =
  if (replaceIfCurrent(session)) {
    DocumentProtectedReloadResult.Replaced
  } else {
    DocumentProtectedReloadResult.NotCurrent
  }
