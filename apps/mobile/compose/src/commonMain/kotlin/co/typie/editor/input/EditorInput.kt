package co.typie.editor.input

import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusEventModifierNode
import androidx.compose.ui.focus.FocusState
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.KeyInputModifierNode
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.key.utf16CodePoint
import androidx.compose.ui.node.ModifierNodeElement
import androidx.compose.ui.platform.PlatformTextInputMethodRequest
import androidx.compose.ui.platform.PlatformTextInputModifierNode
import androidx.compose.ui.platform.PlatformTextInputSessionScope
import androidx.compose.ui.platform.establishTextInputSession
import androidx.compose.ui.text.input.EditCommand
import co.typie.editor.Editor
import co.typie.editor.createBindings
import co.typie.editor.ffi.CompositionOp
import co.typie.editor.ffi.InsertionOp
import co.typie.editor.ffi.Key as FfiKey
import co.typie.editor.ffi.KeyEvent as FfiKeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.matchesKeyBinding
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.awaitWithBringIntoView
import co.typie.editor.scroll.syncWithBringIntoView
import co.typie.ext.TextInputClient
import co.typie.ext.TextInputKey
import co.typie.ext.notifyTextInputFocusChanged
import co.typie.ext.registerTextInputClient
import co.typie.platform.Platform
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.launch

internal fun Modifier.editorInput(
  editor: Editor,
  uiState: EditorUiState,
  platform: Platform,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  textInputSessionEnabled: Boolean,
  suppressSoftwareKeyboard: Boolean,
): Modifier =
  this then
    EditorInputElement(
      editor = editor,
      uiState = uiState,
      platform = platform,
      bringIntoViewRequests = bringIntoViewRequests,
      textInputSessionEnabled = textInputSessionEnabled,
      suppressSoftwareKeyboard = suppressSoftwareKeyboard,
    )

@OptIn(ExperimentalComposeUiApi::class)
internal expect suspend fun PlatformTextInputSessionScope.createEditorInputRequest(
  editor: Editor,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  onEditCommand: (List<EditCommand>) -> Unit,
  focusedRectInRoot: () -> Rect?,
  textFieldRectInRoot: () -> Rect?,
  textClippingRectInRoot: () -> Rect?,
  suppressSoftwareKeyboard: Boolean,
): PlatformTextInputMethodRequest

internal expect fun requiresEditorInputSessionRestartForSoftwareKeyboardSuppression(): Boolean

internal fun shouldRestartEditorInputSession(
  previousTextInputSessionEnabled: Boolean,
  textInputSessionEnabled: Boolean,
  previousSuppressSoftwareKeyboard: Boolean,
  suppressSoftwareKeyboard: Boolean,
  restartOnSoftwareKeyboardSuppressionChange: Boolean =
    requiresEditorInputSessionRestartForSoftwareKeyboardSuppression(),
): Boolean =
  previousTextInputSessionEnabled != textInputSessionEnabled ||
    (previousSuppressSoftwareKeyboard != suppressSoftwareKeyboard &&
      restartOnSoftwareKeyboardSuppressionChange)

internal fun requiresRawKeyTextFallback(platform: Platform): Boolean =
  platform == Platform.Android || platform == Platform.Desktop

internal fun fixedLocalCaretTextFieldRectInRoot(
  focusedRectInRoot: Rect?,
  textClippingRectInRoot: Rect?,
  fallbackRectInRoot: Rect?,
): Rect? {
  val focused = focusedRectInRoot ?: return fallbackRectInRoot
  val rightBoundary = textClippingRectInRoot?.right ?: fallbackRectInRoot?.right ?: focused.right
  return Rect(
    left = focused.left,
    top = focused.top,
    right = maxOf(focused.right, focused.left + 1f, rightBoundary),
    bottom = maxOf(focused.bottom, focused.top + 1f),
  )
}

