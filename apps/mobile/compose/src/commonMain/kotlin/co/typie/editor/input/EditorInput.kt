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
import co.typie.ext.TextInputClient
import co.typie.ext.TextInputKey
import co.typie.ext.notifyTextInputFocusChanged
import co.typie.ext.registerTextInputClient
import co.typie.platform.Platform
import kotlinx.coroutines.Job
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.launch

fun Modifier.editorInput(editor: Editor, platform: Platform): Modifier =
  this then EditorInputElement(editor, platform)

@OptIn(ExperimentalComposeUiApi::class)
internal expect suspend fun PlatformTextInputSessionScope.createEditorInputRequest(
  editor: Editor
): PlatformTextInputMethodRequest

private data class EditorInputElement(private val editor: Editor, private val platform: Platform) :
  ModifierNodeElement<EditorInputNode>() {
  override fun create(): EditorInputNode = EditorInputNode(editor, platform)

  override fun update(node: EditorInputNode) {
    node.editor = editor
    node.platform = platform
  }
}

@OptIn(ExperimentalComposeUiApi::class)
internal class EditorInputNode(var editor: Editor, var platform: Platform) :
  Modifier.Node(), FocusEventModifierNode, PlatformTextInputModifierNode, KeyInputModifierNode {
  private var focusedJob: Job? = null
  private val bindings by lazy { createBindings(platform) }
  private val textInputClient =
    object : TextInputClient {
      override val hasActiveComposition: Boolean
        get() = editor.ime?.composing != null

      override fun requestFocus() {
        editor.focus()
      }

      override fun insertText(text: String): Boolean {
        editor.enqueue(Message.Insertion(InsertionOp.Text(text)))
        return true
      }

      override fun commitText(text: String) {
        if (text == "\n") {
          editor.enqueue(Message.Insertion(InsertionOp.Text("\n")))
        } else {
          editor.enqueue(Message.Composition(CompositionOp.Commit(text)))
        }
      }

      override fun setComposingText(text: String) {
        editor.enqueue(Message.Composition(CompositionOp.Update(text, null)))
      }

      override fun finishComposition() {
        editor.enqueue(Message.Composition(CompositionOp.CommitAsIs))
      }

      override fun pressKey(key: TextInputKey): Boolean {
        val ffiKey =
          when (key) {
            TextInputKey.Enter -> FfiKey.Enter
            TextInputKey.Backspace -> FfiKey.Backspace
          }
        editor.enqueue(Message.Key(FfiKeyEvent(ffiKey)))
        return true
      }

      override fun dismiss() {
        editor.blur()
      }
    }

  override fun onKeyEvent(event: KeyEvent): Boolean {
    if (event.type != KeyEventType.KeyDown) return false
    if (handleKeyDown(editor, platform, bindings, event)) return true

    val cp = event.utf16CodePoint
    if (cp > 0xFFFF) {
      val text =
        charArrayOf(
            (((cp - 0x10000) ushr 10) + 0xD800).toChar(),
            (((cp - 0x10000) and 0x3FF) + 0xDC00).toChar(),
          )
          .concatToString()
      editor.enqueue(Message.Insertion(InsertionOp.Text(text)))
      return true
    }

    val ch = cp.toChar()
    if (!ch.isDefined() || ch.isISOControl() || ch.isSurrogate()) return false

    editor.enqueue(Message.Insertion(InsertionOp.Text(ch.toString())))
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
    notifyTextInputFocusChanged(this, false)
    registerTextInputClient(this, null)
    focusedJob?.cancel()
    super.onDetach()
  }
}

@OptIn(ExperimentalComposeUiApi::class)
internal expect fun PlatformTextInputSessionScope.notifyImeSelectionChanged(editor: Editor)
