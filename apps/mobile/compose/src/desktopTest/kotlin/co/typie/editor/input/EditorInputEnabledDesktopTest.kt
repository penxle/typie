package co.typie.editor.input

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
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.scroll.rememberEditorBringIntoViewRequests
import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class EditorInputEnabledDesktopTest {
  @Test
  fun disabledEditorInputDoesNotDispatchHardwareKeys() = runComposeUiTest {
    val fake = FakeFfiEditor()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)

    try {
      setContent {
        val focusRequester = remember { FocusRequester() }
        val bringIntoViewRequests = rememberEditorBringIntoViewRequests()
        androidx.compose.runtime.CompositionLocalProvider(
          LocalEditorBringIntoViewRequests provides bringIntoViewRequests
        ) {
          Box(
            Modifier.size(200.dp)
              .testTag(InputTag)
              .focusRequester(focusRequester)
              .editorInput(
                editor = editor,
                uiState = EditorUiState(),
                platform = Platform.Desktop,
                bringIntoViewRequests = bringIntoViewRequests,
                enabled = false,
                suppressSoftwareKeyboard = true,
              )
              .focusable()
          )
          LaunchedEffect(Unit) { focusRequester.requestFocus() }
        }
      }
      waitForIdle()

      onNodeWithTag(InputTag).performKeyInput {
        keyDown(Key.A)
        keyUp(Key.A)
      }
      waitForIdle()

      assertTrue(fake.enqueued.isEmpty())
    } finally {
      scope.cancel()
    }
  }

  private companion object {
    const val InputTag = "editor-input-disabled"
  }
}
