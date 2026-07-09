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

internal sealed interface EditorToolbarSecondary {
  data class TextOption(val mode: TextOptionMode) : EditorToolbarSecondary

  data class ImageResize(val nodeId: String) : EditorToolbarSecondary
}

internal data class SecondaryToolbarSession(
  val toolbar: EditorToolbarSecondary,
  val scope: EditorToolbarScope,
)

@Stable
internal class EditorToolbarSessionState {
  private var secondarySession by mutableStateOf<SecondaryToolbarSession?>(null)

  val activeSecondaryToolbar: EditorToolbarSecondary?
    get() = secondarySession?.toolbar

  val activeTextOptionMode: TextOptionMode?
    get() = (activeSecondaryToolbar as? EditorToolbarSecondary.TextOption)?.mode

  var modalActive by mutableStateOf(false)
  var secondaryToolbarInLayout by mutableStateOf(false)

  fun toggleSecondaryToolbar(secondary: EditorToolbarSecondary, scope: EditorToolbarScope) {
    val nextSession = SecondaryToolbarSession(toolbar = secondary, scope = scope)
    secondarySession =
      if (secondarySession == nextSession) {
        null
      } else {
        nextSession
      }
  }

  fun clearSecondaryToolbar() {
    secondarySession = null
  }

  fun clearSecondaryToolbarIfInvalid(currentScope: EditorToolbarScope?) {
    if (secondarySession?.scope != currentScope) {
      secondarySession = null
    }
  }
}
