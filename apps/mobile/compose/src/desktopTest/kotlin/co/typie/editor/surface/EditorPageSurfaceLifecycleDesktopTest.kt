package co.typie.editor.surface

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.EditorZoomController
import co.typie.editor.FakeFfiEditor
import co.typie.editor.LocalEditorZoomController
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.Size
import co.typie.editor.runtime.EditorRuntime
import co.typie.editor.runtime.LocalEditorRuntime
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class EditorPageSurfaceLifecycleDesktopTest {
  @Test
  fun offscreenPageAttachesWhileTheHostIsPreparingItsReplacementSurface() = runComposeUiTest {
    configureRenderBufferLibrary()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    var pageSizes = listOf(Size(width = 100f, height = 100f), Size(width = 100f, height = 100f))
    val fake =
      FakeFfiEditor(
        onTick = { listOf(EditorEvent.RenderInvalidated) },
        pageSizesProvider = { pageSizes },
      )
    val editor = Editor(fake, scope, Dispatchers.Unconfined)
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    val host = Any()

    try {
      fake.applySnapshot(editor)
      editor.activateVisualHost(host)
      var frameKey = 0L
      val stale = editor.attachSurface(1, 10L, 100.0, 100.0, 1.0) { frameKey = it.value }
      editor.deliverFrame(
        session = stale,
        bitmap = ImageBitmap(width = 100, height = 100),
        pixelSize = IntSize(width = 100, height = 100),
        editorRevision = editor.appliedRevision,
        frameKey = frameKey,
      )
      stale.detach()

      pageSizes = listOf(Size(width = 200f, height = 300f))
      fake.applySnapshot(editor)
      assertEquals(0, editor.preparingPage)
      fake.attachCalls.clear()

      setContent {
        CompositionLocalProvider(
          LocalDensity provides Density(1f),
          LocalEditorRuntime provides runtime,
          LocalEditorZoomController provides EditorZoomController(),
        ) {
          Box(Modifier.size(100.dp)) {
            EditorPageSurface(
              page = 0,
              width = 200f,
              height = 300f,
              publishedVersion = editor.publishedRevision ?: 0L,
              publishedFrame = editor.publishedBundle?.frames?.get(0),
              showChrome = false,
              modifier = Modifier.offset(y = 5_000.dp),
            )
          }
        }
      }

      waitUntil { fake.attachCalls.any { it.page == 0 } }

      assertEquals(0, editor.preparingPage)
      assertEquals(
        FakeFfiEditor.SurfaceAttachCall(page = 0, width = 200.0, height = 300.0, scaleFactor = 1.0),
        fake.attachCalls.last(),
      )
    } finally {
      runtime.clear()
      waitForIdle()
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
