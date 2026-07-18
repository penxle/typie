package co.typie.editor.input

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.platform.PlatformTextInputMethodRequest
import androidx.compose.ui.platform.PlatformTextInputSessionScope
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.EditCommand
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.ImeOptions
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.TextEditingScope
import androidx.compose.ui.text.input.TextEditorState
import androidx.compose.ui.text.input.TextFieldValue
import co.typie.editor.Editor
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Key
import co.typie.editor.ffi.KeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.syncWithBringIntoView

@OptIn(ExperimentalComposeUiApi::class)
internal actual suspend fun PlatformTextInputSessionScope.createEditorInputRequest(
  editor: Editor,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  onEditCommand: (List<EditCommand>) -> Unit,
  focusedRectInRoot: () -> Rect?,
  textFieldRectInRoot: () -> Rect?,
  textClippingRectInRoot: () -> Rect?,
  suppressSoftwareKeyboard: Boolean,
  isSessionCurrent: () -> Boolean,
): PlatformTextInputMethodRequest {
  return object : PlatformTextInputMethodRequest {
    override val value: () -> TextFieldValue = {
      editor.ime?.toTextFieldValue() ?: TextFieldValue()
    }

    override val imeOptions: ImeOptions =
      ImeOptions(
        autoCorrect = true,
        capitalization = KeyboardCapitalization.None,
        imeAction = ImeAction.Default,
        keyboardType = KeyboardType.Text,
        singleLine = false,
      )

    override val onEditCommand: (List<EditCommand>) -> Unit = { commands ->
      onEditCommand(commands)
    }

    override val onImeAction: ((ImeAction) -> Unit)? = null

    override val focusedRectInRoot: () -> Rect? = focusedRectInRoot

    override val textLayoutResult: () -> TextLayoutResult? = { null }

    override val textFieldRectInRoot: () -> Rect? = textFieldRectInRoot

    override val textClippingRectInRoot: () -> Rect? = textClippingRectInRoot

    @ExperimentalComposeUiApi override val unclippedTextOffsetInRoot: () -> Offset? = { null }

    override val state: TextEditorState =
      object : TextEditorState {
        override val selection: TextRange
          get() = value().selection

        override val composition: TextRange?
          get() = value().composition

        override val length: Int
          get() = value().text.length

        override fun get(index: Int): Char = value().text[index]

        override fun subSequence(startIndex: Int, endIndex: Int): CharSequence =
          value().text.subSequence(startIndex, endIndex)
      }

    override val editText: (block: TextEditingScope.() -> Unit) -> Unit = { block ->
      editor.syncWithBringIntoView(bringIntoViewRequests) {
        val batch =
          EditorDesktopTextEditingBatch(initialHasActiveComposition = editor.ime?.composing != null)
        batch.block()
        for (message in batch.drainMessages()) {
          enqueue(message)
        }
        beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentSelectionHead) }
      }
    }
  }
}

internal actual fun requiresEditorInputSessionRestartForSoftwareKeyboardSuppression(): Boolean =
  false

@OptIn(ExperimentalComposeUiApi::class)
internal actual fun PlatformTextInputSessionScope.notifyImeStateChanged(editor: Editor) {
  // Desktop Skiko: pull-based via request.value — no explicit notification needed.
}

@OptIn(ExperimentalComposeUiApi::class)
internal class EditorDesktopTextEditingBatch(initialHasActiveComposition: Boolean = false) :
  TextEditingScope {
  private val messages = mutableListOf<Message>()
  private val ops = mutableListOf<FlatImeOp>()
  private var hasActiveComposition = initialHasActiveComposition

  override fun commitText(text: CharSequence, newCursorPosition: Int) {
    if (text.toString() == "\n") {
      flushOps()
      messages += Message.Key(KeyEvent(Key.Enter))
    } else {
      ops += FlatImeOp.Compose(text.toString())
      ops += FlatImeOp.CommitAsIs
      hasActiveComposition = false
    }
  }

  override fun setComposingText(text: CharSequence, newCursorPosition: Int) {
    ops += FlatImeOp.Compose(text.toString())
    hasActiveComposition = true
  }

  override fun finishComposingText() {
    ops +=
      if (hasActiveComposition) {
        FlatImeOp.CommitAsIs
      } else {
        FlatImeOp.ClearComposition
      }
    hasActiveComposition = false
  }

  override fun deleteSurroundingTextInCodePoints(lengthBeforeCursor: Int, lengthAfterCursor: Int) {
    ops += FlatImeOp.DeleteSurrounding(lengthBeforeCursor, lengthAfterCursor)
  }

  fun drainMessages(): List<Message> {
    flushOps()
    val drained = messages.toList()
    messages.clear()
    return drained
  }

  private fun flushOps() {
    if (ops.isEmpty()) return
    messages += Message.TextInput(ops.toList())
    ops.clear()
  }
}
