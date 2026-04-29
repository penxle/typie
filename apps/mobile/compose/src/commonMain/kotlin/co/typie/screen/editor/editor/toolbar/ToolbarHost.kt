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
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.screen.editor.editor.toolbar.bottom.EditorToolbarBottomPanel
import co.typie.ui.component.ResponsiveContainerDefaults

@OptIn(ExperimentalComposeUiApi::class)
@Composable
internal fun EditorToolbarHost(
  editorFocused: Boolean,
  inputState: EditorToolbarInputState,
  environment: ToolbarInputEnvironment,
  onEditorFocusRequest: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val focusManager = LocalFocusManager.current
  val keyboardController = LocalSoftwareKeyboardController.current
  val pages = rememberEditorToolbarPages()
  val panel = inputState.panel
  val activeBottomPanel = panel?.key
  val bottomPanelTransition = remember { MutableTransitionState(activeBottomPanel != null) }
  bottomPanelTransition.targetState = activeBottomPanel != null
  val panelTransitionIdle = bottomPanelTransition.currentState == bottomPanelTransition.targetState
  val hostEnvironment = environment.copy(panelTransitionIdle = panelTransitionIdle)
  val effectiveImeInset = effectiveImeInset(hostEnvironment)
  val softwareKeyboardVisible =
    isSoftwareKeyboardVisible(
      imeBottom = effectiveImeInset,
      safeBottomInset = hostEnvironment.safeBottomInset,
    )
  val retainedKeyboardInset = inputState.retainedKeyboardInset()
  val inputBottomInset =
    maxOf(effectiveImeInset, retainedKeyboardInset, hostEnvironment.safeBottomInset)
  val restoringKeyboard = inputState.keyboardRestore != null
  val bottomPanelHeight = panel?.height ?: inputState.lastBottomPanelHeight
  val bottomPanelContainerHeight = panel?.let { ToolbarBottomPanelGap + it.height } ?: 0.dp
  val lastBottomPanelContainerHeight = ToolbarBottomPanelGap + inputState.lastBottomPanelHeight
  val bottomPanelAnimationTargetHeight =
    if (panel != null) {
      bottomPanelContainerHeight
    } else if (restoringKeyboard) {
      lastBottomPanelContainerHeight
    } else {
      0.dp
    }
  val bottomSpacerTargetHeight =
    if (panel != null) {
      bottomPanelContainerHeight + hostEnvironment.safeBottomInset
    } else {
      inputBottomInset
    }
  val toolbarVisible =
    hostEnvironment.visible &&
      (hostEnvironment.focused ||
        activeBottomPanel != null ||
        retainedKeyboardInset > hostEnvironment.safeBottomInset)
  val fixedAction =
    fixedActionFor(
      activePanel = activeBottomPanel,
      environment = hostEnvironment,
      softwareKeyboardVisible = softwareKeyboardVisible,
    )
  val animatePanelHeight =
    if (panel != null) {
      panel.keyboardSpace == null
    } else {
      !restoringKeyboard
    }

  LaunchedEffect(hostEnvironment) { inputState.onEnvironmentChanged(hostEnvironment) }

  LaunchedEffect(inputState.effectVersion) {
    inputState.takeEffects().forEach { effect ->
      when (effect) {
        ToolbarEffect.ShowKeyboard -> keyboardController?.show()
        ToolbarEffect.HideKeyboard -> keyboardController?.hide()
        ToolbarEffect.RequestFocus -> onEditorFocusRequest()
        ToolbarEffect.ClearFocus -> focusManager.clearFocus(force = true)
      }
    }
  }

  val previousSoftwareKeyboardVisible = remember { mutableStateOf(softwareKeyboardVisible) }
  val softwareKeyboardAppearing = !previousSoftwareKeyboardVisible.value && softwareKeyboardVisible
  val panelVisibilityChanged =
    bottomPanelTransition.currentState != bottomPanelTransition.targetState
  val panelAnimationSpec =
    when {
      !animatePanelHeight -> snap()
      panelVisibilityChanged ->
        tween<Dp>(
          if (bottomPanelTransition.targetState) {
            ToolbarBottomPanelVisibilityEnterMillis
          } else {
            ToolbarBottomPanelVisibilityExitMillis
          }
        )
      else -> tween(ToolbarBottomPanelVisibilityEnterMillis)
    }
  val spacerAnimationSpec =
    when {
      softwareKeyboardAppearing -> snap()
      !animatePanelHeight -> snap()
      panelVisibilityChanged ->
        tween<Dp>(
          if (bottomPanelTransition.targetState) {
            ToolbarBottomPanelVisibilityEnterMillis
          } else {
            ToolbarBottomPanelVisibilityExitMillis
          }
        )
      else -> tween(ToolbarBottomPanelVisibilityEnterMillis)
    }
  val bottomSpacerHeight by
    animateDpAsState(
      targetValue = bottomSpacerTargetHeight,
      animationSpec = spacerAnimationSpec,
      label = "EditorToolbarBottomSpacerHeight",
    )
  val bottomPanelLayoutHeight by
    animateDpAsState(
      targetValue = bottomPanelAnimationTargetHeight,
      animationSpec = panelAnimationSpec,
      label = "EditorToolbarBottomPanelLayoutHeight",
    )
  val bottomInset =
    (maxOf(bottomSpacerHeight, bottomPanelLayoutHeight + hostEnvironment.safeBottomInset) -
        bottomPanelLayoutHeight)
      .coerceAtLeast(0.dp)

  SideEffect { previousSoftwareKeyboardVisible.value = softwareKeyboardVisible }

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
          fixedAction = fixedAction,
          onEditorInputRequest = {
            inputState.dispatch(ToolbarIntent.RestoreEditorInput, hostEnvironment)
          },
          onKeyboardDismissRequest = {
            inputState.dispatch(ToolbarIntent.DismissInput, hostEnvironment)
          },
          onBottomPanelToggle = { panel ->
            inputState.dispatch(ToolbarIntent.OpenPanel(panel), hostEnvironment)
          },
          modifier = Modifier.fillMaxWidth(),
        )

        Box(Modifier.fillMaxWidth().height(bottomPanelLayoutHeight).clipToBounds()) {
          androidx.compose.animation.AnimatedVisibility(
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
            modifier = Modifier.fillMaxWidth(),
          ) {
            val visiblePanel = activeBottomPanel ?: inputState.lastBottomPanel
            if (visiblePanel != null) {
              Column {
                Box(Modifier.height(ToolbarBottomPanelGap))
                EditorToolbarBottomPanel(panel = visiblePanel, height = bottomPanelHeight)
              }
            }
          }
        }
      }
    }
  }
}
