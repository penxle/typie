package co.typie.editor.compose

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
import co.typie.di.Platform
import co.typie.editor.Editor
import co.typie.editor.createBindings
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.InsertionIntent
import co.typie.editor.ffi.Intent
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.StateField
import co.typie.editor.handleKeyDown
import kotlinx.coroutines.Job
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.launch

fun Modifier.editorInput(editor: Editor, platform: Platform): Modifier =
  this then EditorInputElement(editor, platform)

@OptIn(ExperimentalComposeUiApi::class)
internal expect suspend fun PlatformTextInputSessionScope.createEditorInputRequest(
  editor: Editor,
): PlatformTextInputMethodRequest

private data class EditorInputElement(
  private val editor: Editor,
  private val platform: Platform,
) : ModifierNodeElement<EditorInputNode>() {
  override fun create(): EditorInputNode = EditorInputNode(editor, platform)
  override fun update(node: EditorInputNode) {
    node.editor = editor
    node.platform = platform
  }
}

@OptIn(ExperimentalComposeUiApi::class)
internal class EditorInputNode(
  var editor: Editor,
  var platform: Platform,
) : Modifier.Node(), FocusEventModifierNode, PlatformTextInputModifierNode, KeyInputModifierNode {
  private var focusedJob: Job? = null
  private val bindings by lazy { createBindings(platform) }

  override fun onKeyEvent(event: KeyEvent): Boolean {
    if (event.type != KeyEventType.KeyDown) return false
    if (handleKeyDown(editor, platform, bindings, event)) return true

    val cp = event.utf16CodePoint
    if (cp > 0xFFFF) {
      val text = charArrayOf(
        (((cp - 0x10000) ushr 10) + 0xD800).toChar(),
        (((cp - 0x10000) and 0x3FF) + 0xDC00).toChar(),
      ).concatToString()
      editor.enqueue(Message.Intent(Intent.Insertion(InsertionIntent.Text(text))))
      return true
    }

    val ch = cp.toChar()
    if (!ch.isDefined() || ch.isISOControl()) return false

    editor.enqueue(Message.Intent(Intent.Insertion(InsertionIntent.Text(ch.toString()))))
    return true
  }

  override fun onPreKeyEvent(event: KeyEvent) = false

  override fun onFocusEvent(focusState: FocusState) {
    focusedJob?.cancel()
    focusedJob = if (focusState.isFocused) {
      coroutineScope.launch {
        establishTextInputSession {
          val request = createEditorInputRequest(editor)
          launch {
            val unsubscribe = editor.on<EditorEvent.StateChanged> { _, event ->
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
}

@OptIn(ExperimentalComposeUiApi::class)
internal expect fun PlatformTextInputSessionScope.notifyImeSelectionChanged(editor: Editor)
