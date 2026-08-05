package co.typie.editor.input

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performKeyInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.Break
import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.InsertionOp
import co.typie.editor.ffi.Key as FfiKey
import co.typie.editor.ffi.KeyEvent as FfiKeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.ModifierOp
import co.typie.editor.ffi.ModifierType
import co.typie.editor.ffi.Movement
import co.typie.editor.ffi.NavigationOp
import co.typie.editor.ffi.Size as EditorSize
import co.typie.editor.ffi.StateField
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.scroll.rememberEditorBringIntoViewRequests
import co.typie.editor.sync.createTestDocumentEditingSession
import co.typie.platform.Clipboard
import co.typie.platform.IncomingContentCandidates
import co.typie.platform.IncomingContentMode
import co.typie.platform.NoopClipboard
import co.typie.platform.Platform
import java.util.concurrent.ConcurrentLinkedQueue
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestCoroutineScheduler

@OptIn(ExperimentalTestApi::class)
class EditorInputEnabledDesktopTest {
  @Test
  fun later33msKeyRepeatsWaitForTheActiveVisualPublication() = runComposeUiTest {
    val scheduler = TestCoroutineScheduler()
    val dispatcher = StandardTestDispatcher(scheduler)
    var pageHeight = 100f
    val fake =
      FakeFfiEditor(
        onTick = {
          pageHeight += 10f
          listOf(
            EditorEvent.StateChanged(listOf(StateField.PageSizes)),
            EditorEvent.RenderInvalidated,
          )
        },
        pageSizesProvider = { listOf(EditorSize(width = 100f, height = pageHeight)) },
      )
    val scope = CoroutineScope(SupervisorJob() + dispatcher)
    val editor = Editor(fake, scope, dispatcher)
    fake.applySnapshot(editor)
    val session = createTestDocumentEditingSession(editor, scope)
    val readyFrameKeys = ConcurrentLinkedQueue<Long>()
    val host = Any()
    editor.activateVisualHost(host)
    val surface =
      editor.attachSurface(
        page = 0,
        handle = 1L,
        width = 100.0,
        height = pageHeight.toDouble(),
        scaleFactor = 1.0,
      ) { frameKey ->
        readyFrameKeys += frameKey.value
      }
    editor.requestSurfacePages(setOf(0))
    var bringIntoViewRequests: EditorBringIntoViewRequests? = null

    fun deliverFrame(revision: Long) {
      scheduler.runCurrent()
      val frameKey = checkNotNull(readyFrameKeys.poll())
      editor.deliverFrame(
        session = surface,
        bitmap = ImageBitmap(width = 100, height = 100),
        pixelSize = IntSize(width = 100, height = 100),
        editorRevision = revision,
        frameKey = frameKey,
      )
      scheduler.runCurrent()
      editor.publishIfReady(setOf(0))?.let { bundle ->
        check(editor.acceptPublication(bundle))
        editor.completePresentation(bundle)
      }
      bringIntoViewRequests?.activateForVersion(revision)?.let { request ->
        check(bringIntoViewRequests?.markPresented(revision, request) == true)
      }
      scheduler.runCurrent()
    }

    try {
      deliverFrame(editor.appliedRevision)
      assertEquals(editor.appliedRevision, editor.publishedRevision)

      setContent {
        val focusRequester = remember { FocusRequester() }
        val currentBringIntoViewRequests = rememberEditorBringIntoViewRequests()
        bringIntoViewRequests = currentBringIntoViewRequests
        Box(
          Modifier.size(200.dp)
            .testTag(InputTag)
            .focusRequester(focusRequester)
            .editorInput(
              session = session,
              uiState = EditorUiState(),
              platform = Platform.Desktop,
              bringIntoViewRequests = currentBringIntoViewRequests,
              enabled = true,
              suppressSoftwareKeyboard = true,
              clipboard = NoopClipboard,
            )
            .focusable()
        )
        LaunchedEffect(Unit) { focusRequester.requestFocus() }
      }
      waitForIdle()

      onNodeWithTag(InputTag).performKeyInput {
        keyDown(Key.Enter)
        keyUp(Key.Enter)
        advanceEventTime(RealisticRepeatIntervalMillis)
      }
      waitUntil(timeoutMillis = 5_000) {
        scheduler.runCurrent()
        fake.enqueuedRequests.size == 1
      }
      val firstRevision = editor.appliedRevision
      assertTrue(firstRevision > requireNotNull(editor.publishedRevision))

      repeat(3) {
        onNodeWithTag(InputTag).performKeyInput {
          keyDown(Key.Enter)
          keyUp(Key.Enter)
          advanceEventTime(RealisticRepeatIntervalMillis)
        }
      }
      repeat(5) {
        waitForIdle()
        scheduler.runCurrent()
      }

      assertEquals(
        firstRevision,
        editor.appliedRevision,
        "33ms key repeats overtook an extent-changing frame that had not been published",
      )
      assertEquals(1, fake.enqueuedRequests.size)

      deliverFrame(firstRevision)
      waitUntil(timeoutMillis = 5_000) {
        scheduler.runCurrent()
        fake.enqueuedRequests.size == 2
      }
      assertEquals(3, fake.enqueuedRequests.last().messages.size)

      deliverFrame(editor.appliedRevision)
      assertEquals(editor.appliedRevision, editor.publishedRevision)
    } finally {
      editor.deactivateVisualHost(host)
      session.stop()
      scope.cancel()
    }
  }

