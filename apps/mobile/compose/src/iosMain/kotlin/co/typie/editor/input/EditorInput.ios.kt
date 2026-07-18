@file:OptIn(ExperimentalForeignApi::class, ExperimentalComposeUiApi::class)

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
import androidx.compose.ui.text.input.PlatformImeOptions
import androidx.compose.ui.text.input.TextEditingScope
import androidx.compose.ui.text.input.TextEditorState
import androidx.compose.ui.text.input.TextFieldValue
import co.typie.editor.Editor
import co.typie.editor.scroll.EditorBringIntoViewRequests
import kotlinx.cinterop.ExperimentalForeignApi
import platform.CoreGraphics.CGRectMake
import platform.UIKit.UIView

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
    private var lastPulledValue: TextFieldValue? = null

    override val value: () -> TextFieldValue = {
      val pulled = editor.ime?.toTextFieldValue() ?: TextFieldValue()
      val recorder = editor.inputRecorder
      if (recorder != null && pulled != lastPulledValue) {
        lastPulledValue = pulled
        recorder.record { seq, t ->
          RecordedInputEntry.ValuePull(
            seq = seq,
            t = t,
            text = pulled.text,
            selectionStart = pulled.selection.start,
            selectionEnd = pulled.selection.end,
            compositionStart = pulled.composition?.start,
            compositionEnd = pulled.composition?.end,
          )
        }
      }
      pulled
    }

    override val imeOptions: ImeOptions =
      ImeOptions(
        autoCorrect = true,
        capitalization = KeyboardCapitalization.None,
        imeAction = ImeAction.Default,
        keyboardType = KeyboardType.Text,
        singleLine = false,
        platformImeOptions =
          if (suppressSoftwareKeyboard) {
            PlatformImeOptions { inputView(UIView(frame = CGRectMake(0.0, 0.0, 0.0, 0.0))) }
          } else {
            null
          },
      )

    override val onEditCommand: (List<EditCommand>) -> Unit = { commands ->
      onEditCommand(commands)
    }

    override val onImeAction: ((ImeAction) -> Unit)? = null

    override val focusedRectInRoot: () -> Rect? = focusedRectInRoot

    override val textLayoutResult: () -> TextLayoutResult? = { null }

    @ExperimentalComposeUiApi override val unclippedTextOffsetInRoot: () -> Offset? = { null }

    // Workaround for Compose iOS 1.10.3: startInputMethod only uses textFieldRectInRoot
    // to position the hidden UIKit text input view, while IntermediateTextInputUIView
    // returns a fixed local caretRectForPosition(1, 1, 1, 1). Keep the hidden view's
    // origin at the caret, but expand the frame to the visible line edge so Japanese IME
    // candidates anchor near the insertion point without treating the text field as 1x1.
    override val textFieldRectInRoot: () -> Rect? = {
      fixedLocalCaretTextFieldRectInRoot(
        focusedRectInRoot = focusedRectInRoot(),
        textClippingRectInRoot = textClippingRectInRoot(),
        fallbackRectInRoot = textFieldRectInRoot(),
      )
    }

    override val textClippingRectInRoot: () -> Rect? = textClippingRectInRoot

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

    override val editText: (block: TextEditingScope.() -> Unit) -> Unit = { _ -> }
  }
}

internal actual fun requiresEditorInputSessionRestartForSoftwareKeyboardSuppression(): Boolean =
  true

internal actual fun PlatformTextInputSessionScope.notifyImeStateChanged(editor: Editor) {
  // iOS: pull-based via request.value — no explicit notification needed
}
