package co.typie.editor

import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

class EditorState {
  var editor by mutableStateOf<Editor?>(null)
}

val LocalEditorState = compositionLocalOf<EditorState> { error("No EditorState provided") }
