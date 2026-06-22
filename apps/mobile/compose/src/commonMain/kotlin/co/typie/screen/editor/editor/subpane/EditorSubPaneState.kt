package co.typie.screen.editor.editor.subpane

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

internal enum class EditorSubPaneKey {
  RelatedNotes,
  Spellcheck,
  AiFeedback,
}

internal enum class EditorSubPaneVisibleAreaMode {
  ResizeEditor,
  OverlayEditor,
}

internal data class EditorSubPaneLayoutInfo(
  val key: EditorSubPaneKey,
  val visibleHeight: Float,
  val visibleAreaMode: EditorSubPaneVisibleAreaMode,
)

@Stable
internal class EditorSubPaneState {
  var activeKey by mutableStateOf<EditorSubPaneKey?>(null)
    private set

  var layoutInfo by mutableStateOf<EditorSubPaneLayoutInfo?>(null)
    private set

  fun open(key: EditorSubPaneKey) {
    activeKey = key
    layoutInfo = null
  }

  fun dismiss() {
    activeKey = null
    layoutInfo = null
  }

  fun isActive(key: EditorSubPaneKey): Boolean = activeKey == key

  fun updateLayoutInfo(info: EditorSubPaneLayoutInfo) {
    if (info.key != activeKey) {
      return
    }

    layoutInfo = info
  }

  fun clearLayoutInfo(key: EditorSubPaneKey) {
    if (layoutInfo?.key == key) {
      layoutInfo = null
    }
  }
}
