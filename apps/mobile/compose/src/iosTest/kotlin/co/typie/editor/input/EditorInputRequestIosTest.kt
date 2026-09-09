@file:OptIn(androidx.compose.ui.ExperimentalComposeUiApi::class)

package co.typie.editor.input

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.platform.PlatformTextInputMethodRequest
import androidx.compose.ui.platform.PlatformTextInputSessionScope
import androidx.compose.ui.platform.UIKitTextInputMethodRequest
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.CommitTextCommand
import androidx.compose.ui.text.input.EditCommand
import androidx.compose.ui.text.input.SetSelectionCommand
import androidx.compose.ui.text.input.TextEditingScope
import androidx.compose.ui.text.input.TextFieldValue
import co.typie.editor.Editor
import co.typie.editor.EditorViewportTransform
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.scroll.EditorBringIntoViewRequests
import kotlin.coroutines.EmptyCoroutineContext
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runTest

class EditorInputRequestIosTest {
  @Test
  fun platformBindingPreservesRequestedTextRangeGeometry() = runTest {
    val rectangle = Rect(120f, 240f, 180f, 272f)
    val origin = Offset(40f, -120f)
    runNativeEdit(
      before = ime(text = "にほんご", selection = 4),
      firstRectForRangeInRoot = { range ->
        assertEquals(TextRange(1, 3), range)
        rectangle
      },
      unclippedTextOffsetInRoot = { origin },
      inspect = { request ->
        val wrapped =
          EditorPlatformInputBridge()
            .bindInputSession(
              TestTextInputSessionScope(coroutineContext),
              request,
              cursor = { null },
              viewportTransform = { EditorViewportTransform(emptyMap(), emptyList(), 1f) },
              dispatch = {},
            )
        assertEquals(origin, wrapped.unclippedTextOffsetInRoot())
        assertEquals(
          rectangle,
          (wrapped as UIKitTextInputMethodRequest).firstRectForRangeInRoot?.invoke(TextRange(1, 3)),
        )
      },
    ) {}
  }

  @Test
  fun requestExposesTheWholeImeWindowIncludingParagraphBoundaries() = runTest {
    val before = ime(text = "\u2028문단1\u2029\u2028ㅎ\u2029", selection = 7)

    val result = runNativeEdit(before) {}

    assertEquals(before.toTextFieldValue(), result.value)
  }

  @Test
  fun paragraphStartRewriteKeepsFullImeWindowCoordinates() = runTest {
    val result =
      runNativeEdit(before = ime(text = "\u2028문단1\u2029\u2028ㅎ\u2029", selection = 7)) {
        setSelection(0, 1)
        commitText("", 0)
      }

    assertEquals(
      listOf(listOf(SetSelectionCommand(0, 1), CommitTextCommand("", 0))),
      result.dispatched,
    )
  }

  @Test
  fun nativeEditBlockDispatchesEditCommandsInOrder() = runTest {
    val result =
      runNativeEdit(before = ime(text = "x", selection = 1)) {
        setSelection(0, 0)
        commitText("typed", 1)
      }

    val expected: List<List<EditCommand>> =
      listOf(listOf(SetSelectionCommand(0, 0), CommitTextCommand("typed", 1)))
    assertEquals(expected, result.dispatched)
  }

  private data class NativeEditResult(
    val value: TextFieldValue,
    val dispatched: List<List<EditCommand>>,
  )

  private suspend fun TestScope.runNativeEdit(
    before: Ime,
    firstRectForRangeInRoot: (TextRange) -> Rect? = { null },
    unclippedTextOffsetInRoot: () -> Offset? = { null },
    inspect: (PlatformTextInputMethodRequest) -> Unit = {},
    block: TextEditingScope.() -> Unit,
  ): NativeEditResult {
    val editorScope = CoroutineScope(EmptyCoroutineContext)
    val fake = FakeFfiEditor(imeProvider = { _, _ -> before })
    val editor = Editor(fake, editorScope)
    val dispatched = mutableListOf<List<EditCommand>>()
    try {
      editor.setImeSessionActive(true)
      fake.applySnapshot(editor)
      val request =
        TestTextInputSessionScope(coroutineContext)
          .createEditorInputRequest(
            editor = editor,
            bringIntoViewRequests = EditorBringIntoViewRequests(),
            onEditCommand = dispatched::add,
            focusedRectInRoot = { null },
            firstRectForRangeInRoot = firstRectForRangeInRoot,
            unclippedTextOffsetInRoot = unclippedTextOffsetInRoot,
            textFieldRectInRoot = { null },
            textClippingRectInRoot = { null },
            suppressSoftwareKeyboard = false,
            isSessionCurrent = { true },
            onIncomingContent = { false },
          )

      inspect(request)
      val value = request.value()
      request.editText(block)
      return NativeEditResult(value = value, dispatched = dispatched)
    } finally {
      editorScope.cancel()
    }
  }

  private fun ime(text: String, selection: Int): Ime =
    Ime(
      text = text,
      windowStart = 0,
      selection = ImeRange(start = selection, end = selection),
      composing = null,
    )
}

private class TestTextInputSessionScope(
  override val coroutineContext: kotlin.coroutines.CoroutineContext = EmptyCoroutineContext
) : PlatformTextInputSessionScope {
  override suspend fun startInputMethod(request: PlatformTextInputMethodRequest): Nothing =
    error("not used")
}
