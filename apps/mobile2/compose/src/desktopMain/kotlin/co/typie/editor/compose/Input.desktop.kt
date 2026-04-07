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
import co.typie.editor.ffi.CompositionIntent
import co.typie.editor.ffi.DeletionIntent
import co.typie.editor.ffi.Intent
import co.typie.editor.ffi.Key
import co.typie.editor.ffi.KeyEvent
import co.typie.editor.ffi.Message

@OptIn(ExperimentalComposeUiApi::class)
internal actual suspend fun PlatformTextInputSessionScope.createEditorInputRequest(
  editor: Editor,
): PlatformTextInputMethodRequest {
  return object : PlatformTextInputMethodRequest {
    override val value: () -> TextFieldValue = {
      val ctx = editor.inputContext(Int.MAX_VALUE, Int.MAX_VALUE)
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

    // Returning Rect.Zero until page→root coordinate translation is wired here.
    // editor.cursor is a CursorRect whose `rect` is page-local; CMP requires
    // editor-root coordinates so the desktop IME (macOS NSTextInputClient /
    // Windows IMM / X11 XIM) can position candidate windows under the cursor.
    // pageOffsets lives in View.kt and is not reachable from this session scope
    // today — Rect.Zero makes the platform fall back to a default anchor.
    override val focusedRectInRoot: () -> Rect? = { Rect.Zero }

    override val textLayoutResult: () -> TextLayoutResult? = { null }

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
      { block ->
        val scope = object : TextEditingScope {
          override fun commitText(text: CharSequence, newCursorPosition: Int) {
            if (text.toString() == "\n") {
              editor.enqueue(Message.Key(KeyEvent(Key.Enter)))
            } else {
              editor.enqueue(Message.Intent(Intent.Composition(CompositionIntent.Commit(text.toString()))))
            }
          }

          override fun setComposingText(text: CharSequence, newCursorPosition: Int) {
            editor.enqueue(
              Message.Intent(
                Intent.Composition(CompositionIntent.Update(text.toString(), null))
              )
            )
          }

          override fun finishComposingText() {
            editor.enqueue(Message.Intent(Intent.Composition(CompositionIntent.CommitAsIs)))
          }

          override fun deleteSurroundingTextInCodePoints(
            lengthBeforeCursor: Int,
            lengthAfterCursor: Int
          ) {
            editor.enqueue(
              Message.Intent(
                Intent.Deletion(
                  DeletionIntent.SurroundingCodePoints(lengthBeforeCursor, lengthAfterCursor)
                )
              )
            )
          }
        }

        editor.batch { scope.block() }
      }
  }
}

@OptIn(ExperimentalComposeUiApi::class)
internal actual fun PlatformTextInputSessionScope.notifyImeSelectionChanged(editor: Editor) {
  // Desktop Skiko: pull-based via request.value — no explicit notification needed.
}
