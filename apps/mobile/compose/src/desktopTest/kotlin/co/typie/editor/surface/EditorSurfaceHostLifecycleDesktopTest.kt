package co.typie.editor.surface

import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.v2.runComposeUiTest
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.Size
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class EditorSurfaceHostLifecycleDesktopTest {
  @Test
  fun requiredOffscreenPageAttachesWithoutPageComposition() = runComposeUiTest {
    configureRenderBufferLibrary()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val fake =
      FakeFfiEditor(
        pageSizesProvider = {
          listOf(
            Size(width = 100f, height = 100f),
            Size(width = 100f, height = 100f),
            Size(width = 240f, height = 320f),
          )
        }
      )
    val editor = Editor(fake, scope, Dispatchers.Unconfined)
    try {
      fake.applySnapshot(editor)
      editor.requestSurfacePages(setOf(2))
      setContent { EditorSurfaceHost(editor = editor, scaleFactor = 1.0, onFailure = { throw it }) }

      waitUntil { fake.attachCalls.any { it.page == 2 } }

      assertEquals(
        FakeFfiEditor.SurfaceAttachCall(page = 2, width = 240.0, height = 320.0, scaleFactor = 1.0),
        fake.attachCalls.last(),
      )
    } finally {
      scope.cancel()
    }
  }

  @Test
  fun replacingRequirementsDetachesRemovedProducer() = runComposeUiTest {
    configureRenderBufferLibrary()
    val detached = mutableListOf<Int>()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val fake =
      FakeFfiEditor(
        pageSizesProvider = {
          listOf(Size(width = 100f, height = 100f), Size(width = 200f, height = 200f))
        },
        detachSurfaceProvider = detached::add,
      )
    val editor = Editor(fake, scope, Dispatchers.Unconfined)
    try {
      fake.applySnapshot(editor)
      editor.requestSurfacePages(setOf(0))
      setContent { EditorSurfaceHost(editor = editor, scaleFactor = 1.0, onFailure = { throw it }) }
      waitUntil { fake.attachCalls.any { it.page == 0 } }

      runOnUiThread { editor.requestSurfacePages(setOf(1)) }
      waitUntil { fake.attachCalls.any { it.page == 1 } && 0 in detached }

      assertEquals(listOf(0), detached)
    } finally {
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
