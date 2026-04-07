@file:OptIn(ExperimentalForeignApi::class, ExperimentalComposeUiApi::class)

package co.typie.editor.compose

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
import androidx.compose.ui.text.input.TextEditingScope
import androidx.compose.ui.text.input.TextEditorState
import androidx.compose.ui.text.input.TextFieldValue
import co.typie.editor.Editor
import co.typie.editor.InputEditCommandHandler
import kotlinx.cinterop.ExperimentalForeignApi

internal actual suspend fun PlatformTextInputSessionScope.createEditorInputRequest(
  editor: Editor,
): PlatformTextInputMethodRequest = object : PlatformTextInputMethodRequest {
  override val value: () -> TextFieldValue = value@{
    val ctx = editor.inputContext ?: return@value TextFieldValue()
    val selectionStart = ctx.selection.start - ctx.windowStart
    val selectionEnd = ctx.selection.end - ctx.windowStart

    TextFieldValue(
      text = ctx.text,
      selection = TextRange(selectionStart, selectionEnd),
      composition = ctx.composing?.let {
        TextRange(
          it.start - ctx.windowStart,
          it.end - ctx.windowStart
        )
      },
    )
  }

  override val imeOptions: ImeOptions = ImeOptions(
    autoCorrect = true,
    capitalization = KeyboardCapitalization.None,
    imeAction = ImeAction.Default,
    keyboardType = KeyboardType.Text,
    singleLine = false,
  )

  override val onEditCommand: (List<EditCommand>) -> Unit = { commands ->
    InputEditCommandHandler.handle(editor, commands)
  }

  override val onImeAction: ((ImeAction) -> Unit)? = null

  override val focusedRectInRoot: () -> Rect? = { null }

  override val textLayoutResult: () -> TextLayoutResult? = { null }

  // 커서 좌표 연동할 때 요거 써야 함 (snapshot flow로 reactive하게 추적됨)
  override val textFieldRectInRoot: () -> Rect? = { null }

  override val textClippingRectInRoot: () -> Rect? = { null }

  override val state: TextEditorState =
    object : TextEditorState {
      override val selection: TextRange get() = value().selection
      override val composition: TextRange? get() = value().composition
      override val length: Int get() = value().text.length
      override fun get(index: Int): Char = value().text[index]
      override fun subSequence(startIndex: Int, endIndex: Int): CharSequence =
        value().text.subSequence(startIndex, endIndex)
    }

  override val editText: (block: TextEditingScope.() -> Unit) -> Unit =
    { _ -> }
}

internal actual fun PlatformTextInputSessionScope.notifyImeSelectionChanged(editor: Editor) {
  // iOS: pull-based via request.value — no explicit notification needed
}
