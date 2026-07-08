package co.typie.screen.editor.editor.toolbar

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.screen.editor.editor.state.EditorInputEffect

@Composable
internal fun rememberEditorToolbarInputState(): EditorToolbarInputState = remember {
  EditorToolbarInputState()
}

internal data class ToolbarInputEnvironment(
  val visible: Boolean,
  val focused: Boolean,
  val imeBottom: Dp,
  val safeBottomInset: Dp,
  val keyboardState: EditorKeyboardState,
  val panelTransitionRunning: Boolean = false,
) {
  val keyboardType: EditorKeyboardType
    get() = keyboardState.type
}

internal sealed interface ToolbarIntent {
  data class OpenPanel(val panel: EditorToolbarBottomPanel) : ToolbarIntent

  data object RestoreEditorInput : ToolbarIntent

  data object DismissInput : ToolbarIntent

  data object HideInput : ToolbarIntent

  data object Reset : ToolbarIntent
}

internal enum class ToolbarFixedAction {
  ClosePanel,
  HideToolbar,
  DismissInput,
}

internal fun isEditorToolbarPresented(
  environment: ToolbarInputEnvironment,
  activeBottomPanel: EditorToolbarBottomPanel?,
  restoringEditorInput: Boolean = false,
  retainingToolbarModal: Boolean = false,
): Boolean {
  val editorInputActive =
    environment.visible &&
      (environment.focused ||
        activeBottomPanel != null ||
        restoringEditorInput ||
        retainingToolbarModal)
  if (!editorInputActive) {
    return false
  }

  if (activeBottomPanel != null || restoringEditorInput || retainingToolbarModal) {
    return true
  }

  if (environment.keyboardType == EditorKeyboardType.Hardware) {
    return true
  }

  return when (environment.keyboardState.presentation) {
    EditorKeyboardPresentation.Hidden,
    EditorKeyboardPresentation.Hiding -> false
    EditorKeyboardPresentation.Showing ->
      isImeVisible(
        imeBottom = effectiveImeInset(environment),
        safeBottomInset = environment.safeBottomInset,
      )
    is EditorKeyboardPresentation.Shown -> true
  }
}

internal sealed interface PanelKeyboardSpace {
  val inset: Dp

  data class FollowIme(override val inset: Dp) : PanelKeyboardSpace

  data class Fixed(override val inset: Dp, val restoreKeyboardOnClose: Boolean) : PanelKeyboardSpace
}

internal data class PanelSession(
  val panel: EditorToolbarBottomPanel,
  val height: Dp,
  val keyboardSpace: PanelKeyboardSpace?,
)

internal data class PanelSnapshot(val panel: EditorToolbarBottomPanel, val height: Dp)

private data class ImeObservation(val visible: Boolean = false, val hideEventVersion: Int = 0)

@Stable
internal class EditorToolbarInputState {
  var panel by mutableStateOf<PanelSession?>(null)
    private set

  var keyboardRestoreInset by mutableStateOf<Dp?>(null)
    private set

  var rememberedKeyboardInset by mutableStateOf(0.dp)
    private set

  private var previousIme by mutableStateOf(ImeObservation())
  private var lastPanelSnapshot by mutableStateOf<PanelSnapshot?>(null)

  val activeBottomPanel: EditorToolbarBottomPanel?
    get() = panel?.panel

  val lastBottomPanel: EditorToolbarBottomPanel?
    get() = lastPanelSnapshot?.panel

  val lastBottomPanelHeight: Dp
    get() = lastPanelSnapshot?.height ?: ToolbarBottomPanelHeight

  fun retainedKeyboardInset(): Dp =
    maxOf(
      rememberedKeyboardInset,
      keyboardRestoreInset ?: 0.dp,
      panel?.keyboardSpace?.inset ?: 0.dp,
    )

