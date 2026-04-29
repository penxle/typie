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

  private var panelInputState by mutableStateOf(EditorToolbarPanelInputState())
  private var imeInsetMode by mutableStateOf(EditorToolbarImeInsetMode.Track)
  private var keepRememberedKeyboardInsetUntilImeRestored by mutableStateOf(false)

  val softwareKeyboardSuppressedForPanel: Boolean
    get() = panelInputState.softwareKeyboardSuppressed

  val softwareKeyboardRestorePendingForPanel: Boolean
    get() = panelInputState.softwareKeyboardRestorePending

  val keyboardSizedPanel: Boolean
    get() = panelInputState.keyboardSizedPanel

  val tracksImeInsetForPanel: Boolean
    get() = panelInputState.tracksImeInset

  val hardwareKeyboardModeSwitchPendingForPanel: Boolean
    get() = panelInputState.hardwareKeyboardModeSwitchPending

  val hardwareKeyboardModeGenerationAtOpenForPanel: Int
    get() = panelInputState.hardwareKeyboardModeGenerationAtOpen

  fun textInputSessionEnabled(
    keyboardType: EditorKeyboardType,
    softwareKeyboardVisible: Boolean,
    softwareKeyboardAppearing: Boolean = false,
  ): Boolean =
    activePanel == null ||
      panelInputState.softwareKeyboardSuppressed ||
      softwareKeyboardVisible ||
      softwareKeyboardAppearing ||
      (keyboardType == EditorKeyboardType.Hardware && !softwareKeyboardVisible)

  fun visibleImeInset(imeBottom: Dp, safeBottomInset: Dp): Dp =
    visibleImeInset(
      imeBottom = imeBottom,
      safeBottomInset = safeBottomInset,
      keyboardType = EditorKeyboardType.Software,
    )

  fun visibleImeInset(imeBottom: Dp, safeBottomInset: Dp, keyboardType: EditorKeyboardType): Dp =
    if (activePanel == null) {
      maxOf(
        trustedImeInset(
          imeBottom = imeBottom,
          safeBottomInset = safeBottomInset,
          keyboardType = keyboardType,
        ),
        rememberedKeyboardInset,
      )
    } else {
      bottomPanelInset(
        imeBottom = imeBottom,
        safeBottomInset = safeBottomInset,
        keyboardType = keyboardType,
      )
    }

  fun toolbarVisible(visible: Boolean, editorFocused: Boolean): Boolean =
    visible && (editorFocused || activePanel != null || rememberedKeyboardInset > 0.dp)

  fun inputBottomInset(imeBottom: Dp, safeBottomInset: Dp): Dp =
    inputBottomInset(
      imeBottom = imeBottom,
      safeBottomInset = safeBottomInset,
      keyboardType = EditorKeyboardType.Software,
    )

  fun inputBottomInset(imeBottom: Dp, safeBottomInset: Dp, keyboardType: EditorKeyboardType): Dp =
    maxOf(
      trustedImeInset(
        imeBottom = imeBottom,
        safeBottomInset = safeBottomInset,
        keyboardType = keyboardType,
      ),
      rememberedKeyboardInset,
      safeBottomInset,
    )

  fun bottomPanelHeight(imeBottom: Dp, safeBottomInset: Dp): Dp =
    bottomPanelHeight(
      imeBottom = imeBottom,
      safeBottomInset = safeBottomInset,
      keyboardType = EditorKeyboardType.Software,
    )

  fun bottomPanelHeight(imeBottom: Dp, safeBottomInset: Dp, keyboardType: EditorKeyboardType): Dp =
    (bottomPanelInset(
        imeBottom = imeBottom,
        safeBottomInset = safeBottomInset,
        keyboardType = keyboardType,
      ) - safeBottomInset - ToolbarBottomPanelGap)
      .coerceAtLeast(0.dp)

  fun softwareKeyboardVisible(
    keyboardType: EditorKeyboardType,
    imeBottom: Dp,
    safeBottomInset: Dp,
  ): Boolean =
    isSoftwareKeyboardVisible(
      imeBottom =
        trustedImeInset(
          imeBottom = imeBottom,
          safeBottomInset = safeBottomInset,
          keyboardType = keyboardType,
        ),
      safeBottomInset = safeBottomInset,
    )

  fun openPanel(
    panel: EditorToolbarBottomPanelKey,
    imeBottom: Dp,
    safeBottomInset: Dp,
    keyboardType: EditorKeyboardType = EditorKeyboardType.Software,
    hardwareKeyboardModeGeneration: Int = 0,
  ) {
    if (activePanel == null) {
      val trustedImeBottom =
        trustedImeInset(
          imeBottom = imeBottom,
          safeBottomInset = safeBottomInset,
          keyboardType = keyboardType,
        )
      val softwareKeyboardVisible =
        isSoftwareKeyboardVisible(imeBottom = trustedImeBottom, safeBottomInset = safeBottomInset)
      panelInputState =
        resolveEditorToolbarPanelInputState(
          keyboardType = keyboardType,
          softwareKeyboardVisible = softwareKeyboardVisible,
          hardwareKeyboardModeGeneration = hardwareKeyboardModeGeneration,
        )
      rememberKeyboardInset(
        imeBottom = imeBottom,
        safeBottomInset = safeBottomInset,
        keyboardType = keyboardType,
        preserveCurrentInset =
          shouldKeepRememberedKeyboardInsetUntilImeRestored(
            imeBottom = imeBottom,
            safeBottomInset = safeBottomInset,
            keyboardType = keyboardType,
          ),
      )
    }
    activePanel = panel
  }

  fun closePanel(keepRememberedKeyboardInsetUntilImeRestored: Boolean = false) {
    this.keepRememberedKeyboardInsetUntilImeRestored =
      keepRememberedKeyboardInsetUntilImeRestored && rememberedKeyboardInset > 0.dp
    activePanel = null
    panelInputState = EditorToolbarPanelInputState()
  }

  fun clearRememberedKeyboardInset() {
    keepRememberedKeyboardInsetUntilImeRestored = false
    rememberedKeyboardInset = 0.dp
  }

  fun switchOpenPanelToHardwareKeyboardMode() {
    if (activePanel == null) return

    val keepSoftwareKeyboardSuppressed =
      panelInputState.softwareKeyboardSuppressed && !panelInputState.softwareKeyboardRestorePending
    panelInputState =
      EditorToolbarPanelInputState(
        softwareKeyboardSuppressed = keepSoftwareKeyboardSuppressed,
        heightMode = EditorToolbarPanelHeightMode.Minimum,
      )
    imeInsetMode = EditorToolbarImeInsetMode.FilterStaleSoftwareKeyboardInset
    clearRememberedKeyboardInset()
  }

  fun syncOpenPanelWithSoftwareKeyboardAppearance(
    previousSoftwareKeyboardVisible: Boolean,
    softwareKeyboardVisible: Boolean,
    imeBottom: Dp,
    safeBottomInset: Dp,
  ) {
    if (activePanel != null && !previousSoftwareKeyboardVisible && softwareKeyboardVisible) {
      rememberKeyboardInset(
        imeBottom = imeBottom,
        safeBottomInset = safeBottomInset,
        keyboardType = EditorKeyboardType.Software,
      )
      closePanel()
    }
  }

  fun reset() {
    activePanel = null
    rememberedKeyboardInset = 0.dp
    panelInputState = EditorToolbarPanelInputState()
    imeInsetMode = EditorToolbarImeInsetMode.Track
    keepRememberedKeyboardInsetUntilImeRestored = false
  }

  fun syncKeyboardEnvironment(
    keyboardType: EditorKeyboardType,
    imeBottom: Dp,
    safeBottomInset: Dp,
    editorInputActive: Boolean = true,
  ) {
    val rawSoftwareKeyboardVisible =
      isSoftwareKeyboardVisible(imeBottom = imeBottom, safeBottomInset = safeBottomInset)
    val trustedImeBottom =
      trustedImeInset(
        imeBottom = imeBottom,
        safeBottomInset = safeBottomInset,
        keyboardType = keyboardType,
      )
    val softwareKeyboardVisible =
      isSoftwareKeyboardVisible(imeBottom = trustedImeBottom, safeBottomInset = safeBottomInset)
    if (
      imeInsetMode == EditorToolbarImeInsetMode.FilterStaleSoftwareKeyboardInset &&
        (!rawSoftwareKeyboardVisible || keyboardType == EditorKeyboardType.Software)
    ) {
      imeInsetMode = EditorToolbarImeInsetMode.Track
    }
    if (!editorInputActive) {
      if (activePanel == null && rememberedKeyboardInset > 0.dp) {
        clearRememberedKeyboardInset()
      }
      return
    }
    if (imeInsetMode == EditorToolbarImeInsetMode.Track && softwareKeyboardVisible) {
      rememberKeyboardInset(
        imeBottom = imeBottom,
        safeBottomInset = safeBottomInset,
        keyboardType = keyboardType,
        preserveCurrentInset =
          (activePanel != null && panelInputState.softwareKeyboardSuppressed) ||
            shouldKeepRememberedKeyboardInsetUntilImeRestored(
              imeBottom = imeBottom,
              safeBottomInset = safeBottomInset,
              keyboardType = keyboardType,
            ),
      )
    }
  }

  fun clearRememberedKeyboardInsetIfRestored(imeBottom: Dp) {
    if (
      activePanel == null && rememberedKeyboardInset > 0.dp && imeBottom >= rememberedKeyboardInset
    ) {
      keepRememberedKeyboardInsetUntilImeRestored = false
      rememberedKeyboardInset = 0.dp
    }
  }

  private fun bottomPanelInset(
    imeBottom: Dp,
    safeBottomInset: Dp,
    keyboardType: EditorKeyboardType,
  ): Dp =
    if (panelInputState.keyboardSizedPanel) {
      maxOf(
        if (panelInputState.tracksImeInset) {
          rememberedImeInset(
            imeBottom = imeBottom,
            safeBottomInset = safeBottomInset,
            keyboardType = keyboardType,
          )
        } else {
          0.dp
        },
        rememberedKeyboardInset,
        safeBottomInset + ToolbarBottomPanelGap + ToolbarBottomPanelMinHeight,
      )
    } else if (rememberedKeyboardInset > safeBottomInset + ToolbarBottomPanelGap) {
      maxOf(
        rememberedKeyboardInset,
        safeBottomInset + ToolbarBottomPanelGap + ToolbarBottomPanelMinHeight,
      )
    } else {
      safeBottomInset + ToolbarBottomPanelGap + ToolbarBottomPanelHeight
    }

  private fun rememberKeyboardInset(
    imeBottom: Dp,
    safeBottomInset: Dp,
    keyboardType: EditorKeyboardType,
    preserveCurrentInset: Boolean = false,
  ) {
    val nextInset =
      rememberedImeInset(
        imeBottom = imeBottom,
        safeBottomInset = safeBottomInset,
        keyboardType = keyboardType,
      )
    rememberedKeyboardInset =
      if (preserveCurrentInset) {
        maxOf(rememberedKeyboardInset, nextInset)
      } else {
        nextInset
      }
  }

  private fun shouldKeepRememberedKeyboardInsetUntilImeRestored(
    imeBottom: Dp,
    safeBottomInset: Dp,
    keyboardType: EditorKeyboardType,
  ): Boolean =
    keepRememberedKeyboardInsetUntilImeRestored &&
      activePanel == null &&
      rememberedKeyboardInset > 0.dp &&
      rememberedImeInset(
        imeBottom = imeBottom,
        safeBottomInset = safeBottomInset,
        keyboardType = keyboardType,
      ) < rememberedKeyboardInset

  private fun rememberedImeInset(
    imeBottom: Dp,
    safeBottomInset: Dp,
    keyboardType: EditorKeyboardType,
  ): Dp =
    resolveRememberedKeyboardInset(
      imeBottom =
        trustedImeInset(
          imeBottom = imeBottom,
          safeBottomInset = safeBottomInset,
          keyboardType = keyboardType,
        ),
      safeBottomInset = safeBottomInset,
    )

  private fun trustedImeInset(
    imeBottom: Dp,
    safeBottomInset: Dp,
    keyboardType: EditorKeyboardType,
  ): Dp =
    if (
      imeInsetMode == EditorToolbarImeInsetMode.FilterStaleSoftwareKeyboardInset &&
        keyboardType == EditorKeyboardType.Hardware &&
        imeBottom >= safeBottomInset + ToolbarBottomPanelGap + ToolbarBottomPanelMinHeight
    ) {
      0.dp
    } else {
      imeBottom
    }
}

private fun resolveRememberedKeyboardInset(imeBottom: Dp, safeBottomInset: Dp): Dp =
  if (imeBottom > safeBottomInset) imeBottom else 0.dp
