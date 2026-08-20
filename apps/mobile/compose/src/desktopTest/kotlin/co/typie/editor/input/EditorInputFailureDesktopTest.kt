package co.typie.editor.input

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.InterceptPlatformTextInput
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.rememberEditorBringIntoViewRequests
import co.typie.editor.sync.createTestDocumentEditingSession
import co.typie.platform.NoopClipboard
import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class EditorInputFailureDesktopTest {
  @Test
  fun `ime snapshot failure stops the actual platform input session`() = runComposeUiTest {
    val failure = IllegalStateException("ime snapshot failed")
    val reported = CompletableDeferred<Throwable>()
    val fake = FakeFfiEditor(imeProvider = { _, _ -> throw failure })
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope, onError = { _, error -> reported.complete(error) })
    val session = createTestDocumentEditingSession(editor, scope)
    val focusRequester = FocusRequester()
    var inputRequests = 0

    try {
      setContent {
        InterceptPlatformTextInput(
          interceptor = { request, nextHandler ->
            inputRequests += 1
            nextHandler.startInputMethod(request)
          }
        ) {
          val bringIntoViewRequests = rememberEditorBringIntoViewRequests()
          Box(
            Modifier.size(200.dp)
              .focusRequester(focusRequester)
              .editorInput(
                session = session,
                uiState = EditorUiState(),
                platform = Platform.Desktop,
                bringIntoViewRequests = bringIntoViewRequests,
                enabled = true,
                suppressSoftwareKeyboard = false,
                clipboard = NoopClipboard,
              )
              .focusable()
          )
        }
      }
      waitForIdle()
      runOnIdle { assertTrue(focusRequester.requestFocus()) }
      waitUntil(timeoutMillis = 5_000) { reported.isCompleted }

      assertSame(failure, reported.await())
      runOnIdle {
        assertTrue(editor.terminal)
        assertEquals(0, inputRequests)
      }
    } finally {
      session.stop()
      scope.cancel()
    }
  }
}
