package co.typie.editor.input

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
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
import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.InsertionOp
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
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class EditorInputCompositionKeyDesktopTest {
  @Test
  fun `android navigation key during composition commits preedit then moves`() {
    runCompositionKeyTest(
      platform = Platform.Android,
      key = Key.DirectionRight,
      expectEnqueued =
        listOf(
          Message.TextInput(listOf(FlatImeOp.CommitAsIs)),
          Message.Navigation(NavigationOp.Move(Movement.Grapheme(Direction.Forward), false)),
        ),
    )
  }

  @Test
  fun `android backspace during composition stays blocked`() {
    runCompositionKeyTest(
      platform = Platform.Android,
      key = Key.Backspace,
      expectEnqueued = null,
    )
  }

  @Test
  fun `desktop navigation key during composition stays blocked`() {
    runCompositionKeyTest(
      platform = Platform.Desktop,
      key = Key.DirectionRight,
      expectEnqueued = null,
    )
  }

  @Test
  fun `android character key during composition drops raw-key fallback`() {
    runCompositionKeyTest(platform = Platform.Android, key = Key.A, expectEnqueued = null)
  }

  @Test
  fun `android character key without composition inserts through raw-key fallback`() =
    runComposeUiTest {
      val fake = FakeFfiEditor()
      val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
      val editor = Editor(fake, scope)
      val session = createTestDocumentEditingSession(editor, scope)

      try {
        setContent {
          val focusRequester = remember { FocusRequester() }
          val bringIntoViewRequests = rememberEditorBringIntoViewRequests()
          CompositionLocalProvider(
            LocalEditorBringIntoViewRequests provides bringIntoViewRequests
          ) {
            Box(
              Modifier.size(200.dp)
                .testTag(InputTag)
                .focusRequester(focusRequester)
                .editorInput(
                  session = session,
                  uiState = EditorUiState(),
                  platform = Platform.Android,
                  bringIntoViewRequests = bringIntoViewRequests,
                  enabled = true,
                  suppressSoftwareKeyboard = true,
                  clipboard = NoopClipboard,
                )
                .focusable()
            )
            LaunchedEffect(Unit) { focusRequester.requestFocus() }
          }
        }
        waitForIdle()
        fake.enqueued.clear()

        onNodeWithTag(InputTag).performKeyInput {
          keyDown(Key.A)
          keyUp(Key.A)
        }
        waitForIdle()

        waitUntil(timeoutMillis = 5_000) { fake.enqueued.isNotEmpty() }
        assertEquals(
          listOf(Message.Insertion(InsertionOp.Text("a"))),
          fake.enqueued.toList(),
        )
      } finally {
        session.stop()
        scope.cancel()
      }
    }

  private fun runCompositionKeyTest(
    platform: Platform,
    key: Key,
    expectEnqueued: List<Message>?,
  ) = runComposeUiTest {
    val fake =
      FakeFfiEditor(
        imeProvider = { _, _ ->
          Ime(
            text = "하",
            windowStart = 0,
            selection = ImeRange(1, 1),
            composing = ImeRange(0, 1),
          )
        }
      )
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val session = createTestDocumentEditingSession(editor, scope)

    try {
      setContent {
        val focusRequester = remember { FocusRequester() }
        val bringIntoViewRequests = rememberEditorBringIntoViewRequests()
        CompositionLocalProvider(LocalEditorBringIntoViewRequests provides bringIntoViewRequests) {
          Box(
            Modifier.size(200.dp)
              .testTag(InputTag)
              .focusRequester(focusRequester)
              .editorInput(
                session = session,
                uiState = EditorUiState(),
                platform = platform,
                bringIntoViewRequests = bringIntoViewRequests,
                enabled = true,
                suppressSoftwareKeyboard = true,
                clipboard = NoopClipboard,
              )
              .focusable()
          )
          LaunchedEffect(Unit) { focusRequester.requestFocus() }
        }
      }
      waitForIdle()

      editor.sync { enqueue(Message.TextInput(emptyList())) }
      waitUntil(timeoutMillis = 5_000) { editor.ime?.composing != null }
      fake.enqueued.clear()

      onNodeWithTag(InputTag).performKeyInput {
        keyDown(key)
        keyUp(key)
      }
      waitForIdle()

      if (expectEnqueued == null) {
        Thread.sleep(300)
        waitForIdle()
        assertTrue(fake.enqueued.isEmpty())
      } else {
        waitUntil(timeoutMillis = 5_000) { fake.enqueued.size >= expectEnqueued.size }
        assertEquals(expectEnqueued, fake.enqueued.toList())
      }
    } finally {
      session.stop()
      scope.cancel()
    }
  }

  private companion object {
    const val InputTag = "editor-input-composition-key"
  }
}
