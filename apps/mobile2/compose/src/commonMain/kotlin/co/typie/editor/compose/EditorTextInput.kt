package co.typie.editor.compose

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusEventModifierNode
import androidx.compose.ui.focus.FocusState
import androidx.compose.ui.node.ModifierNodeElement
import androidx.compose.ui.platform.PlatformTextInputMethodRequest
import androidx.compose.ui.platform.PlatformTextInputModifierNode
import androidx.compose.ui.platform.PlatformTextInputSessionScope
import androidx.compose.ui.platform.establishTextInputSession
import co.typie.editor.Editor
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.StateField
import kotlinx.coroutines.Job
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.launch

fun Modifier.editorTextInput(editor: Editor): Modifier =
  this then EditorTextInputElement(editor)

@OptIn(ExperimentalComposeUiApi::class)
internal expect suspend fun PlatformTextInputSessionScope.createEditorTextInputRequest(
  editor: Editor,
): PlatformTextInputMethodRequest

private data class EditorTextInputElement(
  private val editor: Editor,
) : ModifierNodeElement<EditorTextInputNode>() {
  override fun create(): EditorTextInputNode = EditorTextInputNode(editor)
  override fun update(node: EditorTextInputNode) {
    node.editor = editor
  }
}

@OptIn(ExperimentalComposeUiApi::class)
internal class EditorTextInputNode(
  var editor: Editor,
) : Modifier.Node(), FocusEventModifierNode, PlatformTextInputModifierNode {
  private var focusedJob: Job? = null

  override fun onFocusEvent(focusState: FocusState) {
    focusedJob?.cancel()
    focusedJob = if (focusState.isFocused) {
      coroutineScope.launch {
        establishTextInputSession {
          val request = createEditorTextInputRequest(editor)
          launch {
            val unsubscribe = editor.on<EditorEvent.StateChanged> { _, event ->
              if (StateField.Selection in event.fields || StateField.Cursor in event.fields) {
                notifyImeSelectionChanged(editor)
              }
            }
            try {
              // Subscribe first, then fire the initial sync — this eliminates
              // the window where a StateChanged could slip between the read
              // and the listener registration. The IME is otherwise stuck at
              // whatever initialSelStart/End the EditorInfo declared.
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
