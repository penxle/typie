package co.typie.screen.editor.editor.state

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import co.typie.editor.viewport.rememberEditorViewportState

@Composable
internal fun rememberEditorScreenState(key: Any): EditorScreenState {
  val viewportState = rememberEditorViewportState(key = "editor-screen:$key:viewport")
  return remember(key, viewportState) { EditorScreenState(viewportState = viewportState) }
}
