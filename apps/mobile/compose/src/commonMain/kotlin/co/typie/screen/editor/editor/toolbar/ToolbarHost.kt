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
import androidx.compose.ui.unit.dp
import co.typie.ext.ime
import co.typie.screen.editor.editor.toolbar.bottom.EditorToolbarBottomPanel
import co.typie.ui.component.ResponsiveContainerDefaults
import kotlinx.coroutines.delay

@OptIn(ExperimentalComposeUiApi::class)
@Composable
internal fun EditorToolbarHost(
  editorFocused: Boolean,
  visible: Boolean,
  safeBottomInset: Dp,
  bottomState: EditorToolbarBottomState,
  keyboardType: EditorKeyboardType,
  hardwareKeyboardConnected: Boolean,
  hardwareKeyboardModeGeneration: Int,
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
    bottomState.softwareKeyboardVisible(
      keyboardType = keyboardType,
      imeBottom = imeBottom,
      safeBottomInset = safeBottomInset,
    )
  var softwareKeyboardRestorePending by remember { mutableStateOf(false) }
  val fixedActionSoftwareKeyboardVisible = softwareKeyboardVisible || softwareKeyboardRestorePending
  val fixedActionKeyboardType =
    resolveEffectiveEditorKeyboardType(
      keyboardType = keyboardType,
      softwareKeyboardVisible = fixedActionSoftwareKeyboardVisible,
    )
  var lastBottomPanel by remember { mutableStateOf(activeBottomPanel) }
  val bottomPanelTransition = remember { MutableTransitionState(activeBottomPanel != null) }
  bottomPanelTransition.targetState = bottomPanelVisible
  val toolbarVisible = bottomState.toolbarVisible(visible = visible, editorFocused = editorFocused)
  val bottomPanelHeight =
    bottomState.bottomPanelHeight(
      imeBottom = imeBottom,
      safeBottomInset = safeBottomInset,
      keyboardType = keyboardType,
    )
  val bottomPanelLayoutHeightTarget =
    resolveEditorToolbarBottomPanelLayoutHeight(
      bottomPanelVisible = bottomPanelVisible,
      bottomPanelHeight = bottomPanelHeight,
    )
  val bottomSpacerHeightTarget =
    resolveEditorToolbarBottomSpacerHeight(
      bottomPanelVisible = bottomPanelVisible,
      bottomPanelLayoutHeight = bottomPanelLayoutHeightTarget,
      inputBottomInset =
        bottomState.inputBottomInset(
          imeBottom = imeBottom,
          safeBottomInset = safeBottomInset,
          keyboardType = keyboardType,
        ),
      safeBottomInset = safeBottomInset,
    )
  val bottomPanelAnimationPrevious = remember { mutableStateOf(bottomPanelVisible) }
  val previousBottomPanelLayoutHeight = remember { mutableStateOf(bottomPanelLayoutHeightTarget) }
  val previousSoftwareKeyboardVisible = remember { mutableStateOf(softwareKeyboardVisible) }
  var rememberedKeyboardInsetRestoreFallbackPending by remember { mutableStateOf(false) }
  val softwareKeyboardAppearing = !previousSoftwareKeyboardVisible.value && softwareKeyboardVisible
  val bottomPanelVisibilityChanged = bottomPanelAnimationPrevious.value != bottomPanelVisible
  val bottomPanelLayoutHeightChangeAnimating =
    shouldAnimateEditorToolbarBottomPanelLayoutHeightChange(
      bottomPanelVisible = bottomPanelVisible,
      softwareKeyboardVisible = softwareKeyboardVisible,
      previousBottomPanelLayoutHeight = previousBottomPanelLayoutHeight.value,
      bottomPanelLayoutHeight = bottomPanelLayoutHeightTarget,
    )
  val bottomPanelAnimationSpec =
    when {
      bottomPanelVisibilityChanged ->
        tween<Dp>(
          if (bottomPanelVisible) {
            ToolbarBottomPanelVisibilityEnterMillis
          } else {
            ToolbarBottomPanelVisibilityExitMillis
          }
        )
      bottomPanelLayoutHeightChangeAnimating -> tween(ToolbarBottomPanelVisibilityEnterMillis)
      else -> snap()
    }
  val bottomSpacerHeightAnimationSpec =
    when {
      softwareKeyboardAppearing -> snap()
      bottomPanelVisibilityChanged ->
        tween<Dp>(
          if (bottomPanelVisible) {
            ToolbarBottomPanelVisibilityEnterMillis
          } else {
            ToolbarBottomPanelVisibilityExitMillis
          }
        )
      bottomPanelLayoutHeightChangeAnimating -> tween(ToolbarBottomPanelVisibilityEnterMillis)
      else -> snap()
    }
  val bottomSpacerHeight by
    animateDpAsState(
      targetValue = bottomSpacerHeightTarget,
      animationSpec = bottomSpacerHeightAnimationSpec,
      label = "EditorToolbarBottomSpacerHeight",
    )
  val bottomPanelLayoutHeight by
    animateDpAsState(
      targetValue = bottomPanelLayoutHeightTarget,
      animationSpec = bottomPanelAnimationSpec,
      label = "EditorToolbarBottomPanelLayoutHeight",
    )
  val bottomInset =
    resolveEditorToolbarBottomInset(
      bottomSpacerHeight = bottomSpacerHeight,
      bottomPanelLayoutHeight = bottomPanelLayoutHeight,
      safeBottomInset = safeBottomInset,
    )
  SideEffect {
    bottomPanelAnimationPrevious.value = bottomPanelVisible
    previousBottomPanelLayoutHeight.value = bottomPanelLayoutHeightTarget
    previousSoftwareKeyboardVisible.value = softwareKeyboardVisible
  }

  fun closeBottomPanelForEditorInputRestore(): Boolean {
    if (bottomState.activePanel == null) {
      return false
    }

    val shouldRestoreSoftwareKeyboard =
      resolveEditorToolbarShouldRestoreSoftwareKeyboard(
        softwareKeyboardRestorePendingForPanel = bottomState.softwareKeyboardRestorePendingForPanel,
        keyboardType =
          resolveEditorToolbarRestoreKeyboardType(
            keyboardType = keyboardType,
            softwareKeyboardRestorePendingForPanel =
              bottomState.softwareKeyboardRestorePendingForPanel,
            hardwareKeyboardConnected = hardwareKeyboardConnected,
          ),
      )
    val rememberedKeyboardInsetClosePolicy =
      resolveEditorToolbarRememberedKeyboardInsetClosePolicy(
        shouldRestoreSoftwareKeyboard = shouldRestoreSoftwareKeyboard,
        softwareKeyboardSuppressedForPanel = bottomState.softwareKeyboardSuppressedForPanel,
        softwareKeyboardRestorePendingForPanel = bottomState.softwareKeyboardRestorePendingForPanel,
      )

    softwareKeyboardRestorePending = shouldRestoreSoftwareKeyboard
    rememberedKeyboardInsetRestoreFallbackPending =
      rememberedKeyboardInsetClosePolicy.restoreFallbackAfterPanelClose &&
        bottomState.rememberedKeyboardInset > 0.dp
    if (rememberedKeyboardInsetClosePolicy.clearBeforePanelClose) {
      bottomState.clearRememberedKeyboardInset()
    }
    bottomState.closePanel(
      keepRememberedKeyboardInsetUntilImeRestored =
        !rememberedKeyboardInsetClosePolicy.clearBeforePanelClose
    )

    return shouldRestoreSoftwareKeyboard
  }

  fun restoreEditorInput() {
    val shouldRestoreSoftwareKeyboard = closeBottomPanelForEditorInputRestore()
    onEditorFocusRequest()
    if (shouldRestoreSoftwareKeyboard) {
      keyboardController?.show()
    }
  }

  fun dismissEditorInput() {
    if (bottomState.activePanel != null) {
      restoreEditorInput()
      return
    }

    val fixedAction =
      resolveEditorToolbarFixedAction(
        activeBottomPanel = null,
        keyboardType = fixedActionKeyboardType,
        softwareKeyboardVisible = fixedActionSoftwareKeyboardVisible,
      )
    softwareKeyboardRestorePending = false
    rememberedKeyboardInsetRestoreFallbackPending = false
    bottomState.clearRememberedKeyboardInset()
    if (fixedAction == EditorToolbarFixedAction.DismissInput) {
      keyboardController?.hide()
    }
    if (editorFocused) {
      focusManager.clearFocus(force = true)
    }
  }

  fun toggleBottomPanel(panel: EditorToolbarBottomPanelKey) {
    if (bottomState.activePanel == panel) {
      restoreEditorInput()
      return
    }

    bottomState.openPanel(
      panel = panel,
      imeBottom = imeBottom,
      safeBottomInset = safeBottomInset,
      keyboardType = keyboardType,
      hardwareKeyboardModeGeneration = hardwareKeyboardModeGeneration,
    )
    if (shouldHideSoftwareKeyboardWhenOpeningBottomPanel(softwareKeyboardVisible)) {
      keyboardController?.hide()
    }
  }

  LaunchedEffect(visible) {
    if (!visible) {
      bottomState.reset()
    }
  }

  LaunchedEffect(activeBottomPanel) {
    if (activeBottomPanel != null) {
      softwareKeyboardRestorePending = false
      rememberedKeyboardInsetRestoreFallbackPending = false
      lastBottomPanel = activeBottomPanel
    }
  }

  LaunchedEffect(softwareKeyboardVisible, editorFocused) {
    if (softwareKeyboardVisible || !editorFocused) {
      softwareKeyboardRestorePending = false
      rememberedKeyboardInsetRestoreFallbackPending = false
    }
  }

  LaunchedEffect(
    rememberedKeyboardInsetRestoreFallbackPending,
    bottomPanelVisible,
    softwareKeyboardVisible,
    bottomState.rememberedKeyboardInset,
  ) {
    if (
      !rememberedKeyboardInsetRestoreFallbackPending ||
        bottomPanelVisible ||
        softwareKeyboardVisible ||
        bottomState.rememberedKeyboardInset <= 0.dp
    ) {
      return@LaunchedEffect
    }

    delay(
      ToolbarBottomPanelVisibilityExitMillis.toLong() +
        ToolbarImplicitSoftwareKeyboardRestoreGraceMillis
    )
    bottomState.clearRememberedKeyboardInset()
    rememberedKeyboardInsetRestoreFallbackPending = false
  }

  LaunchedEffect(
    softwareKeyboardRestorePending,
    bottomPanelVisible,
    softwareKeyboardVisible,
    keyboardType,
    bottomState.rememberedKeyboardInset,
  ) {
    val shouldClearRememberedInset =
      shouldClearRememberedKeyboardInsetAfterHardwareKeyboardRestore(
        softwareKeyboardRestorePending = softwareKeyboardRestorePending,
        bottomPanelVisible = bottomPanelVisible,
        softwareKeyboardVisible = softwareKeyboardVisible,
        keyboardType = keyboardType,
      )
    if (!shouldClearRememberedInset || bottomState.rememberedKeyboardInset <= 0.dp) {
      return@LaunchedEffect
    }

    bottomState.clearRememberedKeyboardInset()
    softwareKeyboardRestorePending = false
  }

  LaunchedEffect(
    keyboardType,
    hardwareKeyboardConnected,
    hardwareKeyboardModeGeneration,
    bottomPanelVisible,
    softwareKeyboardVisible,
  ) {
    val shouldSwitch =
      shouldSwitchOpenEditorToolbarPanelToHardwareKeyboardMode(
        bottomPanelVisible = bottomPanelVisible,
        softwareKeyboardVisible = softwareKeyboardVisible,
        tracksImeInsetForPanel = bottomState.tracksImeInsetForPanel,
        softwareKeyboardRestorePendingForPanel = bottomState.softwareKeyboardRestorePendingForPanel,
        hardwareKeyboardModeGenerationAtOpen =
          bottomState.hardwareKeyboardModeGenerationAtOpenForPanel,
        hardwareKeyboardModeGeneration = hardwareKeyboardModeGeneration,
        hardwareKeyboardModeSwitchPendingForPanel =
          bottomState.hardwareKeyboardModeSwitchPendingForPanel,
        keyboardType = keyboardType,
        hardwareKeyboardConnected = hardwareKeyboardConnected,
      )
    if (shouldSwitch) {
      bottomState.switchOpenPanelToHardwareKeyboardMode()
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
          keyboardType = fixedActionKeyboardType,
          softwareKeyboardVisible = fixedActionSoftwareKeyboardVisible,
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
