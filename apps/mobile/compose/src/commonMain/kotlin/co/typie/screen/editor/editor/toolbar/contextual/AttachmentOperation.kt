package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.runtime.withFrameNanos
import co.touchlab.kermit.Logger
import co.typie.domain.blob.BlobUploadException
import co.typie.network.isRecoverableNetworkError
import co.typie.screen.editor.editor.toolbar.EditorToolbarSessionState
import io.sentry.kotlin.multiplatform.Sentry
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

internal enum class AttachmentKind {
  Image,
  File,
  Embed,
}

internal enum class AttachmentFailureStage {
  PersistAsset,
  CommitDocument,
}

internal class AttachmentException(val stage: AttachmentFailureStage, cause: Throwable) :
  RuntimeException("Attachment failed at $stage", cause)

internal suspend fun <Asset> completeAttachmentOperation(
  persist: suspend () -> Asset,
  isCurrent: () -> Boolean = { true },
  cache: (Asset) -> Unit,
  commit: suspend (Asset) -> Unit,
  clearPending: () -> Unit,
): Asset? {
  try {
    val asset = withAttachmentStage(AttachmentFailureStage.PersistAsset) { persist() }
    if (!isCurrent()) {
      return null
    }
    withAttachmentStage(AttachmentFailureStage.PersistAsset) { cache(asset) }
    withAttachmentStage(AttachmentFailureStage.CommitDocument) { commit(asset) }
    clearPending()
    return asset
  } catch (error: CancellationException) {
    throw error
  } catch (error: Throwable) {
    clearPending()
    throw error
  }
}

private suspend inline fun <T> withAttachmentStage(
  stage: AttachmentFailureStage,
  block: () -> T,
): T =
  try {
    block()
  } catch (error: CancellationException) {
    throw error
  } catch (error: BlobUploadException) {
    throw error
  } catch (error: AttachmentException) {
    throw error
  } catch (error: Throwable) {
    throw AttachmentException(stage = stage, cause = error)
  }

internal fun reportAttachmentFailure(kind: AttachmentKind, error: Throwable) {
  val stage =
    when (error) {
      is BlobUploadException -> error.stage.name
      is AttachmentException -> error.stage.name
      else -> "Unknown"
    }
  val cause = error.cause ?: error
  Logger.e(cause) { "Attachment failed: kind=${kind.name}, stage=$stage" }
  if (!cause.isRecoverableNetworkError()) {
    Sentry.captureException(cause)
  }
}

@Suppress("TooGenericExceptionCaught")
internal fun launchAttachmentPicker(
  scope: CoroutineScope,
  sessionState: EditorToolbarSessionState,
  requestIsCurrent: () -> Boolean,
  clearRequest: () -> Unit,
  launchPicker: () -> Unit,
  onFailure: () -> Unit,
) {
  sessionState.pickerInputActive = true
  scope.launch {
    try {
      // Let the toolbar observe blocked input before the native picker hides the IME.
      withFrameNanos {}
      if (requestIsCurrent()) {
        launchPicker()
      } else {
        sessionState.pickerInputActive = false
      }
    } catch (error: CancellationException) {
      throw error
    } catch (_: Exception) {
      if (requestIsCurrent()) {
        clearRequest()
        sessionState.pickerInputActive = false
        onFailure()
      }
    }
  }
}
