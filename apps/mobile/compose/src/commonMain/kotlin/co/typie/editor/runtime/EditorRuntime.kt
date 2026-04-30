package co.typie.editor.runtime

import androidx.compose.runtime.Stable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.touchlab.kermit.Logger
import co.typie.editor.Editor
import io.sentry.kotlin.multiplatform.Sentry
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

@Stable
class EditorRuntime(private val uiScope: CoroutineScope) {
  var editor by mutableStateOf<Editor?>(null)
    private set

  var error by mutableStateOf<Throwable?>(null)
    private set

  val canCreateEditor: Boolean
    get() = editor == null && error == null

  fun attach(editor: Editor) {
    if (error != null) {
      editor.dispose()
      return
    }
    if (this.editor === editor) {
      return
    }

    this.editor?.dispose()
    this.editor = editor
  }

  fun clear(editor: Editor? = null) {
    if (editor != null && this.editor !== editor) {
      return
    }

    this.editor?.dispose()
    this.editor = null
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

    Logger.e(error) { "Editor failed" }
    Sentry.captureException(error)
    this.error = error
    clear()
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