  @Test
  fun navigationAndShortcutsCommitCompositionBeforeDispatch() = runComposeUiTest {
    val composingIme =
      Ime(text = "한", windowStart = 0, selection = ImeRange(1, 1), composing = ImeRange(0, 1))
    val fake = FakeFfiEditor(imeProvider = { _, _ -> composingIme })
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val session = createTestDocumentEditingSession(editor, scope)

    try {
      editor.setImeSessionActive(true)
      editor.refreshImeSnapshot()

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
            )
            .focusable()
        )
        LaunchedEffect(Unit) { focusRequester.requestFocus() }
      }
      waitForIdle()
      fake.enqueued.clear()

      onNodeWithTag(InputTag).performKeyInput {
        keyDown(Key.MetaLeft)
        keyDown(Key.Enter)
        keyUp(Key.Enter)
        keyUp(Key.MetaLeft)
      }
      waitUntil(timeoutMillis = 5_000) {
        fake.enqueued.any { it == Message.Insertion(InsertionOp.Break(Break.Page)) }
      }

      assertEquals(
        listOf(
          Message.TextInput(listOf(FlatImeOp.CommitAsIs)),
          Message.Insertion(InsertionOp.Break(Break.Page)),
        ),
        fake.enqueued.filter {
          it is Message.TextInput || it == Message.Insertion(InsertionOp.Break(Break.Page))
        },
      )

      editor.refreshImeSnapshot()
      fake.enqueued.clear()
      onNodeWithTag(InputTag).performKeyInput {
        keyDown(Key.DirectionLeft)
        keyUp(Key.DirectionLeft)
      }
      val moveLeft =
        Message.Navigation(NavigationOp.Move(Movement.Grapheme(Direction.Backward), extend = false))
      waitUntil(timeoutMillis = 5_000) { fake.enqueued.any { it == moveLeft } }
      assertEquals(
        listOf(Message.TextInput(listOf(FlatImeOp.CommitAsIs)), moveLeft),
        fake.enqueued.filter { it is Message.TextInput || it == moveLeft },
      )

      editor.refreshImeSnapshot()
      fake.enqueued.clear()
      onNodeWithTag(InputTag).performKeyInput {
        keyDown(Key.MetaLeft)
        keyDown(Key.B)
        keyUp(Key.B)
        keyUp(Key.MetaLeft)
      }
      val toggleBold = Message.Modifier(ModifierOp.Toggle(ModifierType.Bold))
      waitUntil(timeoutMillis = 5_000) { fake.enqueued.any { it == toggleBold } }
      assertEquals(
        listOf(Message.TextInput(listOf(FlatImeOp.CommitAsIs)), toggleBold),
        fake.enqueued.filter { it is Message.TextInput || it == toggleBold },
      )
    } finally {
      session.stop()
      scope.cancel()
    }
  }

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
    const val RealisticRepeatIntervalMillis = 33L
  }
}
