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
import co.typie.editor.scroll.EditorAutoScrollController
import co.typie.editor.scroll.EditorScrollTarget
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
  autoScrollController: EditorAutoScrollController?,
): Modifier = this then EditorInputElement(editor, platform, autoScrollController)

@OptIn(ExperimentalComposeUiApi::class)
internal expect suspend fun PlatformTextInputSessionScope.createEditorInputRequest(
  editor: Editor
): PlatformTextInputMethodRequest

private data class EditorInputElement(
  private val editor: Editor,
  private val platform: Platform,
  private val autoScrollController: EditorAutoScrollController?,
) : ModifierNodeElement<EditorInputNode>() {
  override fun create(): EditorInputNode = EditorInputNode(editor, platform, autoScrollController)

  override fun update(node: EditorInputNode) {
    node.editor = editor
    node.platform = platform
    node.autoScrollController = autoScrollController
  }
}

@OptIn(ExperimentalComposeUiApi::class)
internal class EditorInputNode(
  var editor: Editor,
  var platform: Platform,
  var autoScrollController: EditorAutoScrollController?,
) : Modifier.Node(), FocusEventModifierNode, PlatformTextInputModifierNode, KeyInputModifierNode {
  private var focusedJob: Job? = null
  private val bindings by lazy { createBindings(platform) }

  private fun dispatchAndScrollToCurrentCursorLine(vararg messages: Message) {
    val autoScrollController = autoScrollController
    coroutineScope.launch {
      editor.await { messages.forEach(::enqueue) }
      autoScrollController?.request(
        target = EditorScrollTarget.CurrentCursorLine,
        state = editor.state,
      )
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
    if (handleKeyDown(editor, platform, bindings, autoScrollController, coroutineScope, event)) {
      return true
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
    notifyTextInputFocusChanged(this, focusState.isFocused)
    registerTextInputClient(this, if (focusState.isFocused) textInputClient else null)
    focusedJob?.cancel()
    focusedJob =
      if (focusState.isFocused) {
        coroutineScope.launch {
          establishTextInputSession {
            val request = createEditorInputRequest(editor)
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
    autoScrollController = null
    notifyTextInputFocusChanged(this, false)
    registerTextInputClient(this, null)
    focusedJob?.cancel()
    super.onDetach()
  }
}

@OptIn(ExperimentalComposeUiApi::class)
internal expect fun PlatformTextInputSessionScope.notifyImeSelectionChanged(editor: Editor)
