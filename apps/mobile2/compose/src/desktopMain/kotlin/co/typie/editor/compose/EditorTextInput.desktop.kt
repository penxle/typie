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

@OptIn(ExperimentalComposeUiApi::class)
internal actual suspend fun PlatformTextInputSessionScope.createEditorTextInputRequest(
  editor: Editor,
): PlatformTextInputMethodRequest = object : PlatformTextInputMethodRequest {
  override val value: () -> TextFieldValue = {
    val ctx = editor.inputContext(Int.MAX_VALUE, Int.MAX_VALUE)
    val text = ctx.textBeforeCursor + ctx.selectedText + ctx.textAfterCursor
    val selStart = ctx.textBeforeCursor.length
    val selEnd = selStart + ctx.selectedText.length
    TextFieldValue(
      text = text,
      selection = TextRange(selStart, selEnd),
      composition = ctx.composingRange?.let { TextRange(it.start.toInt(), it.end.toInt()) },
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
    EditorEditCommandHandler.handle(editor, commands)
  }

  override val onImeAction: ((ImeAction) -> Unit)? = null

  // Returning null until page→root coordinate translation is wired here.
  // editor.cursor is a PageRect whose `rect` is page-local; CMP requires
  // editor-root coordinates so the desktop IME (macOS NSTextInputClient /
  // Windows IMM / X11 XIM) can position candidate windows under the cursor.
  // pageOffsets lives in View.kt and is not reachable from this session scope
  // today — returning null makes the platform fall back to a default anchor.
  override val focusedRectInRoot: () -> Rect? = { null }

  override val textLayoutResult: () -> androidx.compose.ui.text.TextLayoutResult? = { null }

  override val textFieldRectInRoot: () -> Rect? = { null }

  override val textClippingRectInRoot: () -> Rect? = { null }

  // Empty stub: desktop Skiko input machinery may read `state` differently
  // from iOS. If an assumption is violated, the error messages surface the
  // stub behavior so we can diagnose and wire up a proper implementation.
  override val state: androidx.compose.ui.text.input.TextEditorState =
    object : androidx.compose.ui.text.input.TextEditorState {
      override val selection: TextRange get() = TextRange.Zero
      override val composition: TextRange? get() = null
      override val length: Int get() = 0
      override fun get(index: Int): Char =
        error("EditorTextInput state is a stub; desktop editor uses pull-based value() queries")
      override fun subSequence(startIndex: Int, endIndex: Int): CharSequence =
        error("EditorTextInput state is a stub; desktop editor uses pull-based value() queries")
    }

  override val editText: (block: androidx.compose.ui.text.input.TextEditingScope.() -> Unit) -> Unit = { _ -> }
}

@OptIn(ExperimentalComposeUiApi::class)
internal actual fun PlatformTextInputSessionScope.notifyImeSelectionChanged(editor: Editor) {
  // Desktop Skiko: pull-based via request.value — no explicit notification needed.
}
