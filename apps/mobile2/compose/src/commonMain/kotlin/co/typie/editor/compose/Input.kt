package co.typie.editor.compose

import androidx.compose.foundation.layout.size
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.input.TextFieldState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.ffi.Break
import co.typie.editor.ffi.CompositionIntent
import co.typie.editor.ffi.InsertionIntent
import co.typie.editor.ffi.Intent
import co.typie.editor.ffi.Key
import co.typie.editor.ffi.KeyEvent
import co.typie.editor.ffi.Message
import androidx.compose.ui.input.key.Key as ComposeKey

private const val SENTINEL = "\u200B"

@Composable
internal fun Input(
  editor: Editor,
  focusRequester: FocusRequester,
) {
  val textFieldState = remember { TextFieldState(SENTINEL) }

  LaunchedEffect(textFieldState) {
    var composing = false

    snapshotFlow {
      Triple(
        textFieldState.text.toString(),
        textFieldState.composition,
        textFieldState.composition?.let {
          textFieldState.text.subSequence(it.start, it.end).toString()
        },
      )
    }.collect { (text, composition, composingText) ->
      when {
        composition != null && composingText != null -> {
          editor.enqueue(
            Message.Intent(Intent.Composition(CompositionIntent.Update(composingText, null)))
          )
          composing = true
        }

        composing -> {
          editor.enqueue(Message.Intent(Intent.Composition(CompositionIntent.End)))
          composing = false
          textFieldState.resetToSentinel()
        }

        !text.contains(SENTINEL) -> {
          // Sentinel was deleted — software keyboard backspace
          editor.enqueue(Message.Key(KeyEvent(Key.Backspace)))
          textFieldState.resetToSentinel()
        }

        text.contains("\n") -> {
          editor.enqueue(
            Message.Intent(Intent.Insertion(InsertionIntent.Break(Break.Paragraph)))
          )
          textFieldState.resetToSentinel()
        }

        text.length > SENTINEL.length -> {
          val input = text.replace(SENTINEL, "")
          if (input.isNotEmpty()) {
            editor.enqueue(
              Message.Intent(Intent.Insertion(InsertionIntent.Text(input)))
            )
          }
          textFieldState.resetToSentinel()
        }
      }
    }
  }

  BasicTextField(
    state = textFieldState,
    modifier = Modifier
      .size(1.dp)
      .focusRequester(focusRequester)
      .onPreviewKeyEvent { event ->
        if (event.type != KeyEventType.KeyDown) return@onPreviewKeyEvent false
        if (textFieldState.composition != null) return@onPreviewKeyEvent false
        when (event.key) {
          ComposeKey.Backspace -> {
            editor.enqueue(Message.Key(KeyEvent(Key.Backspace)))
            true
          }
          ComposeKey.Enter -> {
            editor.enqueue(
              Message.Intent(Intent.Insertion(InsertionIntent.Break(Break.Paragraph)))
            )
            true
          }
          else -> false
        }
      },
  )
}

private fun TextFieldState.resetToSentinel() {
  edit { replace(0, length, SENTINEL) }
}