  fun onEnvironmentChanged(environment: ToolbarInputEnvironment): List<EditorInputEffect> {
    if (!environment.visible) {
      resetInputState()
      previousIme = ImeObservation(hideEventVersion = environment.keyboardState.imeHideEventVersion)
      return emptyList()
    }

    val panelRetainsHiddenImeSpace = panel?.keyboardSpace?.retainsPanelSpaceWhenImeHidden == true
    if (
      !environment.keyboardState.usesImeInset &&
        !panelRetainsHiddenImeSpace &&
        keyboardRestoreInset == null
    ) {
      rememberedKeyboardInset = 0.dp
    }

    val effectiveImeInset = effectiveImeInset(environment)
    val imeVisible =
      isImeVisible(imeBottom = effectiveImeInset, safeBottomInset = environment.safeBottomInset)
    val currentIme =
      ImeObservation(
        visible = imeVisible,
        hideEventVersion = environment.keyboardState.imeHideEventVersion,
      )

    panel
      ?.takeIf { !environment.focused }
      ?.let { currentPanel ->
        lastPanelSnapshot = currentPanel.snapshot()
        panel = null
        keyboardRestoreInset = null
        rememberedKeyboardInset =
          visibleImeInsetOrZero(effectiveImeInset, environment.safeBottomInset)
        previousIme = currentIme
        return emptyList()
      }

    val retainedKeyboardInset = retainedKeyboardInset()
    val editorInputActive =
      environment.focused ||
        panel != null ||
        keyboardRestoreInset != null ||
        (imeVisible && retainedKeyboardInset > environment.safeBottomInset)
    val imeHideEvent = currentIme.hideEventVersion != previousIme.hideEventVersion

    if (!editorInputActive) {
      keyboardRestoreInset = null
      rememberedKeyboardInset = 0.dp
      previousIme = currentIme
      return emptyList()
    }

    var currentPanel = panel
    if (
      imeHideEvent && !previousIme.visible && !imeVisible && !environment.panelTransitionRunning
    ) {
      val nextPanel = currentPanel?.withoutKeyboardRestoreOnClose()
      if (nextPanel != currentPanel) {
        currentPanel = nextPanel
        panel = nextPanel
      }
    }

    if (currentPanel != null && !previousIme.visible && imeVisible) {
      if (currentPanel.keyboardSpace is PanelKeyboardSpace.FollowIme) {
        rememberKeyboardInset(
          effectiveImeInset = effectiveImeInset,
          safeBottomInset = environment.safeBottomInset,
          preserveCurrentInset = true,
        )
        previousIme = currentIme
        return listOf(EditorInputEffect.HideKeyboard)
      } else {
        lastPanelSnapshot = currentPanel.snapshot()
        keyboardRestoreInset = null
        rememberedKeyboardInset =
          visibleImeInsetOrZero(effectiveImeInset, environment.safeBottomInset)
        panel = null
      }
      previousIme = currentIme
      return emptyList()
    }

    if (
      currentPanel?.keyboardSpace != null &&
        environment.keyboardType == EditorKeyboardType.Hardware &&
        !environment.keyboardState.usesImeInset &&
        !currentPanel.keyboardSpace.retainsPanelSpaceWhenImeHidden
    ) {
      panel = currentPanel.copy(height = ToolbarBottomPanelMinHeight, keyboardSpace = null)
      lastPanelSnapshot = PanelSnapshot(currentPanel.panel, ToolbarBottomPanelMinHeight)
    }

    if (keyboardRestoreInset != null) {
      syncKeyboardRestore(environment, effectiveImeInset, imeVisible)
    } else if (imeVisible) {
      rememberKeyboardInset(
        effectiveImeInset = effectiveImeInset,
        safeBottomInset = environment.safeBottomInset,
        preserveCurrentInset = panel != null,
      )
    }

    previousIme = currentIme
    return emptyList()
  }

  fun dispatch(
    intent: ToolbarIntent,
    environment: ToolbarInputEnvironment,
  ): List<EditorInputEffect> =
    when (intent) {
      is ToolbarIntent.OpenPanel -> openPanel(intent.panel, environment)
      ToolbarIntent.RestoreEditorInput -> restoreEditorInput(environment)
      ToolbarIntent.DismissInput -> dismissInput(environment)
      ToolbarIntent.HideInput -> hideInput()
      ToolbarIntent.Reset -> reset()
    }

