package co.typie.editor.input

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
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.InsertionOp
import co.typie.editor.ffi.Key as FfiKey
import co.typie.editor.ffi.KeyEvent as FfiKeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.StateField
import co.typie.editor.handleKeyDown
import co.typie.editor.scroll.EditorScrollController
import co.typie.editor.scroll.EditorScrollTarget
import co.typie.ext.TextInputClient
import co.typie.ext.TextInputKey
import co.typie.ext.notifyTextInputFocusChanged
import co.typie.ext.registerTextInputClient
import co.typie.platform.Platform
import kotlinx.coroutines.Job
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.launch

internal fun Modifier.editorInput(
  editor: Editor,
  platform: Platform,
  scrollController: EditorScrollController?,
): Modifier = this then EditorInputElement(editor, platform, scrollController)

@OptIn(ExperimentalComposeUiApi::class)
internal expect suspend fun PlatformTextInputSessionScope.createEditorInputRequest(
  editor: Editor
): PlatformTextInputMethodRequest

private data class EditorInputElement(
  private val editor: Editor,
  private val platform: Platform,
  private val scrollController: EditorScrollController?,
) : ModifierNodeElement<EditorInputNode>() {
  override fun create(): EditorInputNode = EditorInputNode(editor, platform, scrollController)

  override fun update(node: EditorInputNode) {
    node.editor = editor
    node.platform = platform
    node.scrollController = scrollController
  }
}

@OptIn(ExperimentalComposeUiApi::class)
internal class EditorInputNode(
  var editor: Editor,
  var platform: Platform,
  var scrollController: EditorScrollController?,
) : Modifier.Node(), FocusEventModifierNode, PlatformTextInputModifierNode, KeyInputModifierNode {
  private var focusedJob: Job? = null
  private val bindings by lazy { createBindings(platform) }

  private fun dispatchForCurrentCursor(vararg messages: Message) {
    // TODO(editor-parity): 입력 후 스크롤 요청 대상을 현재 cursor로 고정하지 말고,
    // dispatch 결과의 실제 scroll anchor(selection head 또는 cursor)를 기준으로 정해야
    // 확장 selection/IME 조합에서도 웹·플러터와 같은 동작이 나온다.
    val controller = scrollController
    coroutineScope.launch {
      editor.dispatch(*messages)
      controller?.request(target = EditorScrollTarget.CurrentCursor)
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
        dispatchForCurrentCursor(Message.Insertion(InsertionOp.Text(text)))
        return true
      }

      override fun commitText(text: String) {
        if (text == "\n") {
          dispatchForCurrentCursor(Message.Insertion(InsertionOp.Text("\n")))
        } else {
          dispatchForCurrentCursor(Message.Composition(CompositionOp.Commit(text)))
        }
      }

      override fun setComposingText(text: String) {
        dispatchForCurrentCursor(Message.Composition(CompositionOp.Update(text, null)))
      }

      override fun finishComposition() {
        dispatchForCurrentCursor(Message.Composition(CompositionOp.CommitAsIs))
      }

      override fun pressKey(key: TextInputKey): Boolean {
        val ffiKey =
          when (key) {
            TextInputKey.Enter -> FfiKey.Enter
            TextInputKey.Backspace -> FfiKey.Backspace
          }
        dispatchForCurrentCursor(Message.Key(FfiKeyEvent(ffiKey)))
        return true
      }

      override fun dismiss() {
        editor.blur()
      }
    }

  override fun onKeyEvent(event: KeyEvent): Boolean {
    if (event.type != KeyEventType.KeyDown) return false
    if (handleKeyDown(editor, platform, bindings, scrollController, coroutineScope, event)) {
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
      dispatchForCurrentCursor(Message.Insertion(InsertionOp.Text(text)))
      return true
    }

    val ch = cp.toChar()
    if (!ch.isDefined() || ch.isISOControl() || ch.isSurrogate()) return false

    dispatchForCurrentCursor(Message.Insertion(InsertionOp.Text(ch.toString())))
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
              val unsubscribe =
                editor.on<EditorEvent.StateChanged> { _, event ->
                  if (StateField.Selection in event.fields || StateField.Cursor in event.fields) {
                    notifyImeSelectionChanged(editor)
                  }
                }

              try {
                notifyImeSelectionChanged(editor)
                awaitCancellation()
              } finally {
                unsubscribe()
              }
            }

            startInputMethod(request)
          }
        }
      } else {
        null
      }
  }

  override fun onDetach() {
    scrollController = null
    notifyTextInputFocusChanged(this, false)
    registerTextInputClient(this, null)
    focusedJob?.cancel()
    super.onDetach()
  }
}

@OptIn(ExperimentalComposeUiApi::class)
internal expect fun PlatformTextInputSessionScope.notifyImeSelectionChanged(editor: Editor)
