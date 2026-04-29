package co.typie.editor.runtime

import androidx.compose.runtime.Stable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.editor.Editor

@Stable
class EditorRuntime {
  var editor by mutableStateOf<Editor?>(null)
    private set

  fun attach(editor: Editor) {
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

  fun focus(): Boolean = editor?.focus() == true

  fun blur() {
    editor?.blur()
  }

  fun deactivateScene() {
    editor?.deactivateScene()
  }
}

val LocalEditorRuntime = compositionLocalOf<EditorRuntime> { error("No EditorRuntime provided") }
