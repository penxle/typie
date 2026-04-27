@file:OptIn(ExperimentalForeignApi::class, ExperimentalComposeUiApi::class)

package co.typie.editor.input

import androidx.compose.ui.ExperimentalComposeUiApi
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
import co.typie.editor.InputEditCommandHandler
import co.typie.editor.scroll.EditorBringIntoViewRequests
import kotlinx.cinterop.ExperimentalForeignApi
import platform.CoreGraphics.CGRectMake
import platform.UIKit.UIView

internal actual suspend fun PlatformTextInputSessionScope.createEditorInputRequest(
  editor: Editor,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  suppressSoftwareKeyboard: Boolean,
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
        platformImeOptions =
          if (suppressSoftwareKeyboard) {
            PlatformImeOptions { inputView(UIView(frame = CGRectMake(0.0, 0.0, 0.0, 0.0))) }
          } else {
            null
          },
      )

    override val onEditCommand: (List<EditCommand>) -> Unit = { commands ->
      InputEditCommandHandler.handle(editor, bringIntoViewRequests, commands)
    }

    override val onImeAction: ((ImeAction) -> Unit)? = null

    override val focusedRectInRoot: () -> Rect? = { null }

    override val textLayoutResult: () -> TextLayoutResult? = { null }

    // 커서 좌표 연동할 때 요거 써야 함 (snapshot flow로 reactive하게 추적됨)
    override val textFieldRectInRoot: () -> Rect? = { null }

    override val textClippingRectInRoot: () -> Rect? = { null }

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

internal actual fun shouldRestartEditorInputSessionOnSoftwareKeyboardSuppressionChange(): Boolean =
  true

internal actual fun PlatformTextInputSessionScope.notifyImeSelectionChanged(editor: Editor) {
  // iOS: pull-based via request.value — no explicit notification needed
}
