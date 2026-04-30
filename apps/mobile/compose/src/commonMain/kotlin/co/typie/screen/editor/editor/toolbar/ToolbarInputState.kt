package co.typie.screen.editor.editor.toolbar

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

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
  data class OpenPanel(val panel: EditorToolbarBottomPanelKey) : ToolbarIntent

  data object RestoreEditorInput : ToolbarIntent

  data object DismissInput : ToolbarIntent

  data object Reset : ToolbarIntent
}

internal enum class ToolbarEffect {
  ShowKeyboard,
  HideKeyboard,
  RequestFocus,
  ClearFocus,
}

internal enum class ToolbarFixedAction {
  ClosePanel,
  HideToolbar,
  DismissInput,
}

internal fun isEditorToolbarVisible(
  environment: ToolbarInputEnvironment,
  activeBottomPanel: EditorToolbarBottomPanelKey?,
  retainedKeyboardInset: Dp,
): Boolean =
  environment.visible &&
    (environment.focused ||
      activeBottomPanel != null ||
      retainedKeyboardInset > environment.safeBottomInset)

internal sealed interface PanelKeyboardSpace {
  val inset: Dp

  data class FollowIme(override val inset: Dp) : PanelKeyboardSpace

  data class Fixed(override val inset: Dp, val restoreKeyboardOnClose: Boolean) : PanelKeyboardSpace
}

internal data class PanelSession(
  val key: EditorToolbarBottomPanelKey,
  val height: Dp,
  val keyboardSpace: PanelKeyboardSpace?,
)

