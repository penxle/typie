package co.typie.screen.editor.editor.subpane

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.editor.ffi.Axis
import co.typie.editor.ffi.Selection

internal sealed interface EditorSubPane {
  data object RelatedNotes : EditorSubPane

  data object Comments : EditorSubPane

  data class TableAxisActions(
    val target: EditorTableAxisActionsTarget,
    val openedSelection: Selection?,
  ) : EditorSubPane
}

private enum class EditorSubPaneSurface {
  RelatedNotes,
  Comments,
  TableAxisActions,
}

private val EditorSubPane.surface: EditorSubPaneSurface
  get() =
    when (this) {
      EditorSubPane.RelatedNotes -> EditorSubPaneSurface.RelatedNotes
      EditorSubPane.Comments -> EditorSubPaneSurface.Comments
      is EditorSubPane.TableAxisActions -> EditorSubPaneSurface.TableAxisActions
    }

internal data class EditorTableAxisActionsTarget(
  val tableId: String,
  val axis: Axis,
  val index: Int,
  val count: Int,
)

internal enum class EditorSubPaneVisibleAreaMode {
  ResizeEditor,
  OverlayEditor,
}

internal data class EditorSubPaneLayoutInfo(
  val pane: EditorSubPane,
  val visibleHeight: Float,
  val visibleAreaMode: EditorSubPaneVisibleAreaMode,
)

@Stable
internal class EditorSubPaneState {
  var active by mutableStateOf<EditorSubPane?>(null)
    private set

  var layoutInfo by mutableStateOf<EditorSubPaneLayoutInfo?>(null)
    private set

  var dismissRequestVersion by mutableIntStateOf(0)
    private set

  private var dismissalInProgress by mutableStateOf(false)

  val editorInputBlocked: Boolean
    get() = active != null && !dismissalInProgress

  fun open(pane: EditorSubPane) {
    val previous = active
    active = pane
    dismissalInProgress = false
    dismissRequestVersion = 0
    layoutInfo =
      if (previous?.surface == pane.surface) {
        layoutInfo?.copy(pane = pane)
      } else {
        null
      }
  }

  fun dismiss() {
    active = null
    layoutInfo = null
    dismissalInProgress = false
    dismissRequestVersion = 0
  }

  fun isActive(pane: EditorSubPane): Boolean = active == pane

  fun beginDismiss() {
    if (active != null) {
      dismissalInProgress = true
    }
  }

  fun requestDismiss() {
    if (active != null) {
      beginDismiss()
      dismissRequestVersion += 1
    }
  }

  fun dismissTableAxisActionsIfSelectionChanged(selection: Selection?) {
    val pane = active as? EditorSubPane.TableAxisActions ?: return
    if (pane.openedSelection != selection) {
      requestDismiss()
    }
  }

  fun updateLayoutInfo(info: EditorSubPaneLayoutInfo) {
    if (info.pane != active) {
      return
    }

    layoutInfo = info
  }

  fun clearLayoutInfo(pane: EditorSubPane) {
    if (layoutInfo?.pane == pane) {
      layoutInfo = null
    }
  }
}
