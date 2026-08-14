package co.typie.screen.editor.editor.entry

import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.v2.runComposeUiTest
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.StableSelection
import co.typie.editor.ffi.ViewOp
import co.typie.editor.scroll.EditorBringIntoViewRequests
import java.io.File
import java.util.UUID
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class EditorEntryStateSessionDesktopTest {
  @Test
  fun entryIsNotReadyUntilItsStoredBodySelectionRevealIsPresented() = runComposeUiTest {
    configureRenderBufferLibrary()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val fake = FakeFfiEditor()
    val editor = Editor(fake, scope, Dispatchers.Unconfined)
    val documentId = "entry-restore-test-${UUID.randomUUID()}"
    val requests = EditorBringIntoViewRequests { editor.requestPublication() }
    val saved =
      StableSelection(
        version = 2,
        anchor = FakeFfiEditor.EmptyStablePosition,
        head = FakeFfiEditor.EmptyStablePosition,
      )
    EditorEntryStateStore()
      .save(
        documentId,
        StoredEditorEntryState(
          target = EditorEntryTarget.Body,
          bodySelection = saved,
          updatedAt = 0,
        ),
      )
    fake.applySnapshot(editor)
    var entry: EditorEntryStateSession? = null

    try {
      setContent {
        entry =
          rememberEditorEntryStateSession(
            documentId = documentId,
            editor = editor,
            editorFocused = false,
            bringIntoViewRequests = requests,
          )
      }
      waitUntil { requests.activateForVersion(editor.appliedRevision) != null }

      assertFalse(requireNotNull(entry).presentationReady)
      assertEquals(
        listOf(
          Message.Selection(SelectionOp.SetFrozen(selection = saved)),
          Message.View(ViewOp.ExpandFoldsForSelection),
        ),
        fake.enqueued,
      )

      runOnIdle {
        val request = requireNotNull(requests.activateForVersion(editor.appliedRevision))
        assertTrue(requests.markPresented(editor.appliedRevision, request))
      }
      waitUntil { requireNotNull(entry).presentationReady }
    } finally {
      editor.dispose()
      scope.cancel()
    }
  }

  @Test
  fun savesAppliedSelectionBeforeItIsPublished() = runComposeUiTest {
    configureRenderBufferLibrary()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val fake = FakeFfiEditor()
    val editor = Editor(fake, scope, Dispatchers.Unconfined)
    val documentId = "entry-state-test-${UUID.randomUUID()}"

    try {
      fake.applySnapshot(editor)
      assertNotNull(editor.appliedState.selection)
      assertNull(editor.publishedState.selection)

      setContent {
        rememberEditorEntryStateSession(
          documentId = documentId,
          editor = editor,
          editorFocused = true,
          bringIntoViewRequests = EditorBringIntoViewRequests(),
        )
      }
      waitForIdle()

      val saved = EditorEntryStateStore().load(documentId)
      assertNotNull(saved?.bodySelection)
    } finally {
      editor.dispose()
      scope.cancel()
    }
  }

  @Test
  fun savesSelectionChangedWhileUnfocusedWhenEditorBecomesFocused() = runComposeUiTest {
    configureRenderBufferLibrary()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val fake = FakeFfiEditor()
    val editor = Editor(fake, scope, Dispatchers.Unconfined)
    val documentId = "entry-reading-selection-test-${UUID.randomUUID()}"
    val editorFocused = mutableStateOf(false)

    try {
      fake.applySnapshot(editor)
      setContent {
        rememberEditorEntryStateSession(
          documentId = documentId,
          editor = editor,
          editorFocused = editorFocused.value,
          bringIntoViewRequests = EditorBringIntoViewRequests(),
        )
      }
      waitForIdle()

      runOnIdle {
        val position = Position(node = "text", offset = 1, affinity = Affinity.Downstream)
        fake.selectionProvider = { Selection(anchor = position, head = position) }
        fake.applySnapshot(editor)
      }
      waitForIdle()
      assertNull(EditorEntryStateStore().load(documentId))

      runOnIdle { editorFocused.value = true }
      waitForIdle()

      val saved = EditorEntryStateStore().load(documentId)
      assertNotNull(saved?.bodySelection)
    } finally {
      editor.dispose()
      scope.cancel()
    }
  }

  @Test
  fun doesNotSaveUnchangedSelectionWhenEditorBecomesFocused() = runComposeUiTest {
    configureRenderBufferLibrary()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val fake = FakeFfiEditor()
    val editor = Editor(fake, scope, Dispatchers.Unconfined)
    val documentId = "entry-unchanged-selection-test-${UUID.randomUUID()}"
    val editorFocused = mutableStateOf(false)

    try {
      fake.applySnapshot(editor)
      setContent {
        rememberEditorEntryStateSession(
          documentId = documentId,
          editor = editor,
          editorFocused = editorFocused.value,
          bringIntoViewRequests = EditorBringIntoViewRequests(),
        )
      }
      waitForIdle()

      runOnIdle { editorFocused.value = true }
      waitForIdle()

      assertNull(EditorEntryStateStore().load(documentId))
    } finally {
      editor.dispose()
      scope.cancel()
    }
  }
}

private fun configureRenderBufferLibrary() {
  if (System.getProperty("jna.library.path") != null) return

  val repository =
    generateSequence(File(System.getProperty("user.dir"))) { it.parentFile }
      .firstOrNull { File(it, "Cargo.toml").isFile } ?: error("Typie repository root not found")
  val host =
    when (System.getProperty("os.arch")) {
      "aarch64" -> "aarch64-apple-darwin"
      "x86_64" -> "x86_64-apple-darwin"
      else -> error("Unsupported desktop test architecture: ${System.getProperty("os.arch")}")
    }
  val directory = File(repository, "target/$host/release-uniffi")
  check(File(directory, "libeditor_ffi.dylib").isFile) {
    "Desktop editor FFI is not built; run `just -f crates/editor-ffi/justfile desktop`"
  }
  System.setProperty("jna.library.path", directory.absolutePath)
}
