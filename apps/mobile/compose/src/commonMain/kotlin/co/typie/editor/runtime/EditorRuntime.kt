package co.typie.editor.runtime

import androidx.compose.runtime.Stable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.touchlab.kermit.Logger
import co.typie.editor.DocumentEditingSession
import co.typie.editor.Editor
import co.typie.editor.unwrapEditorFailureSignal
import io.sentry.kotlin.multiplatform.Sentry
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

@Stable
class EditorRuntime(private val uiScope: CoroutineScope) {
  private data class Failure(val error: Throwable, val editor: Editor?)

  private sealed interface Attachment {
    val editor: Editor
  }

  private class EditorOnly(override val editor: Editor) : Attachment

  private class DocumentSession(val session: DocumentEditingSession) : Attachment {
    override val editor: Editor
      get() = session.editor
  }

  private var attachment by mutableStateOf<Attachment?>(null)
  private var failureState by mutableStateOf<Failure?>(null)

  val editor: Editor?
    get() = attachment?.editor

  internal val session: DocumentEditingSession?
    get() = (attachment as? DocumentSession)?.session

  val failure: Throwable?
    get() = failureState?.error

  // The editor is already disposed; retain it only to keep its last committed surfaces composed
  // until the failure is cleared.
  internal val failedEditor: Editor?
    get() = failureState?.editor

  val canCreateEditor: Boolean
    get() = editor == null && failure == null

  fun attach(editor: Editor) {
    if (failure != null) {
      editor.dispose()
      return
    }
    val current = attachment
    if (current?.editor === editor) return

    dispose(current)
    attachment = EditorOnly(editor)
  }

  internal fun attach(session: DocumentEditingSession) {
    if (failure != null) {
      session.stop()
      session.editor.dispose()
      return
    }

    val current = attachment
    if ((current as? DocumentSession)?.session === session) return
    check(current?.editor !== session.editor) {
      "An attached editor cannot be rebound to a document editing session"
    }

    dispose(current)
    attachment = DocumentSession(session)
  }

  fun clear(editor: Editor? = null) {
    val current = attachment
    if (editor != null && current?.editor !== editor) return

    attachment = null
    dispose(current)
  }

  internal fun clear(session: DocumentEditingSession) {
    val current = attachment as? DocumentSession ?: return
    if (current.session !== session) return

    attachment = null
    dispose(current)
  }

  fun fail(error: Throwable) {
    val failure = failureSource(error)
    uiScope.launch { setFailed(failure) }
  }

  fun fail(editor: Editor, error: Throwable) {
    val failure = failureSource(error)
    uiScope.launch {
      if (this@EditorRuntime.editor !== editor) {
        return@launch
      }
      setFailed(failure)
    }
  }

  internal fun fail(session: DocumentEditingSession, error: Throwable) {
    val failure = failureSource(error)
    uiScope.launch {
      if (this@EditorRuntime.session !== session) {
        return@launch
      }
      setFailed(failure)
    }
  }

  private fun failureSource(error: Throwable): Throwable =
    error.unwrapEditorFailureSignal().also { failure ->
      if (failure is CancellationException) throw failure
    }

  private fun setFailed(error: Throwable) {
    if (failure != null) {
      return
    }

    runCatching {
      val detail = error.cause?.let { "$error; cause=$it" } ?: error.toString()
      Logger.e(error) { "Editor failed: $detail" }
    }
    runCatching { Sentry.captureException(error) }
    failureState = Failure(error = error, editor = editor)
    clear()
  }

  private fun dispose(attachment: Attachment?) {
    (attachment as? DocumentSession)?.session?.stop()
    attachment?.editor?.dispose()
  }

  fun clearFailure() {
    failureState = null
  }

  fun focus(): Boolean = editor?.focus() == true

  fun blur() {
    editor?.blur()
  }

  fun deactivateScene() {
    editor?.deactivateScene()
  }
}

val LocalEditorRuntime = compositionLocalOf<EditorRuntime> { error("No EditorRuntime provided") }
