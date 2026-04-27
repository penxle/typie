package co.typie.editor.input

import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusEventModifierNode
import androidx.compose.ui.focus.FocusState
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
import co.typie.editor.Editor
import co.typie.editor.createBindings
import co.typie.editor.ffi.CompositionOp
import co.typie.editor.ffi.InsertionOp
import co.typie.editor.ffi.Key as FfiKey
import co.typie.editor.ffi.KeyEvent as FfiKeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.handleKeyDown
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.awaitWithBringIntoView
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
  platform: Platform,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  textInputSessionEnabled: Boolean,
  suppressSoftwareKeyboard: Boolean,
): Modifier =
  this then
    EditorInputElement(
      editor = editor,
      platform = platform,
      bringIntoViewRequests = bringIntoViewRequests,
      textInputSessionEnabled = textInputSessionEnabled,
      suppressSoftwareKeyboard = suppressSoftwareKeyboard,
    )

@OptIn(ExperimentalComposeUiApi::class)
internal expect suspend fun PlatformTextInputSessionScope.createEditorInputRequest(
  editor: Editor,
  bringIntoViewRequests: EditorBringIntoViewRequests,
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

private data class EditorInputElement(
  private val editor: Editor,
  private val platform: Platform,
  private val bringIntoViewRequests: EditorBringIntoViewRequests,
  private val textInputSessionEnabled: Boolean,
  private val suppressSoftwareKeyboard: Boolean,
) : ModifierNodeElement<EditorInputNode>() {
  override fun create(): EditorInputNode =
    EditorInputNode(
      editor = editor,
      platform = platform,
      bringIntoViewRequests = bringIntoViewRequests,
      textInputSessionEnabled = textInputSessionEnabled,
      suppressSoftwareKeyboard = suppressSoftwareKeyboard,
    )

  override fun update(node: EditorInputNode) {
    node.editor = editor
    node.platform = platform
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
  var platform: Platform,
  var bringIntoViewRequests: EditorBringIntoViewRequests,
  textInputSessionEnabled: Boolean,
  suppressSoftwareKeyboard: Boolean,
) : Modifier.Node(), FocusEventModifierNode, PlatformTextInputModifierNode, KeyInputModifierNode {
  private var focusedJob: Job? = null
  private var focused = false
  private val bindings by lazy { createBindings(platform) }
  private var textInputSessionEnabled = textInputSessionEnabled
  private var suppressSoftwareKeyboard = suppressSoftwareKeyboard

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

  private fun dispatchAndScrollToCurrentCursorLine(vararg messages: Message) {
    coroutineScope.launch {
      editor.awaitWithBringIntoView(bringIntoViewRequests) {
        messages.forEach(::enqueue)
        beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentCursorLine) }
      }
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
        dispatchAndScrollToCurrentCursorLine(Message.Insertion(InsertionOp.Text(text)))
        return true
      }

      override fun commitText(text: String) {
        if (text == "\n") {
          dispatchAndScrollToCurrentCursorLine(Message.Insertion(InsertionOp.Text("\n")))
        } else {
          dispatchAndScrollToCurrentCursorLine(Message.Composition(CompositionOp.Commit(text)))
        }
      }

      override fun setComposingText(text: String) {
        dispatchAndScrollToCurrentCursorLine(Message.Composition(CompositionOp.Update(text, null)))
      }

      override fun finishComposition() {
        dispatchAndScrollToCurrentCursorLine(Message.Composition(CompositionOp.CommitAsIs))
      }

      override fun pressKey(key: TextInputKey): Boolean {
        val ffiKey =
          when (key) {
            TextInputKey.Enter -> FfiKey.Enter
            TextInputKey.Backspace -> FfiKey.Backspace
          }
        dispatchAndScrollToCurrentCursorLine(Message.Key(FfiKeyEvent(ffiKey)))
        return true
      }

      override fun dismiss() {
        editor.blur()
      }
    }

  override fun onKeyEvent(event: KeyEvent): Boolean {
    if (event.type != KeyEventType.KeyDown) return false
    if (handleKeyDown(editor, platform, bindings, bringIntoViewRequests, coroutineScope, event)) {
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
      dispatchAndScrollToCurrentCursorLine(Message.Insertion(InsertionOp.Text(text)))
      return true
    }

    val ch = cp.toChar()
    if (!ch.isDefined() || ch.isISOControl() || ch.isSurrogate()) return false

    dispatchAndScrollToCurrentCursorLine(Message.Insertion(InsertionOp.Text(ch.toString())))
    return true
  }

  override fun onPreKeyEvent(event: KeyEvent) = false

  override fun onFocusEvent(focusState: FocusState) {
    focused = focusState.isFocused
    syncTextInputSession()
  }

  private fun syncTextInputSession() {
    val sessionEnabled = focused && textInputSessionEnabled
    focusedJob?.cancel()
    focusedJob = null
    notifyTextInputFocusChanged(this, sessionEnabled)
    registerTextInputClient(this, if (sessionEnabled) textInputClient else null)
    focusedJob =
      if (sessionEnabled) {
        coroutineScope.launch {
          establishTextInputSession {
            val request =
              createEditorInputRequest(
                editor = editor,
                bringIntoViewRequests = bringIntoViewRequests,
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
        }
      } else {
        null
      }
  }

  override fun onDetach() {
    focused = false
    notifyTextInputFocusChanged(this, false)
    registerTextInputClient(this, null)
    focusedJob?.cancel()
    super.onDetach()
  }
}

@OptIn(ExperimentalComposeUiApi::class)
internal expect fun PlatformTextInputSessionScope.notifyImeSelectionChanged(editor: Editor)
