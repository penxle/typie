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
  val panelTransitionIdle: Boolean = true,
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

internal data class KeyboardSpaceAnchor(val inset: Dp)

internal data class KeyboardRestore(val anchor: KeyboardSpaceAnchor)

internal data class PanelSession(
  val key: EditorToolbarBottomPanelKey,
  val height: Dp,
  val keyboardSpace: KeyboardSpaceAnchor?,
)

@Stable
internal class EditorToolbarInputState {
  var panel by mutableStateOf<PanelSession?>(null)
    private set

  var keyboardRestore by mutableStateOf<KeyboardRestore?>(null)
    private set

  var rememberedKeyboardInset by mutableStateOf(0.dp)
    private set

  private var previousSoftwareKeyboardVisible by mutableStateOf(false)
  private var lastPanel by mutableStateOf<EditorToolbarBottomPanelKey?>(null)
  private var lastPanelHeight by mutableStateOf(ToolbarBottomPanelHeight)
  private var queuedEffects by mutableStateOf(emptyList<ToolbarEffect>())
  var effectVersion by mutableIntStateOf(0)
    private set

  val activeBottomPanel: EditorToolbarBottomPanelKey?
    get() = panel?.key

  val lastBottomPanel: EditorToolbarBottomPanelKey?
    get() = lastPanel

  val lastBottomPanelHeight: Dp
    get() = lastPanelHeight

  fun retainedKeyboardInset(): Dp =
    maxOf(
      rememberedKeyboardInset,
      keyboardRestore?.anchor?.inset ?: 0.dp,
      panel?.keyboardSpace?.inset ?: 0.dp,
    )

  fun onEnvironmentChanged(environment: ToolbarInputEnvironment) {
    if (!environment.visible) {
      reset()
      previousSoftwareKeyboardVisible = false
      return
    }

    if (environment.keyboardType == EditorKeyboardType.Hardware) {
      keyboardRestore = null
      rememberedKeyboardInset = 0.dp
    }

    val effectiveImeInset = effectiveImeInset(environment)
    val softwareKeyboardVisible =
      isSoftwareKeyboardVisible(
        imeBottom = effectiveImeInset,
        safeBottomInset = environment.safeBottomInset,
      )
    val editorInputActive =
      environment.focused || panel != null || retainedKeyboardInset() > environment.safeBottomInset

    if (!editorInputActive) {
      keyboardRestore = null
      rememberedKeyboardInset = 0.dp
      previousSoftwareKeyboardVisible = softwareKeyboardVisible
      return
    }

    val currentPanel = panel
    if (currentPanel != null && !previousSoftwareKeyboardVisible && softwareKeyboardVisible) {
      lastPanel = currentPanel.key
      lastPanelHeight = currentPanel.height
      keyboardRestore = null
      rememberedKeyboardInset =
        visibleImeInsetOrZero(effectiveImeInset, environment.safeBottomInset)
      panel = null
      previousSoftwareKeyboardVisible = softwareKeyboardVisible
      return
    }

    if (
      currentPanel?.keyboardSpace != null && environment.keyboardType == EditorKeyboardType.Hardware
    ) {
      panel = currentPanel.copy(height = ToolbarBottomPanelMinHeight, keyboardSpace = null)
      lastPanelHeight = ToolbarBottomPanelMinHeight
    }

    if (keyboardRestore != null) {
      syncKeyboardRestore(environment, effectiveImeInset, softwareKeyboardVisible)
    } else if (softwareKeyboardVisible) {
      rememberKeyboardInset(
        effectiveImeInset = effectiveImeInset,
        safeBottomInset = environment.safeBottomInset,
        preserveCurrentInset = panel != null,
      )
    }

    previousSoftwareKeyboardVisible = softwareKeyboardVisible
  }

  fun dispatch(intent: ToolbarIntent, environment: ToolbarInputEnvironment) {
    when (intent) {
      is ToolbarIntent.OpenPanel -> openPanel(intent.panel, environment)
      ToolbarIntent.RestoreEditorInput -> restoreEditorInput()
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
        restoreEditorInput()
      } else {
        panel = currentPanel.copy(key = panelKey)
        lastPanel = panelKey
      }
      return
    }

