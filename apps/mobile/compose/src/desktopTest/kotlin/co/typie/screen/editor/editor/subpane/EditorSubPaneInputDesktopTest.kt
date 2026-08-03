package co.typie.screen.editor.editor.subpane

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performKeyInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.Key as FfiKey
import co.typie.editor.ffi.KeyEvent as FfiKeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.input.editorInput
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.rememberEditorBringIntoViewRequests
import co.typie.editor.sync.createTestDocumentEditingSession
import co.typie.platform.NoopClipboard
import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class EditorSubPaneInputDesktopTest {
  @Test
  fun commentsSubPaneDoesNotBlockFocusedEditorHardwareInput() = runComposeUiTest {
    val fake = FakeFfiEditor()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val session = createTestDocumentEditingSession(editor, scope)
    val subPaneState = EditorSubPaneState().apply { open(EditorSubPane.Comments) }

    try {
      setContent {
        val focusRequester = remember { FocusRequester() }
        val bringIntoViewRequests = rememberEditorBringIntoViewRequests()
        Box(
          Modifier.size(200.dp)
            .testTag(EditorInputTag)
            .focusRequester(focusRequester)
            .editorInput(
              session = session,
              uiState = EditorUiState(),
              platform = Platform.Desktop,
              bringIntoViewRequests = bringIntoViewRequests,
              enabled = !subPaneState.editorInputBlocked,
              suppressSoftwareKeyboard = true,
              clipboard = NoopClipboard,
            )
            .focusable()
        )
        LaunchedEffect(Unit) { focusRequester.requestFocus() }
      }
      waitForIdle()

      onNodeWithTag(EditorInputTag).performKeyInput {
        keyDown(Key.Backspace)
        keyUp(Key.Backspace)
      }

      val backspace = Message.Key(FfiKeyEvent(FfiKey.Backspace))
      waitUntil(timeoutMillis = 5_000) { fake.enqueued.contains(backspace) }
      assertEquals(EditorSubPane.Comments, subPaneState.active)
    } finally {
      session.stop()
      scope.cancel()
    }
  }

  private companion object {
    const val EditorInputTag = "editor-input-with-comments-subpane"
  }
}
