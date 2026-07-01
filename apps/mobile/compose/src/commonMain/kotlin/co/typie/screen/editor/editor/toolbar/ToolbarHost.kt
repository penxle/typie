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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.editor.EditorState
import co.typie.editor.ffi.Message
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.scroll.awaitWithBringIntoView
import co.typie.graphql.fragment.EditorSettingsFontFamily_family
import co.typie.screen.editor.editor.state.EditorInputEffect
import co.typie.screen.editor.editor.toolbar.contextual.TextOptionMode
import co.typie.screen.editor.editor.toolbar.contextual.TextOptionsToolbar
import co.typie.screen.editor.editor.toolbar.contextual.rememberTextToolbarPage
import co.typie.ui.component.ResponsiveContainerDefaults
import kotlinx.coroutines.launch

@Composable
internal fun EditorToolbarHost(
  editorState: EditorState,
  pagerState: ToolbarPagerState,
  bottomPanelTransition: MutableTransitionState<Boolean>,
  editorFocused: Boolean,
  inputState: EditorToolbarInputState,
  environment: ToolbarInputEnvironment,
  fontFamilies: List<EditorSettingsFontFamily_family>,
  sessionState: EditorToolbarSessionState,
  commentEnabled: Boolean,
  onCommentRequest: () -> Unit,
  onInputEffects: (List<EditorInputEffect>) -> Unit,
  onToolAction: (EditorToolbarToolAction) -> Unit,
  modifier: Modifier = Modifier,
) {
  val commandScope = rememberCoroutineScope()
  val runtime = LocalEditorRuntime.current
  val bringIntoViewRequests = LocalEditorBringIntoViewRequests.current
  val latestEnvironment = rememberUpdatedState(environment)

  fun runToolbarModal(block: suspend () -> Unit) {
    sessionState.modalActive = true
    commandScope.launch {
      try {
        block()
      } finally {
        val environment = latestEnvironment.value
        if (environment.visible) {
          onInputEffects(inputState.dispatch(ToolbarIntent.RestoreEditorInput, environment))
        }
        sessionState.modalActive = false
      }
    }
  }

  val onTextOptionModeChange: (TextOptionMode?) -> Unit =
    remember(sessionState) { { mode -> sessionState.activeTextOptionMode = mode } }
  val latestRunToolbarModal = rememberUpdatedState<(suspend () -> Unit) -> Unit>(::runToolbarModal)
  val runToolbarModalAction: (suspend () -> Unit) -> Unit = remember {
    { block -> latestRunToolbarModal.value(block) }
  }

  val toolbarContext = remember(editorState.version) { resolveEditorToolbarContext(editorState) }
  val activeTextOptionMode = sessionState.activeTextOptionMode
  var displayedTextOptionMode by remember { mutableStateOf(activeTextOptionMode) }
  val textToolbarPage =
    rememberTextToolbarPage(
      modifierState = editorState.modifierState,
      selection = editorState.selection,
      fontFamilies = fontFamilies,
      activeTextOptionMode = activeTextOptionMode,
      onTextOptionModeChange = onTextOptionModeChange,
      runToolbarModal = runToolbarModalAction,
      commentEnabled = commentEnabled,
      onCommentRequest = onCommentRequest,
    )
  val pages =
    rememberEditorToolbarPages(toolbarContext = toolbarContext, textToolbarPage = textToolbarPage)
  val panel = inputState.panel
  val activeBottomPanel = panel?.key
  val effectiveImeInset = effectiveImeInset(environment)
  val imeVisible =
    isImeVisible(imeBottom = effectiveImeInset, safeBottomInset = environment.safeBottomInset)
  val retainedKeyboardInset = inputState.retainedKeyboardInset()
  val restoringKeyboard = inputState.keyboardRestoreInset != null
  val inputBottomInset =
    inputState.keyboardRestoreInset?.let { maxOf(it, environment.safeBottomInset) }
      ?: maxOf(effectiveImeInset, retainedKeyboardInset, environment.safeBottomInset)
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
      bottomPanelContainerHeight + environment.safeBottomInset
    } else {
      inputBottomInset
    }
  val toolbarPresented =
    isEditorToolbarPresented(
      environment = environment,
      activeBottomPanel = activeBottomPanel,
      restoringEditorInput = restoringKeyboard,
      retainingToolbarModal = sessionState.modalActive,
    )
  val textPageActive =
    toolbarPresented &&
      pagerState.settledPageKey == EditorToolbarPageKey.Text &&
      EditorToolbarPageKey.Text in toolbarContext.pageKeys
  val textOptionsToolbarVisible = textPageActive && activeTextOptionMode != null
  val fixedAction =
    fixedActionFor(
      activePanel = activeBottomPanel,
      environment = environment,
      imeVisible = imeVisible,
    )
  val animatePanelHeight =
    if (panel != null) {
      panel.keyboardSpace == null
    } else {
      !restoringKeyboard
    }

  LaunchedEffect(environment) { onInputEffects(inputState.onEnvironmentChanged(environment)) }
  LaunchedEffect(toolbarPresented, textPageActive) {
    if ((!toolbarPresented || !textPageActive) && activeTextOptionMode != null) {
      sessionState.activeTextOptionMode = null
    }
  }
  LaunchedEffect(activeTextOptionMode) {
    if (activeTextOptionMode != null) {
      displayedTextOptionMode = activeTextOptionMode
    }
  }

  val previousImeVisible = remember { mutableStateOf(imeVisible) }
  val imeAppearing = !previousImeVisible.value && imeVisible
  val panelTransitionRunning =
    bottomPanelTransition.currentState != bottomPanelTransition.targetState
  val inputSpaceOwnsSpacer = activeBottomPanel == null && (imeVisible || !panelTransitionRunning)
  val panelAnimationSpec =
    when {
      !animatePanelHeight -> snap()
      panelTransitionRunning ->
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
      imeAppearing -> snap()
      inputSpaceOwnsSpacer -> snap()
      !animatePanelHeight -> snap()
      panelTransitionRunning ->
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
    (maxOf(bottomSpacerHeight, bottomPanelLayoutHeight + environment.safeBottomInset) -
        bottomPanelLayoutHeight)
      .coerceAtLeast(0.dp)

  SideEffect { previousImeVisible.value = imeVisible }

  fun sendEditorMessages(messages: List<Message>) {
    if (messages.isEmpty()) {
      return
    }
    val editor = runtime.editor ?: return
    val bringIntoViewTarget = EditorBringIntoViewTarget.CurrentSelectionHead

    commandScope.launch {
      editor.awaitWithBringIntoView(bringIntoViewRequests) {
        messages.forEach { enqueue(it) }
        beforeCommit { bringIntoView(bringIntoViewTarget) }
      }
    }
  }

  AnimatedVisibility(
    visible = toolbarPresented,
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
          commandScope = commandScope,
          pagerState = pagerState,
          autoTargetPageKey = toolbarContext.autoTargetPageKey,
          autoTargetKey = toolbarContext.autoTargetKey,
          editorFocused = editorFocused,
          activeBottomPanel = activeBottomPanel,
          fixedAction = fixedAction,
          onEditorInputRequest = {
            onInputEffects(inputState.dispatch(ToolbarIntent.RestoreEditorInput, environment))
          },
          onKeyboardDismissRequest = {
            sessionState.activeTextOptionMode = null
            onInputEffects(inputState.dispatch(ToolbarIntent.DismissInput, environment))
          },
          onBottomPanelToggle = { panel ->
            onInputEffects(inputState.dispatch(ToolbarIntent.OpenPanel(panel), environment))
          },
          onEditorMessage = { message -> sendEditorMessages(listOf(message)) },
          onToolAction = onToolAction,
          onCurrentPageKeyChange = { pageKey ->
            if (pageKey != EditorToolbarPageKey.Text && sessionState.activeTextOptionMode != null) {
              sessionState.activeTextOptionMode = null
            }
          },
          secondaryToolbarVisible = textOptionsToolbarVisible,
          onSecondaryToolbarInLayoutChange = { sessionState.secondaryToolbarInLayout = it },
          secondaryToolbar = {
            (activeTextOptionMode ?: displayedTextOptionMode)?.let { mode ->
              TextOptionsToolbar(
                mode = mode,
                editorState = editorState,
                fontFamilies = fontFamilies,
                onModeChange = onTextOptionModeChange,
                sendMessages = ::sendEditorMessages,
                runToolbarModal = runToolbarModalAction,
                modifier = Modifier.fillMaxWidth(),
              )
            }
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
                BottomToolbar(
                  panel = visiblePanel,
                  height = bottomPanelHeight,
                  onEditorInputRequest = {
                    onInputEffects(
                      inputState.dispatch(ToolbarIntent.RestoreEditorInput, environment)
                    )
                  },
                  onToolAction = onToolAction,
                )
              }
            }
          }
        }
      }
    }
  }
}
