package co.typie.editor

import androidx.compose.runtime.State
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.snapshotFlow
import co.touchlab.kermit.Logger
import io.sentry.kotlin.multiplatform.Sentry
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.transformLatest
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
  canPresent: () -> Boolean = { true },
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
    val initialResult =
      withDelayedFeedback(
        canPresent = canPresent,
        delayMillis = delayedFeedbackMillis,
        timeoutMillis = checkpointWatchdogMillis,
        show = showDelayedFeedback,
        hide = hideDelayedFeedback,
      ) {
        stop.awaitCheckpoint()
      }

    when (initialResult) {
      EditingCheckpointResult.Protected ->
        return replaceExact(session, canPresent, replaceIfCurrent)
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
            stop = stop,
            resolveFailure = { state ->
              withReloadPresentation(canPresent) { resolveFailure(state) }
            },
          )
      ) {
        is FailureResolution.Decision ->
          when (resolution.decision) {
            DocumentReloadFailureDecision.Discard,
            DocumentReloadFailureDecision.Continue ->
              return replaceExact(session, canPresent, replaceIfCurrent)
            DocumentReloadFailureDecision.Retry -> {
              // Sync reload keeps admission closed until protection or explicit discard.
            }
          }
        FailureResolution.SessionStopped -> return DocumentProtectedReloadResult.SessionStopped
      }

      when (
        withDelayedFeedback(
          canPresent = canPresent,
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

@OptIn(ExperimentalCoroutinesApi::class)
private suspend fun <T> withReloadPresentation(
  canPresent: () -> Boolean,
  block: suspend () -> T,
): T = snapshotFlow(canPresent).transformLatest { allowed -> if (allowed) emit(block()) }.first()

private suspend fun <T> withDelayedFeedback(
  canPresent: () -> Boolean,
  delayMillis: Long,
  timeoutMillis: Long,
  show: () -> Unit,
  hide: () -> Unit,
  block: suspend () -> T,
): T? = coroutineScope {
  val feedback =
    launch(start = CoroutineStart.UNDISPATCHED) {
      snapshotFlow(canPresent).collectLatest { allowed ->
        if (allowed) {
          delay(delayMillis)
          try {
            runReloadFeedback("show", show)
            awaitCancellation()
          } finally {
            runReloadFeedback("hide", hide)
          }
        }
      }
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
        session.awaitStopped()
        FailureResolution.SessionStopped
      }
      EditingCheckpointResult.SessionStopped,
      EditingCheckpointResult.StopCancelled -> FailureResolution.SessionStopped
      is EditingCheckpointResult.EditFailed,
      is EditingCheckpointResult.ProtectionFailed -> {
        saveState.value = DocumentSaveState.Failed
        session.awaitStopped()
        FailureResolution.SessionStopped
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

private suspend fun replaceExact(
  session: DocumentEditingSession,
  canPresent: () -> Boolean,
  replaceIfCurrent: (DocumentEditingSession) -> Boolean,
): DocumentProtectedReloadResult = coroutineScope {
  val stopped = async { session.awaitStopped() }
  try {
    var result: DocumentProtectedReloadResult?
    do {
      val presentation = async { snapshotFlow(canPresent).first { it } }
      try {
        result = select {
          stopped.onAwait { DocumentProtectedReloadResult.SessionStopped }
          presentation.onAwait {
            // Observation can precede this continuation. Recheck without suspending before
            // replacement, and wait again if route removal has reclaimed priority.
            when {
              session.isStopped -> DocumentProtectedReloadResult.SessionStopped
              !canPresent() -> null
              replaceIfCurrent(session) -> DocumentProtectedReloadResult.Replaced
              else -> DocumentProtectedReloadResult.NotCurrent
            }
          }
        }
      } finally {
        presentation.cancel()
      }
    } while (result == null)
    result
  } finally {
    stopped.cancel()
  }
}