private data class EditorInputElement(
  private val editor: Editor,
  private val uiState: EditorUiState,
  private val platform: Platform,
  private val bringIntoViewRequests: EditorBringIntoViewRequests,
  private val textInputSessionEnabled: Boolean,
  private val suppressSoftwareKeyboard: Boolean,
) : ModifierNodeElement<EditorInputNode>() {
  override fun create(): EditorInputNode =
    EditorInputNode(
      editor = editor,
      uiState = uiState,
      platform = platform,
      bringIntoViewRequests = bringIntoViewRequests,
      textInputSessionEnabled = textInputSessionEnabled,
      suppressSoftwareKeyboard = suppressSoftwareKeyboard,
    )

  override fun update(node: EditorInputNode) {
    node.editor = editor
    node.uiState = uiState
    node.updatePlatform(platform)
    node.bringIntoViewRequests = bringIntoViewRequests
    node.updateInputSessionPolicy(
      textInputSessionEnabled = textInputSessionEnabled,
      suppressSoftwareKeyboard = suppressSoftwareKeyboard,
    )
  }
}

@OptIn(ExperimentalComposeUiApi::class)
internal class EditorInputNode(
  var editor: Editor,
  var uiState: EditorUiState,
  var platform: Platform,
  var bringIntoViewRequests: EditorBringIntoViewRequests,
  textInputSessionEnabled: Boolean,
  suppressSoftwareKeyboard: Boolean,
) : Modifier.Node(), FocusEventModifierNode, PlatformTextInputModifierNode, KeyInputModifierNode {
  private var focusedJob: Job? = null
  private var focused = false
  private var bindings = createBindings(platform)
    private set

  private var textInputSessionEnabled = textInputSessionEnabled
  private var suppressSoftwareKeyboard = suppressSoftwareKeyboard
  private val platformInputBridge = EditorPlatformInputBridge()

  fun updatePlatform(platform: Platform) {
    if (this.platform == platform) return

    this.platform = platform
    bindings = createBindings(platform)
    platformInputBridge.reset()
  }

  fun updateInputSessionPolicy(
    textInputSessionEnabled: Boolean,
    suppressSoftwareKeyboard: Boolean,
  ) {
    val shouldRestart =
      shouldRestartEditorInputSession(
        previousTextInputSessionEnabled = this.textInputSessionEnabled,
        textInputSessionEnabled = textInputSessionEnabled,
        previousSuppressSoftwareKeyboard = this.suppressSoftwareKeyboard,
        suppressSoftwareKeyboard = suppressSoftwareKeyboard,
      )
    if (
      this.textInputSessionEnabled == textInputSessionEnabled &&
        this.suppressSoftwareKeyboard == suppressSoftwareKeyboard
    ) {
      return
    }

    this.textInputSessionEnabled = textInputSessionEnabled
    this.suppressSoftwareKeyboard = suppressSoftwareKeyboard
    if (shouldRestart) {
      syncTextInputSession()
    }
  }

  private fun dispatch(
    messages: List<Message>,
    bringIntoViewTarget: EditorBringIntoViewTarget? = EditorBringIntoViewTarget.CurrentCursorLine,
  ) {
    if (messages.isEmpty()) return
    coroutineScope.launch {
      editor.awaitWithBringIntoView(bringIntoViewRequests) {
        messages.forEach(::enqueue)
        beforeCommit { bringIntoViewTarget?.let { target -> bringIntoView(target) } }
      }
    }
  }

  private fun dispatchSync(
    messages: List<Message>,
    bringIntoViewTarget: EditorBringIntoViewTarget? = EditorBringIntoViewTarget.CurrentCursorLine,
  ) {
    if (messages.isEmpty()) return
    editor.syncWithBringIntoView(bringIntoViewRequests) {
      messages.forEach(::enqueue)
      beforeCommit { bringIntoViewTarget?.let { target -> bringIntoView(target) } }
    }
  }

  private val textInputClient =
    object : TextInputClient {
      override val hasActiveComposition: Boolean
        get() = editor.ime?.composing != null

      override fun requestFocus() {
        editor.focus()
      }

      override fun insertText(text: String): Boolean {
        dispatch(listOf(Message.Insertion(InsertionOp.Text(text))))
        return true
      }

      override fun commitText(text: String) {
        if (text == "\n") {
          dispatch(listOf(Message.Insertion(InsertionOp.Text("\n"))))
        } else {
          dispatch(listOf(Message.Composition(CompositionOp.Commit(text))))
        }
      }

      override fun setComposingText(text: String) {
        dispatch(listOf(Message.Composition(CompositionOp.Update(text, null))))
      }

      override fun finishComposition() {
        dispatch(listOf(Message.Composition(CompositionOp.CommitAsIs)))
      }

      override fun pressKey(key: TextInputKey): Boolean {
        val ffiKey =
          when (key) {
            TextInputKey.Enter -> FfiKey.Enter
            TextInputKey.Backspace -> FfiKey.Backspace
          }
        dispatch(listOf(Message.Key(FfiKeyEvent(ffiKey))))
        return true
      }

      override fun dismiss() {
        editor.blur()
      }
    }

  override fun onKeyEvent(event: KeyEvent): Boolean {
    if (event.type != KeyEventType.KeyDown) return false
    val binding = bindings.find { matchesKeyBinding(it, platform, event) }
    if (binding != null) {
      if (platformInputBridge.shouldConsumeKeyEvent(event)) {
        return true
      }
      dispatch(messages = binding.action(editor), bringIntoViewTarget = binding.bringIntoViewTarget)
      return true
    }
    if (!requiresRawKeyTextFallback(platform)) {
      return false
    }

    val cp = event.utf16CodePoint
    if (cp > 0xFFFF) {
      val text =
        charArrayOf(
            (((cp - 0x10000) ushr 10) + 0xD800).toChar(),
            (((cp - 0x10000) and 0x3FF) + 0xDC00).toChar(),
          )
          .concatToString()
      dispatch(listOf(Message.Insertion(InsertionOp.Text(text))))
      return true
    }

    val ch = cp.toChar()
    if (!ch.isDefined() || ch.isISOControl() || ch.isSurrogate()) return false

    dispatch(listOf(Message.Insertion(InsertionOp.Text(ch.toString()))))
    return true
  }

  override fun onPreKeyEvent(event: KeyEvent): Boolean {
    if (event.type != KeyEventType.KeyDown) return false
    val binding = bindings.find { matchesKeyBinding(it, platform, event) } ?: return false
    return platformInputBridge.onPreKeyEvent(
      event = event,
      selection = editor.ime?.selection,
      inputCoroutineScope = coroutineScope,
      dispatch = {
        dispatch(
          messages = binding.action(editor),
          bringIntoViewTarget = binding.bringIntoViewTarget,
        )
      },
    )
  }

  override fun onFocusEvent(focusState: FocusState) {
    focused = focusState.isFocused
    syncTextInputSession()
  }

  private fun syncTextInputSession() {
    val sessionEnabled = focused && textInputSessionEnabled
    focusedJob?.cancel()
    focusedJob = null
    platformInputBridge.reset()
    notifyTextInputFocusChanged(this, sessionEnabled)
    registerTextInputClient(this, if (sessionEnabled) textInputClient else null)
    focusedJob =
      if (sessionEnabled) {
        coroutineScope.launch {
          val uninstallPlatformSessionEffects =
            platformInputBridge.installSessionEffects(
              cursor = { editor.cursor },
              viewportTransform = { uiState.resolveViewportTransform(editor.pageSizes) },
              dispatch = { messages -> dispatch(messages) },
            )
          try {
            establishTextInputSession {
              val request =
                createEditorInputRequest(
                  editor = editor,
                  bringIntoViewRequests = bringIntoViewRequests,
                  onEditCommand = { commands ->
                    dispatchSync(
                      platformInputBridge.interceptImeMessages(
                        EditorImeCommandNormalizer.normalize(commands = commands, ime = editor.ime)
                      )
                    )
                  },
                  focusedRectInRoot = { uiState.cursorRectInRoot(editor.cursor) },
                  textFieldRectInRoot = uiState::editorRectInRoot,
                  textClippingRectInRoot = uiState::textClippingRectInRoot,
                  suppressSoftwareKeyboard = suppressSoftwareKeyboard,
                )
              launch {
                notifyImeSelectionChanged(editor)
                snapshotFlow { editor.selection to editor.cursor }
                  .distinctUntilChanged()
                  .drop(1) // initial emission already handled above
                  .collect { notifyImeSelectionChanged(editor) }
              }

              startInputMethod(request)
            }
          } finally {
            uninstallPlatformSessionEffects()
          }
        }
      } else {
        null
      }
  }

  override fun onDetach() {
    focused = false
    platformInputBridge.reset()
    notifyTextInputFocusChanged(this, false)
    registerTextInputClient(this, null)
    focusedJob?.cancel()
    super.onDetach()
  }
}

@OptIn(ExperimentalComposeUiApi::class)
internal expect fun PlatformTextInputSessionScope.notifyImeSelectionChanged(editor: Editor)
