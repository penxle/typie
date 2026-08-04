@file:OptIn(androidx.compose.ui.ExperimentalComposeUiApi::class)

package co.typie.editor.input

import androidx.compose.ui.platform.PlatformTextInputMethodRequest
import androidx.compose.ui.platform.PlatformTextInputSessionScope
import androidx.compose.ui.text.input.CommitTextCommand
import androidx.compose.ui.text.input.EditCommand
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.scroll.EditorBringIntoViewRequests
import kotlin.coroutines.EmptyCoroutineContext
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.cancel

class EditorInputRequestIosTest {
  @Test
  fun nativeEditBlockDispatchesEditCommands() {
    val editorScope = CoroutineScope(EmptyCoroutineContext)
    val editor = Editor(FakeFfiEditor(), editorScope)
    val dispatched = mutableListOf<List<EditCommand>>()
    try {
      val request =
        kotlinx.coroutines.runBlocking {
          TestTextInputSessionScope.createEditorInputRequest(
            editor = editor,
            bringIntoViewRequests = EditorBringIntoViewRequests(),
            onEditCommand = dispatched::add,
            focusedRectInRoot = { null },
            textFieldRectInRoot = { null },
            textClippingRectInRoot = { null },
            suppressSoftwareKeyboard = false,
            isSessionCurrent = { true },
            onIncomingContent = { false },
          )
        }

      request.editText { commitText("typed", 1) }

      val expected: List<List<EditCommand>> = listOf(listOf(CommitTextCommand("typed", 1)))
      assertEquals(expected, dispatched)
    } finally {
      editorScope.cancel()
    }
  }
}

private object TestTextInputSessionScope : PlatformTextInputSessionScope {
  override val coroutineContext = EmptyCoroutineContext

  override suspend fun startInputMethod(request: PlatformTextInputMethodRequest): Nothing =
    error("not used")
}