  private fun openPanel(
    nextPanel: EditorToolbarBottomPanel,
    environment: ToolbarInputEnvironment,
  ): List<EditorInputEffect> {
    val currentPanel = panel
    if (currentPanel != null) {
      if (currentPanel.panel == nextPanel) {
        return restoreEditorInput(environment)
      } else {
        panel = currentPanel.copy(panel = nextPanel)
        lastPanelSnapshot = PanelSnapshot(nextPanel, currentPanel.height)
      }
      return emptyList()
    }

    val currentRestoreInset = keyboardRestoreInset
    val effectiveImeInset = effectiveImeInset(environment)
    val imeVisible =
      isImeVisible(imeBottom = effectiveImeInset, safeBottomInset = environment.safeBottomInset)
    val observedInset = visibleImeInsetOrZero(effectiveImeInset, environment.safeBottomInset)
    val keyboardSpaceInset = maxOf(observedInset, rememberedKeyboardInset)
    val keyboardSpace =
      when {
        currentRestoreInset != null -> panelKeyboardSpace(currentRestoreInset, environment)
        imeVisible -> panelKeyboardSpace(keyboardSpaceInset, environment)
        rememberedKeyboardInset > environment.safeBottomInset + ToolbarBottomPanelGap ->
          PanelKeyboardSpace.FollowIme(inset = rememberedKeyboardInset)
        else -> null
      }
    val panelHeight =
      keyboardSpace?.panelHeight(environment.safeBottomInset)
        ?: if (environment.keyboardType == EditorKeyboardType.Hardware && !imeVisible) {
          ToolbarBottomPanelMinHeight
        } else {
          ToolbarBottomPanelHeight
        }

    keyboardRestoreInset = null
    rememberedKeyboardInset =
      if (keyboardSpace != null) {
        maxOf(rememberedKeyboardInset, keyboardSpace.inset)
      } else {
        0.dp
      }
    panel = PanelSession(panel = nextPanel, height = panelHeight, keyboardSpace = keyboardSpace)
    lastPanelSnapshot = PanelSnapshot(nextPanel, panelHeight)

    return if (imeVisible || currentRestoreInset != null) {
      listOf(EditorInputEffect.HideKeyboard)
    } else {
      emptyList()
    }
  }

  private fun restoreEditorInput(environment: ToolbarInputEnvironment): List<EditorInputEffect> {
    val currentPanel = panel
    if (currentPanel == null) {
      return listOf(EditorInputEffect.RequestFocus)
    }

    val keyboardSpace = currentPanel.keyboardSpace
    val restoreKeyboard =
      keyboardSpace != null &&
        (environment.keyboardState.usesImeInset || keyboardSpace.restoresKeyboardOnClose)

    lastPanelSnapshot = currentPanel.snapshot()
    keyboardRestoreInset = keyboardSpace?.inset?.takeIf { restoreKeyboard }
    if (!restoreKeyboard) {
      rememberedKeyboardInset = 0.dp
    }
    panel = null

    return if (restoreKeyboard) {
      listOf(EditorInputEffect.RequestFocus, EditorInputEffect.ShowKeyboard)
    } else {
      listOf(EditorInputEffect.RequestFocus)
    }
  }

  private fun dismissInput(environment: ToolbarInputEnvironment): List<EditorInputEffect> {
    if (panel != null) {
      return restoreEditorInput(environment)
    }

    val effectiveImeInset = effectiveImeInset(environment)
    val imeVisible =
      isImeVisible(imeBottom = effectiveImeInset, safeBottomInset = environment.safeBottomInset)
    val fixedAction =
      fixedActionFor(activePanel = null, environment = environment, imeVisible = imeVisible)

    keyboardRestoreInset = null
    rememberedKeyboardInset = 0.dp
    val effects = mutableListOf<EditorInputEffect>()
    if (fixedAction == ToolbarFixedAction.DismissInput) {
      effects += EditorInputEffect.HideKeyboard
    }
    if (environment.focused) {
      effects += EditorInputEffect.ClearFocus
    }
    return effects
  }

  private fun hideInput(): List<EditorInputEffect> {
    resetInputState()
    lastPanelSnapshot = null
    return listOf(EditorInputEffect.HideKeyboard)
  }

  private fun resetInputState() {
    panel = null
    keyboardRestoreInset = null
    rememberedKeyboardInset = 0.dp
    lastPanelSnapshot = null
    previousIme = ImeObservation()
  }

  private fun reset(): List<EditorInputEffect> {
    resetInputState()
    return emptyList()
  }

