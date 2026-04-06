package co.typie.editor.compose

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.platform.PlatformTextInputMethodRequest
import androidx.compose.ui.platform.PlatformTextInputSessionScope
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.EditCommand
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.ImeOptions
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.TextFieldValue
import co.typie.editor.Editor
import co.typie.editor.InputEditCommandHandler

@OptIn(ExperimentalComposeUiApi::class)
internal actual suspend fun PlatformTextInputSessionScope.createEditorInputRequest(
  editor: Editor,
): PlatformTextInputMethodRequest = object : PlatformTextInputMethodRequest {
  override val value: () -> TextFieldValue = {
    val ctx = editor.inputContext(Int.MAX_VALUE, Int.MAX_VALUE)
    val selStart = ctx.selection.start - ctx.windowStart
    val selEnd = ctx.selection.end - ctx.windowStart
    TextFieldValue(
      text = ctx.text,
      selection = TextRange(selStart, selEnd),
      composition = ctx.composing?.let {
        TextRange(
          it.start - ctx.windowStart,
          it.end - ctx.windowStart
        )
      },
    )
  }

  override val imeOptions: ImeOptions = ImeOptions(
    singleLine = false,
    capitalization = KeyboardCapitalization.Sentences,
    autoCorrect = true,
    keyboardType = KeyboardType.Text,
    imeAction = ImeAction.Default,
  )

  override val onEditCommand: (List<EditCommand>) -> Unit = { commands ->
    InputEditCommandHandler.handle(editor, commands)
  }

  override val onImeAction: ((ImeAction) -> Unit)? = null

  // Returning null until page→root coordinate translation is wired here.
  // editor.cursor is a PageRect whose `rect` is page-local; CMP requires
  // editor-root coordinates so iOS IME can position candidate bars/magnifier
  // under the cursor. pageOffsets lives in View.kt and is not reachable from
  // this session scope today — returning null makes iOS fall back to a
  // default anchor, which is wrong but not actively misleading.
  override val focusedRectInRoot: () -> Rect? = { null }

  override val textLayoutResult: () -> androidx.compose.ui.text.TextLayoutResult? = { null }

  override val textFieldRectInRoot: () -> Rect? = { null }

  override val textClippingRectInRoot: () -> Rect? = { null }

  override val state: androidx.compose.ui.text.input.TextEditorState =
    object : androidx.compose.ui.text.input.TextEditorState {
      override val selection: TextRange get() = value().selection
      override val composition: TextRange? get() = value().composition
      override val length: Int get() = value().text.length
      override fun get(index: Int): Char = value().text[index]
      override fun subSequence(startIndex: Int, endIndex: Int): CharSequence =
        value().text.subSequence(startIndex, endIndex)
    }

  override val editText: (block: androidx.compose.ui.text.input.TextEditingScope.() -> Unit) -> Unit =
    { _ -> }
}

@OptIn(ExperimentalComposeUiApi::class)
internal actual fun PlatformTextInputSessionScope.notifyImeSelectionChanged(editor: Editor) {
  // iOS: pull-based via request.value — no explicit notification needed
}
