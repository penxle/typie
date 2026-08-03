package co.typie.screen.editor.editor.entry

import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.v2.runComposeUiTest
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.scroll.EditorBringIntoViewRequests
import java.io.File
import java.util.UUID
import kotlin.test.Test
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class EditorEntryStateSessionDesktopTest {
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
