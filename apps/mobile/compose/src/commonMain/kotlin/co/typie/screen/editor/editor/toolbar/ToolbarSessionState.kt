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
  val pageKey: EditorToolbarPageKey
  val ownerNodeId: String?
    get() = null

  data class TextOption(val mode: TextOptionMode) : EditorToolbarSecondary {
    override val pageKey = EditorToolbarPageKey.Text
  }

  data class ImageResize(val nodeId: String) : EditorToolbarSecondary {
    override val pageKey = EditorToolbarPageKey.Image
    override val ownerNodeId = nodeId
  }
}

@Stable
internal class EditorToolbarSessionState {
  var activeSecondaryToolbar by mutableStateOf<EditorToolbarSecondary?>(null)

  var activeTextOptionMode: TextOptionMode?
    get() = (activeSecondaryToolbar as? EditorToolbarSecondary.TextOption)?.mode
    set(value) {
      activeSecondaryToolbar = value?.let(EditorToolbarSecondary::TextOption)
    }

  var modalActive by mutableStateOf(false)
  var secondaryToolbarInLayout by mutableStateOf(false)

  fun toggleSecondaryToolbar(secondary: EditorToolbarSecondary) {
    activeSecondaryToolbar =
      if (activeSecondaryToolbar == secondary) {
        null
      } else {
        secondary
      }
  }

  fun clearSecondaryToolbarIfInvalid(
    currentPageKey: EditorToolbarPageKey?,
    selectedNodeId: String?,
  ) {
    activeSecondaryToolbar = activeSecondaryToolbar?.takeIf { secondary ->
      currentPageKey == secondary.pageKey &&
        (secondary.ownerNodeId == null || secondary.ownerNodeId == selectedNodeId)
    }
  }
}