    val currentRestore = keyboardRestore
    val effectiveImeInset = effectiveImeInset(environment)
    val softwareKeyboardVisible =
      isSoftwareKeyboardVisible(
        imeBottom = effectiveImeInset,
        safeBottomInset = environment.safeBottomInset,
      )
    val observedInset = visibleImeInsetOrZero(effectiveImeInset, environment.safeBottomInset)
    val keyboardSpace =
      when {
        currentRestore != null -> currentRestore.anchor
        softwareKeyboardVisible -> KeyboardSpaceAnchor(inset = observedInset)
        rememberedKeyboardInset > environment.safeBottomInset + ToolbarBottomPanelGap ->
          KeyboardSpaceAnchor(inset = rememberedKeyboardInset)
        else -> null
      }
    val panelHeight =
      keyboardSpace?.panelHeight(environment.safeBottomInset)
        ?: if (
          environment.keyboardType == EditorKeyboardType.Hardware && !softwareKeyboardVisible
        ) {
          ToolbarBottomPanelMinHeight
        } else {
          ToolbarBottomPanelHeight
        }

    keyboardRestore = null
    rememberedKeyboardInset =
      if (keyboardSpace != null) {
        maxOf(rememberedKeyboardInset, keyboardSpace.inset)
      } else {
        0.dp
      }
    panel = PanelSession(key = panelKey, height = panelHeight, keyboardSpace = keyboardSpace)
    lastPanel = panelKey
    lastPanelHeight = panelHeight

    if (softwareKeyboardVisible || currentRestore != null) {
      emit(ToolbarEffect.HideKeyboard)
    }
  }

  private fun restoreEditorInput() {
    val currentPanel = panel
    if (currentPanel == null) {
      emit(ToolbarEffect.RequestFocus)
      return
    }

    val keyboardSpace = currentPanel.keyboardSpace

    lastPanel = currentPanel.key
    lastPanelHeight = currentPanel.height
    keyboardRestore = keyboardSpace?.let { KeyboardRestore(anchor = it) }
    if (keyboardSpace == null) {
      rememberedKeyboardInset = 0.dp
    }
    panel = null

    emit(ToolbarEffect.RequestFocus)
    if (keyboardSpace != null) {
      emit(ToolbarEffect.ShowKeyboard)
    }
  }

  private fun dismissInput(environment: ToolbarInputEnvironment) {
    if (panel != null) {
      restoreEditorInput()
      return
    }

    val effectiveImeInset = effectiveImeInset(environment)
    val softwareKeyboardVisible =
      isSoftwareKeyboardVisible(
        imeBottom = effectiveImeInset,
        safeBottomInset = environment.safeBottomInset,
      )
    val fixedAction =
      fixedActionFor(
        activePanel = null,
        environment = environment,
        softwareKeyboardVisible = softwareKeyboardVisible,
      )

    keyboardRestore = null
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
    keyboardRestore = null
    rememberedKeyboardInset = 0.dp
    lastPanel = null
    lastPanelHeight = ToolbarBottomPanelHeight
    queuedEffects = emptyList()
  }

  private fun syncKeyboardRestore(
    environment: ToolbarInputEnvironment,
    effectiveImeInset: Dp,
    softwareKeyboardVisible: Boolean,
  ) {
    val restore = keyboardRestore ?: return
    when {
      softwareKeyboardVisible &&
        environment.panelTransitionIdle &&
        effectiveImeInset >= restore.anchor.inset -> {
        keyboardRestore = null
        rememberedKeyboardInset =
          visibleImeInsetOrZero(effectiveImeInset, environment.safeBottomInset)
      }
      !environment.focused && !softwareKeyboardVisible && environment.panelTransitionIdle -> {
        keyboardRestore = null
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
  if (environment.keyboardType == EditorKeyboardType.Software) environment.imeBottom else 0.dp

private fun KeyboardSpaceAnchor.panelHeight(safeBottomInset: Dp): Dp =
  (maxOf(inset, safeBottomInset + ToolbarBottomPanelGap + ToolbarBottomPanelMinHeight) -
      safeBottomInset -
      ToolbarBottomPanelGap)
    .coerceAtLeast(0.dp)

internal fun fixedActionFor(
  activePanel: EditorToolbarBottomPanelKey?,
  environment: ToolbarInputEnvironment,
  softwareKeyboardVisible: Boolean,
): ToolbarFixedAction =
  when {
    activePanel != null -> ToolbarFixedAction.ClosePanel
    environment.keyboardType == EditorKeyboardType.Hardware && !softwareKeyboardVisible ->
      ToolbarFixedAction.HideToolbar
    else -> ToolbarFixedAction.DismissInput
  }

internal fun textInputSessionEnabledForBottomPanel(
  environment: ToolbarInputEnvironment,
  softwareKeyboardVisible: Boolean,
  suppressSoftwareKeyboard: Boolean,
): Boolean =
  suppressSoftwareKeyboard ||
    softwareKeyboardVisible ||
    (environment.keyboardType == EditorKeyboardType.Hardware && !softwareKeyboardVisible)

internal fun suppressSoftwareKeyboard(panel: PanelSession): Boolean = panel.keyboardSpace != null
