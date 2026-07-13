package co.typie.editor.runtime

import androidx.compose.runtime.Stable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.touchlab.kermit.Logger
import co.typie.editor.DocumentEditingSession
import co.typie.editor.Editor
import io.sentry.kotlin.multiplatform.Sentry
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

@Stable
class EditorRuntime(private val uiScope: CoroutineScope) {
  private sealed interface Attachment {
    val editor: Editor
  }

  private class EditorOnly(override val editor: Editor) : Attachment

  private class DocumentSession(val session: DocumentEditingSession) : Attachment {
    override val editor: Editor
      get() = session.editor
  }

  private var attachment by mutableStateOf<Attachment?>(null)

  val editor: Editor?
    get() = attachment?.editor

  internal val session: DocumentEditingSession?
    get() = (attachment as? DocumentSession)?.session

  var error by mutableStateOf<Throwable?>(null)
    private set

  val canCreateEditor: Boolean
    get() = editor == null && error == null

  fun attach(editor: Editor) {
    if (error != null) {
      editor.dispose()
      return
    }
    val current = attachment
    if (current?.editor === editor) return

    dispose(current)
    attachment = EditorOnly(editor)
  }

  internal fun attach(session: DocumentEditingSession) {
    if (error != null) {
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

  fun reportError(error: Throwable) {
    if (error is CancellationException) {
      throw error
    }
    uiScope.launch { fail(error) }
  }

  fun reportError(editor: Editor, error: Throwable) {
    if (error is CancellationException) {
      throw error
    }
    uiScope.launch {
      if (this@EditorRuntime.editor !== editor) {
        return@launch
      }
      fail(error)
    }
  }

  private fun fail(error: Throwable) {
    if (this.error != null) {
      return
    }

    val detail = error.cause?.let { "$error; cause=$it" } ?: error.toString()
    Logger.e(error) { "Editor failed: $detail" }
    Sentry.captureException(error)
    this.error = error
    clear()
  }

  private fun dispose(attachment: Attachment?) {
    (attachment as? DocumentSession)?.session?.stop()
    attachment?.editor?.dispose()
  }

  fun clearError() {
    error = null
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
