package co.typie.screen.editor.editor.toolbar

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import co.typie.ext.ime
import co.typie.screen.editor.editor.toolbar.bottom.EditorToolbarBottomPanel
import co.typie.ui.component.ResponsiveContainerDefaults

@OptIn(ExperimentalComposeUiApi::class)
@Composable
internal fun EditorToolbarHost(
  editorFocused: Boolean,
  visible: Boolean,
  safeBottomInset: Dp,
  bottomState: EditorToolbarBottomState,
  onEditorFocusRequest: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val imeBottom = WindowInsets.ime.asPaddingValues().calculateBottomPadding()
  val focusManager = LocalFocusManager.current
  val keyboardController = LocalSoftwareKeyboardController.current
  val pages = rememberEditorToolbarPages()
  val activeBottomPanel = bottomState.activePanel
  val toolbarVisible = bottomState.toolbarVisible(visible = visible, editorFocused = editorFocused)
  val bottomInset =
    bottomState.toolbarBottomInset(imeBottom = imeBottom, safeBottomInset = safeBottomInset)
  val bottomPanelHeight = bottomState.bottomPanelHeight(safeBottomInset = safeBottomInset)

  fun restoreEditorInput() {
    if (bottomState.activePanel != null) {
      bottomState.closePanel()
    }
    onEditorFocusRequest()
    keyboardController?.show()
  }

  fun dismissEditorInput() {
    if (bottomState.activePanel != null) {
      restoreEditorInput()
    } else if (editorFocused) {
      focusManager.clearFocus()
    } else {
      keyboardController?.hide()
    }
  }

  fun toggleBottomPanel(panel: EditorToolbarBottomPanelKey) {
    if (bottomState.activePanel == panel) {
      restoreEditorInput()
      return
    }

    if (bottomState.activePanel == null) {
      keyboardController?.hide()
    }
    bottomState.openPanel(panel = panel, imeBottom = imeBottom, safeBottomInset = safeBottomInset)
  }

  LaunchedEffect(visible) {
    if (!visible) {
      bottomState.reset()
    }
  }

  AnimatedVisibility(
    visible = toolbarVisible,
    enter = fadeIn(animationSpec = tween(ToolbarVisibilityEnterMillis)),
    exit = fadeOut(animationSpec = tween(ToolbarVisibilityExitMillis)),
    modifier =
      modifier
        .fillMaxWidth()
        .offset { IntOffset(x = 0, y = -bottomInset.roundToPx()) }
        .padding(
          start = ToolbarHorizontalPadding,
          end = ToolbarHorizontalPadding,
          bottom = ToolbarBottomPadding,
        ),
  ) {
    Box(contentAlignment = Alignment.BottomCenter) {
      Column(
        modifier = Modifier.widthIn(max = ResponsiveContainerDefaults.MaxWidth).fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally,
      ) {
        EditorToolbarPages(
          pages = pages,
          editorFocused = editorFocused,
          activeBottomPanel = activeBottomPanel,
          onEditorInputRequest = ::restoreEditorInput,
          onKeyboardDismissRequest = ::dismissEditorInput,
          onBottomPanelToggle = ::toggleBottomPanel,
          modifier = Modifier.fillMaxWidth(),
        )

        if (activeBottomPanel != null) {
          Column {
            Box(Modifier.height(ToolbarBottomPanelGap))
            EditorToolbarBottomPanel(panel = activeBottomPanel, height = bottomPanelHeight)
          }
        }
      }
    }
  }
}
