package co.typie.editor.input

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.platform.PlatformTextInputMethodRequest
import androidx.compose.ui.platform.PlatformTextInputSessionScope
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.CommitTextCommand
import androidx.compose.ui.text.input.DeleteSurroundingTextInCodePointsCommand
import androidx.compose.ui.text.input.EditCommand
import androidx.compose.ui.text.input.FinishComposingTextCommand
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.ImeOptions
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.SetComposingTextCommand
import androidx.compose.ui.text.input.TextEditingScope
import androidx.compose.ui.text.input.TextEditorState
import androidx.compose.ui.text.input.TextFieldValue
import co.typie.editor.Editor
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.platform.IncomingContentCandidates

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
  onIncomingContent: (IncomingContentCandidates) -> Boolean,
): PlatformTextInputMethodRequest {
  return object : PlatformTextInputMethodRequest {
    override val value: () -> TextFieldValue = {
      editor.appliedState.ime?.toTextFieldValue() ?: TextFieldValue()
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

    // Desktop's AWT IME callback edits through TextEditingScope rather than onEditCommand.
    // Adapt it to the common onEditCommand normalization and apply pipeline.
    override val editText: (block: TextEditingScope.() -> Unit) -> Unit = editText@{ block ->
      val batch = EditorDesktopTextEditingBatch()
      batch.block()
      val commands = batch.drainCommands()
      if (commands.isNotEmpty()) onEditCommand(commands)
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
internal class EditorDesktopTextEditingBatch : TextEditingScope {
  private val commands = mutableListOf<EditCommand>()

  override fun commitText(text: CharSequence, newCursorPosition: Int) {
    commands += CommitTextCommand(text.toString(), newCursorPosition)
  }

  override fun setComposingText(text: CharSequence, newCursorPosition: Int) {
    commands += SetComposingTextCommand(text.toString(), newCursorPosition)
  }

  override fun finishComposingText() {
    commands += FinishComposingTextCommand()
  }

  override fun deleteSurroundingTextInCodePoints(lengthBeforeCursor: Int, lengthAfterCursor: Int) {
    commands += DeleteSurroundingTextInCodePointsCommand(lengthBeforeCursor, lengthAfterCursor)
  }

  fun drainCommands(): List<EditCommand> {
    val drained = commands.toList()
    commands.clear()
    return drained
  }
}
