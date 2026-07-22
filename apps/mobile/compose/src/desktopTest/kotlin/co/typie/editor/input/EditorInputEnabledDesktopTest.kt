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
import co.typie.editor.ffi.Key as FfiKey
import co.typie.editor.ffi.KeyEvent as FfiKeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.scroll.rememberEditorBringIntoViewRequests
import co.typie.editor.sync.createTestDocumentEditingSession
import co.typie.platform.Clipboard
import co.typie.platform.IncomingContentCandidates
import co.typie.platform.IncomingContentMode
import co.typie.platform.NoopClipboard
import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class EditorInputEnabledDesktopTest {
  @Test
  fun laterKeyWaitsForPasteReadQueuedFromTheSameInputCallback() = runComposeUiTest {
    val fake = FakeFfiEditor()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val session = createTestDocumentEditingSession(editor, scope)
    val pasteStarted = CompletableDeferred<Unit>()
    val finishPaste = CompletableDeferred<Unit>()
    val handler =
      object : EditorIncomingContentHandler {
        override suspend fun handleClipboard(
          session: co.typie.editor.DocumentEditingSession,
          clipboard: Clipboard,
          mode: IncomingContentMode,
        ): Boolean {
          pasteStarted.complete(Unit)
          finishPaste.await()
          return false
        }

        override suspend fun handleCandidates(
          session: co.typie.editor.DocumentEditingSession,
          candidates: IncomingContentCandidates,
          mode: IncomingContentMode,
        ): Boolean = false
      }

    try {
      setContent {
        val focusRequester = remember { FocusRequester() }
        val bringIntoViewRequests = rememberEditorBringIntoViewRequests()
        Box(
          Modifier.size(200.dp)
            .testTag(InputTag)
            .focusRequester(focusRequester)
            .editorInput(
              session = session,
              uiState = EditorUiState(),
              platform = Platform.Desktop,
              bringIntoViewRequests = bringIntoViewRequests,
              enabled = true,
              suppressSoftwareKeyboard = true,
              clipboard = NoopClipboard,
              incomingContentHandler = handler,
            )
            .focusable()
        )
        LaunchedEffect(Unit) { focusRequester.requestFocus() }
      }
      waitForIdle()

      onNodeWithTag(InputTag).performKeyInput {
        keyDown(Key.MetaLeft)
        keyDown(Key.V)
        keyUp(Key.V)
        keyUp(Key.MetaLeft)
        keyDown(Key.Backspace)
        keyUp(Key.Backspace)
      }
      waitUntil(timeoutMillis = 5_000) { pasteStarted.isCompleted }

      val backspace = Message.Key(FfiKeyEvent(FfiKey.Backspace))
      assertFalse(fake.enqueued.contains(backspace))

      finishPaste.complete(Unit)
      waitUntil(timeoutMillis = 5_000) { fake.enqueued.contains(backspace) }
    } finally {
      finishPaste.complete(Unit)
      session.stop()
      scope.cancel()
    }
  }

  @Test
  fun platformEnumDoesNotReplaceDesktopKeyEventRouting() = runComposeUiTest {
    val fake = FakeFfiEditor()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val recorder = EditorInputRecorder()
    editor.inputRecorder = recorder
    val session = createTestDocumentEditingSession(editor, scope)
    val pasteHandled = CompletableDeferred<Unit>()
    val handler =
      object : EditorIncomingContentHandler {
        override suspend fun handleClipboard(
          session: co.typie.editor.DocumentEditingSession,
          clipboard: Clipboard,
          mode: IncomingContentMode,
        ): Boolean {
          pasteHandled.complete(Unit)
          return true
        }

        override suspend fun handleCandidates(
          session: co.typie.editor.DocumentEditingSession,
          candidates: IncomingContentCandidates,
          mode: IncomingContentMode,
        ): Boolean = false
      }

    try {
      setContent {
        val focusRequester = remember { FocusRequester() }
        val bringIntoViewRequests = rememberEditorBringIntoViewRequests()
        Box(
          Modifier.size(200.dp)
            .testTag(InputTag)
            .focusRequester(focusRequester)
            .editorInput(
              session = session,
              uiState = EditorUiState(),
              platform = Platform.iOS,
              bringIntoViewRequests = bringIntoViewRequests,
              enabled = true,
              suppressSoftwareKeyboard = true,
              clipboard = NoopClipboard,
              incomingContentHandler = handler,
            )
            .focusable()
        )
        LaunchedEffect(Unit) { focusRequester.requestFocus() }
      }
      waitForIdle()

      onNodeWithTag(InputTag).performKeyInput {
        keyDown(Key.MetaLeft)
        keyDown(Key.V)
        keyUp(Key.V)
        keyUp(Key.MetaLeft)
      }
      waitUntil(timeoutMillis = 5_000) { pasteHandled.isCompleted }

      val hardwareKeys = recorder.snapshot().filterIsInstance<RecordedInputEntry.HardwareKey>()
      assertTrue(
        hardwareKeys.any { it.stage == "onPreKeyEvent" && it.matchedBinding && !it.consumed }
      )
      assertTrue(hardwareKeys.any { it.stage == "onKeyEvent" && it.matchedBinding && it.consumed })
      assertFalse(
        hardwareKeys.any { it.stage == "onPreKeyEvent" && it.matchedBinding && it.consumed }
      )
    } finally {
      session.stop()
      scope.cancel()
    }
  }

  @Test
  fun disabledEditorInputDoesNotDispatchHardwareKeys() = runComposeUiTest {
    val fake = FakeFfiEditor()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val session = createTestDocumentEditingSession(editor, scope)

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
                session = session,
                uiState = EditorUiState(),
                platform = Platform.Desktop,
                bringIntoViewRequests = bringIntoViewRequests,
                enabled = false,
                suppressSoftwareKeyboard = true,
                clipboard = NoopClipboard,
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
      session.stop()
      scope.cancel()
    }
  }

  private companion object {
    const val InputTag = "editor-input-disabled"
  }
}
