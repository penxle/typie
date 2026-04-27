package co.typie.screen.editor.editor.toolbar

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

@Composable
internal fun rememberEditorToolbarBottomState(): EditorToolbarBottomState = remember {
  EditorToolbarBottomState()
}

@Stable
internal class EditorToolbarBottomState {
  var activePanel by mutableStateOf<EditorToolbarBottomPanelKey?>(null)
    private set

  var rememberedKeyboardInset by mutableStateOf(0.dp)
    private set

  val textInputSessionEnabled: Boolean
    get() = activePanel == null

  fun visibleImeInset(imeBottom: Dp, safeBottomInset: Dp): Dp =
    if (activePanel == null) {
      maxOf(imeBottom, rememberedKeyboardInset)
    } else {
      bottomPanelInset(safeBottomInset)
    }

  fun toolbarVisible(visible: Boolean, editorFocused: Boolean): Boolean =
    visible && (editorFocused || activePanel != null || rememberedKeyboardInset > 0.dp)

  fun inputBottomInset(imeBottom: Dp, safeBottomInset: Dp): Dp =
    maxOf(imeBottom, rememberedKeyboardInset, safeBottomInset)

  fun bottomPanelHeight(safeBottomInset: Dp): Dp =
    (bottomPanelInset(safeBottomInset) - safeBottomInset - ToolbarBottomPanelGap).coerceAtLeast(
      0.dp
    )

  fun openPanel(panel: EditorToolbarBottomPanelKey, imeBottom: Dp, safeBottomInset: Dp) {
    if (activePanel == null) {
      rememberedKeyboardInset =
        maxOf(
          rememberedKeyboardInset,
          resolveRememberedKeyboardInset(imeBottom = imeBottom, safeBottomInset = safeBottomInset),
        )
    }
    activePanel = panel
  }

  fun closePanel() {
    activePanel = null
  }

  fun reset() {
    activePanel = null
    rememberedKeyboardInset = 0.dp
  }

  fun clearRememberedKeyboardInsetIfRestored(imeBottom: Dp) {
    if (
      activePanel == null && rememberedKeyboardInset > 0.dp && imeBottom >= rememberedKeyboardInset
    ) {
      rememberedKeyboardInset = 0.dp
    }
  }

  private fun bottomPanelInset(safeBottomInset: Dp): Dp =
    if (rememberedKeyboardInset > safeBottomInset + ToolbarBottomPanelGap) {
      rememberedKeyboardInset
    } else {
      safeBottomInset + ToolbarBottomPanelGap + ToolbarBottomPanelHeight
    }
}

private fun resolveRememberedKeyboardInset(imeBottom: Dp, safeBottomInset: Dp): Dp =
  if (imeBottom > safeBottomInset) imeBottom else 0.dp
