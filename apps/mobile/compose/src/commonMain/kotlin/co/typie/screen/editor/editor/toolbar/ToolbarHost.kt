package co.typie.screen.editor.editor.toolbar

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.MutableTransitionState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.snap
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
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
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.TransformOrigin
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
  keyboardType: EditorKeyboardType,
  onEditorFocusRequest: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val imeBottom = WindowInsets.ime.asPaddingValues().calculateBottomPadding()
  val focusManager = LocalFocusManager.current
  val keyboardController = LocalSoftwareKeyboardController.current
  val pages = rememberEditorToolbarPages()
  val activeBottomPanel = bottomState.activePanel
  val bottomPanelVisible = activeBottomPanel != null
  val softwareKeyboardVisible =
    isSoftwareKeyboardVisible(imeBottom = imeBottom, safeBottomInset = safeBottomInset)
  var lastBottomPanel by remember { mutableStateOf(activeBottomPanel) }
  val bottomPanelTransition = remember { MutableTransitionState(activeBottomPanel != null) }
  bottomPanelTransition.targetState = bottomPanelVisible
  val toolbarVisible = bottomState.toolbarVisible(visible = visible, editorFocused = editorFocused)
  val bottomPanelHeight = bottomState.bottomPanelHeight(safeBottomInset = safeBottomInset)
  val bottomPanelAnimationPrevious = remember { mutableStateOf(bottomPanelVisible) }
  val bottomPanelAnimationSpec =
    if (bottomPanelAnimationPrevious.value != bottomPanelVisible) {
      tween<Dp>(
        if (bottomPanelVisible) {
          ToolbarBottomPanelVisibilityEnterMillis
        } else {
          ToolbarBottomPanelVisibilityExitMillis
        }
      )
    } else {
      snap()
    }
  val bottomInsetTarget =
    if (bottomPanelVisible) {
      safeBottomInset
    } else {
      bottomState.inputBottomInset(imeBottom = imeBottom, safeBottomInset = safeBottomInset)
    }
  val bottomInset by
    animateDpAsState(
      targetValue = bottomInsetTarget,
      animationSpec = bottomPanelAnimationSpec,
      label = "EditorToolbarBottomInset",
    )
  val bottomPanelLayoutHeight by
    animateDpAsState(
      targetValue =
        resolveEditorToolbarBottomPanelLayoutHeight(
          bottomPanelVisible = bottomPanelVisible,
          bottomPanelHeight = bottomPanelHeight,
        ),
      animationSpec = bottomPanelAnimationSpec,
      label = "EditorToolbarBottomPanelLayoutHeight",
    )
  SideEffect { bottomPanelAnimationPrevious.value = bottomPanelVisible }

  fun restoreEditorInput() {
    if (bottomState.activePanel != null) {
      bottomState.closePanel()
    }
    onEditorFocusRequest()
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

    bottomState.openPanel(panel = panel, imeBottom = imeBottom, safeBottomInset = safeBottomInset)
  }

  LaunchedEffect(visible) {
    if (!visible) {
      bottomState.reset()
    }
  }

  LaunchedEffect(activeBottomPanel) {
    if (activeBottomPanel != null) {
      lastBottomPanel = activeBottomPanel
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
          keyboardType = keyboardType,
          softwareKeyboardVisible = softwareKeyboardVisible,
          onEditorInputRequest = ::restoreEditorInput,
          onKeyboardDismissRequest = ::dismissEditorInput,
          onBottomPanelToggle = ::toggleBottomPanel,
          modifier = Modifier.fillMaxWidth(),
        )

        AnimatedVisibility(
          visibleState = bottomPanelTransition,
          enter =
            fadeIn(animationSpec = tween(ToolbarBottomPanelVisibilityEnterMillis)) +
              scaleIn(
                animationSpec = tween(ToolbarBottomPanelVisibilityEnterMillis),
                initialScale = ToolbarBottomPanelHiddenScale,
                transformOrigin = TransformOrigin(0.5f, 0f),
              ),
          exit =
            fadeOut(animationSpec = tween(ToolbarBottomPanelVisibilityExitMillis)) +
              scaleOut(
                animationSpec = tween(ToolbarBottomPanelVisibilityExitMillis),
                targetScale = ToolbarBottomPanelHiddenScale,
                transformOrigin = TransformOrigin(0.5f, 0f),
              ),
          modifier = Modifier.fillMaxWidth().height(bottomPanelLayoutHeight).clipToBounds(),
        ) {
          val panel = activeBottomPanel ?: lastBottomPanel
          if (panel != null) {
            Column {
              Box(Modifier.height(ToolbarBottomPanelGap))
              EditorToolbarBottomPanel(panel = panel, height = bottomPanelHeight)
            }
          }
        }
      }
    }
  }
}
