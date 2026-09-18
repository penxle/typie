package co.typie.editor

import androidx.compose.runtime.State
import androidx.compose.runtime.mutableStateOf
import co.touchlab.kermit.Logger
import io.sentry.kotlin.multiplatform.Sentry
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.selects.select
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull

internal enum class DocumentReloadFailureDecision {
  Retry,
  Discard,
  Continue,
}

internal enum class DocumentProtectedReloadResult {
  Replaced,
  NotCurrent,
  SessionStopped,
}

private sealed interface FailureResolution {
  data class Decision(val decision: DocumentReloadFailureDecision) : FailureResolution

  data object SessionStopped : FailureResolution
}

internal suspend fun runProtectedDocumentReload(
  session: DocumentEditingSession,
  finalizeInput: () -> Unit,
  onStopAcquired: () -> Unit = {},
  showDelayedFeedback: () -> Unit = {},
  hideDelayedFeedback: () -> Unit = {},
  resolveFailure: suspend (State<DocumentSaveState>) -> DocumentReloadFailureDecision,
  replaceIfCurrent: (DocumentEditingSession) -> Boolean,
  delayedFeedbackMillis: Long = 350,
  checkpointWatchdogMillis: Long = 3_000,
): DocumentProtectedReloadResult {
  finalizeInput()
  val stop = session.beginStop()
  try {
    onStopAcquired()
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
          awaitFailureResolution(session = session, stop = stop, resolveFailure = resolveFailure)
      ) {
        is FailureResolution.Decision ->
          when (resolution.decision) {
            DocumentReloadFailureDecision.Discard,
            DocumentReloadFailureDecision.Continue -> return replaceExact(session, replaceIfCurrent)
            DocumentReloadFailureDecision.Retry -> {
              // TODO: 저장 실패 상태 인디케이터가 생기면 reload 실패 시 admission을 다시 열고
              // `계속 편집`을 제공한다. 현재는 실패 상태를 숨기지 않기 위해 재시도 modal로 막는다.
            }
          }
        FailureResolution.SessionStopped -> return DocumentProtectedReloadResult.SessionStopped
      }

      when (
        withDelayedFeedback(
          delayMillis = delayedFeedbackMillis,
          timeoutMillis = checkpointWatchdogMillis,
          show = showDelayedFeedback,
          hide = hideDelayedFeedback,
        ) {
          stop.retryCheckpoint()
        }
      ) {
        EditingCheckpointResult.Protected -> Unit
        EditingCheckpointResult.SessionStopped ->
          return DocumentProtectedReloadResult.SessionStopped
        EditingCheckpointResult.StopCancelled -> return DocumentProtectedReloadResult.NotCurrent
        is EditingCheckpointResult.EditFailed,
        is EditingCheckpointResult.ProtectionFailed,
        null -> {}
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
  stop: DocumentEditingStop,
  resolveFailure: suspend (State<DocumentSaveState>) -> DocumentReloadFailureDecision,
): FailureResolution = coroutineScope {
  val saveState = mutableStateOf(DocumentSaveState.Pending)
  val decision = async { FailureResolution.Decision(resolveFailure(saveState)) }
  val protection = async {
    when (session.awaitProtectedCheckpoint(stop) { saveState.value = it }) {
      EditingCheckpointResult.Protected -> {
        saveState.value = DocumentSaveState.Protected
        awaitCancellation()
      }
      EditingCheckpointResult.SessionStopped,
      EditingCheckpointResult.StopCancelled -> FailureResolution.SessionStopped
      is EditingCheckpointResult.EditFailed,
      is EditingCheckpointResult.ProtectionFailed -> {
        saveState.value = DocumentSaveState.Failed
        awaitCancellation()
      }
    }
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

private fun replaceExact(
  session: DocumentEditingSession,
  replaceIfCurrent: (DocumentEditingSession) -> Boolean,
): DocumentProtectedReloadResult =
  if (replaceIfCurrent(session)) {
    DocumentProtectedReloadResult.Replaced
  } else {
    DocumentProtectedReloadResult.NotCurrent
  }
