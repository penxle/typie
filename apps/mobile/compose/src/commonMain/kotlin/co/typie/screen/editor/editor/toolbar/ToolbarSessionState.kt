package co.typie.screen.editor.editor.toolbar

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import co.typie.screen.editor.editor.toolbar.contextual.TextOptionMode

@Composable
internal fun rememberEditorToolbarSessionState(key: Any? = Unit): EditorToolbarSessionState =
  remember(key) { EditorToolbarSessionState() }

@Stable
internal class EditorToolbarSessionState {
  var activeTextOptionMode by mutableStateOf<TextOptionMode?>(null)
  var modalActive by mutableStateOf(false)
  var secondaryToolbarInLayout by mutableStateOf(false)
}
