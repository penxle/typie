package co.typie.editor.preview

import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.EditorRootId
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Modifier as EditorModifier
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.PlainRootNode
import co.typie.editor.runtime.EditorRuntime
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class EditorPreviewDesktopTest {
  @Test
  fun graphPreviewRoutesInitializationFailureToItsRuntime() = runComposeUiTest {
    configureRenderBufferLibrary()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val runtime = EditorRuntime(scope)
    val liveEditor = Editor(FakeFfiEditor(), scope, Dispatchers.Unconfined)
    val liveRuntime = EditorRuntime(scope).apply { attach(liveEditor) }

    try {
      setContent {
        CompositionLocalProvider(
          LocalDensity provides Density(1f),
          LocalThemeMode provides ResolvedThemeMode.Light,
        ) {
          EditorPreview(
            layoutMode = A4Layout,
            runtime = runtime,
            modifier = Modifier.size(100.dp),
            shape = RoundedCornerShape(0.dp),
            source = EditorPreviewSource.Graph(byteArrayOf(0)),
          )
        }
      }

      waitUntil(timeoutMillis = 10_000) { runtime.failure != null }

      runOnIdle {
        assertNotNull(runtime.failure)
        assertNull(runtime.editor)
        assertNull(liveRuntime.failure)
        assertSame(liveEditor, liveRuntime.editor)
      }

      val firstFailure = runtime.failure
      runOnIdle { runtime.clearFailure() }
      waitUntil(timeoutMillis = 10_000) {
        runtime.failure != null && runtime.failure !== firstFailure
      }
    } finally {
      runtime.clear()
      runtime.clearFailure()
      liveRuntime.clear()
      scope.cancel()
    }
  }

  @Test
  fun pageSizeAndBodySizeChangesPublishCoherentPreviewFrames() = runComposeUiTest {
    configureRenderBufferLibrary()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val runtime = EditorRuntime(scope)
    var layout by mutableStateOf<LayoutMode>(A5Layout)
    var modifiers by mutableStateOf(listOf<EditorModifier>(EditorModifier.FontSize(1200)))

    try {
      setContent {
        CompositionLocalProvider(
          LocalDensity provides Density(1f),
          LocalThemeMode provides ResolvedThemeMode.Light,
        ) {
          EditorPreview(
            layoutMode = layout,
            runtime = runtime,
            modifier = Modifier.size(width = 320.dp, height = 220.dp),
            shape = RoundedCornerShape(0.dp),
            modifiers = modifiers,
          )
        }
      }
      waitUntil(timeoutMillis = 10_000) {
        runtime.editor?.hasPublishedFrameFor(layout = A5Layout, fontSize = 1200) == true
      }
      val a5FrameSize = runtime.editor!!.publishedBundle!!.frames.getValue(0).pixelSize

      runOnIdle { layout = B6Layout }
      waitUntil(timeoutMillis = 10_000) {
        runtime.editor?.hasPublishedFrameFor(layout = B6Layout, fontSize = 1200) == true
      }
      waitForIdle()
      val b6Bundle = runtime.editor!!.publishedBundle!!

      assertNotEquals(a5FrameSize, b6Bundle.frames.getValue(0).pixelSize)

      runOnIdle { modifiers = listOf(EditorModifier.FontSize(1800)) }
      waitUntil(timeoutMillis = 10_000) {
        runtime.editor?.hasPublishedFrameFor(layout = B6Layout, fontSize = 1800) == true
      }
      waitForIdle()
      val bodySizeBundle = runtime.editor!!.publishedBundle!!

      assertTrue(bodySizeBundle.snapshot.version > b6Bundle.snapshot.version)
      assertNotEquals(
        b6Bundle.frames.getValue(0).proof.frameKey,
        bodySizeBundle.frames.getValue(0).proof.frameKey,
      )
    } finally {
      runtime.clear()
      scope.cancel()
    }
  }

  @Test
  fun generatedPreviewKeepsItsEditorWhenPageSizeChanges() = runComposeUiTest {
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val fake = FakeFfiEditor(rootAttrsProvider = { PlainRootNode(layoutMode = A4Layout) })
    val editor = Editor(fake, scope, Dispatchers.Unconfined)
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    var layout by mutableStateOf<LayoutMode>(A4Layout)
    fake.applySnapshot(editor)

    try {
      setPreviewContent(runtime = runtime, layout = { layout })
      waitForIdle()

      runOnIdle { layout = B6Layout }
      waitForIdle()

      runOnIdle { assertSame(editor, runtime.editor) }
    } finally {
      runtime.clear()
      scope.cancel()
    }
  }

  @Test
  fun graphPreviewAppliesPageSizeBeforePersistedChangesReturn() = runComposeUiTest {
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val fake = FakeFfiEditor(rootAttrsProvider = { PlainRootNode(layoutMode = A4Layout) })
    val editor = Editor(fake, scope, Dispatchers.Unconfined)
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    var layout by mutableStateOf<LayoutMode>(A4Layout)
    fake.applySnapshot(editor)

    try {
      setPreviewContent(runtime = runtime, graph = byteArrayOf(1), layout = { layout })
      waitForIdle()
      fake.enqueued.clear()

      runOnIdle { layout = B6Layout }
      waitUntil { fake.enqueued.filterIsInstance<Message.Node>().isNotEmpty() }

      assertEquals(
        listOf(
          Message.Node(
            NodeOp.SetAttrs(id = EditorRootId, attrs = PlainNode.Root(layoutMode = B6Layout))
          )
        ),
        fake.enqueued.filterIsInstance<Message.Node>(),
      )
    } finally {
      runtime.clear()
      scope.cancel()
    }
  }

  @Test
  fun attachedDocumentPreviewDoesNotOwnOrRestyleItsEditor() = runComposeUiTest {
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val fake =
      FakeFfiEditor(
        rootAttrsProvider = { PlainRootNode(layoutMode = A4Layout) },
        rootModifiersProvider = { listOf(EditorModifier.FontSize(1200)) },
      )
    val editor = Editor(fake, scope, Dispatchers.Unconfined)
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    var shown by mutableStateOf(true)
    var layout by mutableStateOf<LayoutMode>(A4Layout)
    var modifiers by mutableStateOf(listOf<EditorModifier>(EditorModifier.FontSize(1200)))
    fake.applySnapshot(editor)

    try {
      setContent {
        if (shown) {
          CompositionLocalProvider(
            LocalDensity provides Density(1f),
            LocalThemeMode provides ResolvedThemeMode.Light,
          ) {
            EditorPreview(
              layoutMode = layout,
              runtime = runtime,
              modifier = Modifier.size(0.dp),
              shape = RoundedCornerShape(0.dp),
              source = EditorPreviewSource.AttachedEditor,
              modifiers = modifiers,
            )
          }
        }
      }
      waitForIdle()
      fake.enqueued.clear()

      runOnIdle {
        layout = B6Layout
        modifiers = listOf(EditorModifier.FontSize(1800))
      }
      waitForIdle()

      assertTrue(fake.enqueued.none { it is Message.Node || it is Message.Modifier })

      runOnIdle { shown = false }
      waitForIdle()

      runOnIdle { assertSame(editor, runtime.editor) }
    } finally {
      runtime.clear()
      scope.cancel()
    }
  }

  private fun androidx.compose.ui.test.ComposeUiTest.setPreviewContent(
    runtime: EditorRuntime,
    graph: ByteArray? = null,
    layout: () -> LayoutMode,
  ) {
    setContent {
      CompositionLocalProvider(
        LocalDensity provides Density(1f),
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        EditorPreview(
          layoutMode = layout(),
          runtime = runtime,
          modifier = Modifier.size(0.dp),
          shape = RoundedCornerShape(0.dp),
          source = graph?.let(EditorPreviewSource::Graph) ?: EditorPreviewSource.Generated,
        )
      }
    }
  }

  private companion object {
    val A4Layout =
      LayoutMode.Paginated(
        pageWidth = 794,
        pageHeight = 1123,
        pageMarginTop = 94,
        pageMarginBottom = 94,
        pageMarginLeft = 94,
        pageMarginRight = 94,
      )

    val B6Layout =
      LayoutMode.Paginated(
        pageWidth = 472,
        pageHeight = 665,
        pageMarginTop = 38,
        pageMarginBottom = 38,
        pageMarginLeft = 38,
        pageMarginRight = 38,
      )

    val A5Layout =
      LayoutMode.Paginated(
        pageWidth = 559,
        pageHeight = 794,
        pageMarginTop = 76,
        pageMarginBottom = 76,
        pageMarginLeft = 76,
        pageMarginRight = 76,
      )
  }
}

private fun Editor.hasPublishedFrameFor(layout: LayoutMode, fontSize: Int): Boolean {
  val bundle = publishedBundle ?: return false
  val presentedFontSize =
    bundle.snapshot.rootModifiers
      .orEmpty()
      .filterIsInstance<EditorModifier.FontSize>()
      .firstOrNull()
  return bundle.snapshot.rootAttrs?.layoutMode == layout &&
    presentedFontSize?.value == fontSize &&
    bundle.frames.size == bundle.snapshot.pageSizes.size &&
    bundle.frames.values.all { frame -> frame.proof.editorRevision == bundle.snapshot.version }
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