  private fun syncKeyboardRestore(
    environment: ToolbarInputEnvironment,
    effectiveImeInset: Dp,
    imeVisible: Boolean,
  ) {
    val restoreInset = keyboardRestoreInset ?: return
    when {
      imeVisible &&
        !environment.panelTransitionRunning &&
        effectiveImeInset == (environment.keyboardState.settledImeBottom ?: restoreInset) -> {
        keyboardRestoreInset = null
        rememberedKeyboardInset =
          visibleImeInsetOrZero(effectiveImeInset, environment.safeBottomInset)
      }
      !environment.focused && !imeVisible && !environment.panelTransitionRunning -> {
        keyboardRestoreInset = null
        rememberedKeyboardInset = 0.dp
      }
    }
  }

  private fun rememberKeyboardInset(
    effectiveImeInset: Dp,
    safeBottomInset: Dp,
    preserveCurrentInset: Boolean,
  ) {
    val nextInset = visibleImeInsetOrZero(effectiveImeInset, safeBottomInset)
    rememberedKeyboardInset =
      if (preserveCurrentInset) {
        maxOf(rememberedKeyboardInset, nextInset)
      } else {
        nextInset
      }
  }
}

internal fun visibleImeInsetOrZero(effectiveImeInset: Dp, safeBottomInset: Dp): Dp =
  if (effectiveImeInset > safeBottomInset) effectiveImeInset else 0.dp

internal fun effectiveImeInset(environment: ToolbarInputEnvironment): Dp =
  trustedImeBottomInset(
    rawImeBottom = environment.imeBottom,
    keyboardState = environment.keyboardState,
  )

private fun PanelKeyboardSpace.panelHeight(safeBottomInset: Dp): Dp =
  (maxOf(inset, safeBottomInset + ToolbarBottomPanelGap + ToolbarBottomPanelMinHeight) -
      safeBottomInset -
      ToolbarBottomPanelGap)
    .coerceAtLeast(0.dp)

private fun panelKeyboardSpace(
  inset: Dp,
  environment: ToolbarInputEnvironment,
): PanelKeyboardSpace =
  if (environment.keyboardType == EditorKeyboardType.Hardware) {
    PanelKeyboardSpace.Fixed(inset = inset, restoreKeyboardOnClose = true)
  } else {
    PanelKeyboardSpace.FollowIme(inset = inset)
  }

private fun PanelSession.withoutKeyboardRestoreOnClose(): PanelSession =
  when (val space = keyboardSpace) {
    is PanelKeyboardSpace.Fixed -> copy(keyboardSpace = space.copy(restoreKeyboardOnClose = false))
    is PanelKeyboardSpace.FollowIme,
    null -> this
  }

private fun PanelSession.snapshot(): PanelSnapshot = PanelSnapshot(panel, height)

private val PanelKeyboardSpace.retainsPanelSpaceWhenImeHidden: Boolean
  get() =
    when (this) {
      is PanelKeyboardSpace.Fixed -> true
      is PanelKeyboardSpace.FollowIme -> false
    }

private val PanelKeyboardSpace.restoresKeyboardOnClose: Boolean
  get() =
    when (this) {
      is PanelKeyboardSpace.Fixed -> restoreKeyboardOnClose
      is PanelKeyboardSpace.FollowIme -> false
    }

internal fun fixedActionFor(
  activePanel: EditorToolbarBottomPanel?,
  environment: ToolbarInputEnvironment,
  imeVisible: Boolean,
): ToolbarFixedAction =
  when {
    activePanel != null -> ToolbarFixedAction.ClosePanel
    environment.keyboardType == EditorKeyboardType.Hardware && !imeVisible ->
      ToolbarFixedAction.HideToolbar
    else -> ToolbarFixedAction.DismissInput
  }

internal fun textInputSessionEnabledForBottomPanel(
  environment: ToolbarInputEnvironment,
  imeVisible: Boolean,
  suppressSoftwareKeyboard: Boolean,
): Boolean =
  suppressSoftwareKeyboard || imeVisible || environment.keyboardType == EditorKeyboardType.Hardware

internal fun suppressSoftwareKeyboard(panel: PanelSession): Boolean = panel.keyboardSpace != null
