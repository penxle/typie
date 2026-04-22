package co.typie.screen.editor.editor.state

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import co.typie.ui.state.rememberScrollState

@Composable
internal fun rememberEditorScreenState(key: Any): EditorScreenState {
  val scrollState = rememberScrollState(key = "editor-screen:$key")
  val horizontalScrollState = rememberScrollState(key = "editor-screen:$key:horizontal")
  return remember(key, scrollState, horizontalScrollState) {
    EditorScreenState(scrollState = scrollState, horizontalScrollState = horizontalScrollState)
  }
}