internal data class PanelSnapshot(val key: EditorToolbarBottomPanelKey, val height: Dp)

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
  private var queuedEffects by mutableStateOf(emptyList<ToolbarEffect>())
  var effectVersion by mutableIntStateOf(0)
    private set

  val activeBottomPanel: EditorToolbarBottomPanelKey?
    get() = panel?.key

  val lastBottomPanel: EditorToolbarBottomPanelKey?
    get() = lastPanelSnapshot?.key

  val lastBottomPanelHeight: Dp
    get() = lastPanelSnapshot?.height ?: ToolbarBottomPanelHeight

  fun retainedKeyboardInset(): Dp =
    maxOf(
      rememberedKeyboardInset,
      keyboardRestoreInset ?: 0.dp,
      panel?.keyboardSpace?.inset ?: 0.dp,
    )

  fun onEnvironmentChanged(environment: ToolbarInputEnvironment) {
    if (!environment.visible) {
      reset()
      previousIme = ImeObservation(hideEventVersion = environment.keyboardState.imeHideEventVersion)
      return
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
    val editorInputActive =
      environment.focused || panel != null || retainedKeyboardInset() > environment.safeBottomInset
    val imeHideEvent = currentIme.hideEventVersion != previousIme.hideEventVersion

    if (!editorInputActive) {
      keyboardRestoreInset = null
      rememberedKeyboardInset = 0.dp
      previousIme = currentIme
      return
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
      lastPanelSnapshot = currentPanel.snapshot()
      keyboardRestoreInset = null
      rememberedKeyboardInset =
        visibleImeInsetOrZero(effectiveImeInset, environment.safeBottomInset)
      panel = null
      previousIme = currentIme
      return
    }

    if (
      currentPanel?.keyboardSpace != null &&
        environment.keyboardType == EditorKeyboardType.Hardware &&
        !environment.keyboardState.usesImeInset &&
        !currentPanel.keyboardSpace.retainsPanelSpaceWhenImeHidden
    ) {
      panel = currentPanel.copy(height = ToolbarBottomPanelMinHeight, keyboardSpace = null)
      lastPanelSnapshot = PanelSnapshot(currentPanel.key, ToolbarBottomPanelMinHeight)
    }

    if (keyboardRestoreInset != null) {
      syncKeyboardRestore(environment, effectiveImeInset, imeVisible)
    } else if (imeVisible) {
      rememberKeyboardInset(
        effectiveImeInset = effectiveImeInset,
        safeBottomInset = environment.safeBottomInset,
        preserveCurrentInset = panel != null || previousIme.visible,
      )
    }

    previousIme = currentIme
  }

  fun dispatch(intent: ToolbarIntent, environment: ToolbarInputEnvironment) {
    when (intent) {
      is ToolbarIntent.OpenPanel -> openPanel(intent.panel, environment)
      ToolbarIntent.RestoreEditorInput -> restoreEditorInput(environment)
      ToolbarIntent.DismissInput -> dismissInput(environment)
      ToolbarIntent.Reset -> reset()
    }
  }

  fun takeEffects(): List<ToolbarEffect> {
    val effects = queuedEffects
    queuedEffects = emptyList()
    return effects
  }

  private fun openPanel(
    panelKey: EditorToolbarBottomPanelKey,
    environment: ToolbarInputEnvironment,
  ) {
    val currentPanel = panel
    if (currentPanel != null) {
      if (currentPanel.key == panelKey) {
        restoreEditorInput(environment)
      } else {
        panel = currentPanel.copy(key = panelKey)
        lastPanelSnapshot = PanelSnapshot(panelKey, currentPanel.height)
      }
      return
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
    panel = PanelSession(key = panelKey, height = panelHeight, keyboardSpace = keyboardSpace)
    lastPanelSnapshot = PanelSnapshot(panelKey, panelHeight)

    if (imeVisible || currentRestoreInset != null) {
      emit(ToolbarEffect.HideKeyboard)
    }
  }

  private fun restoreEditorInput(environment: ToolbarInputEnvironment) {
    val currentPanel = panel
    if (currentPanel == null) {
      emit(ToolbarEffect.RequestFocus)
      return
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

    emit(ToolbarEffect.RequestFocus)
    if (restoreKeyboard) {
      emit(ToolbarEffect.ShowKeyboard)
    }
  }

  private fun dismissInput(environment: ToolbarInputEnvironment) {
    if (panel != null) {
      restoreEditorInput(environment)
      return
    }

    val effectiveImeInset = effectiveImeInset(environment)
    val imeVisible =
      isImeVisible(imeBottom = effectiveImeInset, safeBottomInset = environment.safeBottomInset)
    val fixedAction =
      fixedActionFor(activePanel = null, environment = environment, imeVisible = imeVisible)

    keyboardRestoreInset = null
    rememberedKeyboardInset = 0.dp
    if (fixedAction == ToolbarFixedAction.DismissInput) {
      emit(ToolbarEffect.HideKeyboard)
    }
    if (environment.focused) {
      emit(ToolbarEffect.ClearFocus)
    }
  }

  private fun reset() {
    panel = null
    keyboardRestoreInset = null
    rememberedKeyboardInset = 0.dp
    lastPanelSnapshot = null
    queuedEffects = emptyList()
    previousIme = ImeObservation()
  }

  private fun syncKeyboardRestore(
    environment: ToolbarInputEnvironment,
    effectiveImeInset: Dp,
    imeVisible: Boolean,
  ) {
    val restoreInset = keyboardRestoreInset ?: return
    when {
      imeVisible && !environment.panelTransitionRunning && effectiveImeInset >= restoreInset -> {
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

  private fun emit(effect: ToolbarEffect) {
    queuedEffects = queuedEffects + effect
    effectVersion++
  }
}

internal fun visibleImeInsetOrZero(effectiveImeInset: Dp, safeBottomInset: Dp): Dp =
  if (effectiveImeInset > safeBottomInset) effectiveImeInset else 0.dp

internal fun effectiveImeInset(environment: ToolbarInputEnvironment): Dp =
  if (environment.keyboardState.usesImeInset) environment.imeBottom else 0.dp

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

private fun PanelSession.snapshot(): PanelSnapshot = PanelSnapshot(key, height)

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
  activePanel: EditorToolbarBottomPanelKey?,
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
