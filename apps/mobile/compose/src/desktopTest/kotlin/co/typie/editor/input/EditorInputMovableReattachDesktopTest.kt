package co.typie.editor.input

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.movableContentOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
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
import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Movement
import co.typie.editor.ffi.NavigationOp
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.scroll.rememberEditorBringIntoViewRequests
import co.typie.editor.sync.createTestDocumentEditingSession
import co.typie.platform.NoopClipboard
import co.typie.platform.Platform
import kotlin.test.Test
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class EditorInputMovableReattachDesktopTest {
  @Test
  fun `key bindings dispatch after movable content reattaches the input node`() = runComposeUiTest {
    val fake = FakeFfiEditor()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val session = createTestDocumentEditingSession(editor, scope)
    val focusRequester = FocusRequester()
    var slot by mutableStateOf(0)

    try {
      setContent {
        val bringIntoViewRequests = rememberEditorBringIntoViewRequests()
        val uiState = remember { EditorUiState() }
        val content = remember {
          movableContentOf {
            CompositionLocalProvider(
              LocalEditorBringIntoViewRequests provides bringIntoViewRequests
            ) {
              Box(
                Modifier.size(200.dp)
                  .testTag(InputTag)
                  .focusRequester(focusRequester)
                  .editorInput(
                    session = session,
                    uiState = uiState,
                    platform = Platform.Desktop,
                    bringIntoViewRequests = bringIntoViewRequests,
                    enabled = true,
                    suppressSoftwareKeyboard = true,
                    clipboard = NoopClipboard,
                  )
                  .focusable()
              )
            }
          }
        }
        if (slot == 0) {
          Box { content() }
        } else {
          Box { content() }
        }
      }
      waitForIdle()
      runOnIdle { focusRequester.requestFocus() }
      waitForIdle()

      onNodeWithTag(InputTag).performKeyInput {
        keyDown(Key.DirectionRight)
        keyUp(Key.DirectionRight)
      }
      waitUntil(timeoutMillis = 5_000) { fake.enqueued.isNotEmpty() }

      slot = 1
      waitForIdle()
      slot = 0
      waitForIdle()

      fake.enqueued.clear()
      runOnIdle { focusRequester.requestFocus() }
      waitForIdle()

      onNodeWithTag(InputTag).performKeyInput {
        keyDown(Key.DirectionRight)
        keyUp(Key.DirectionRight)
      }
      waitUntil(timeoutMillis = 5_000) {
        fake.enqueued.contains(
          Message.Navigation(NavigationOp.Move(Movement.Grapheme(Direction.Forward), false))
        )
      }
    } finally {
      session.stop()
      scope.cancel()
    }
  }

  private companion object {
    const val InputTag = "editor-input-movable-reattach"
  }
}
