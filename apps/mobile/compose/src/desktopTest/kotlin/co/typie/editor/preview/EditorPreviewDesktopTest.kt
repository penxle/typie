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
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.PlainRootNode
import co.typie.editor.runtime.EditorRuntime
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertSame
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class EditorPreviewDesktopTest {
  @Test
  fun generatedPreviewKeepsItsEditorWhenPageSizeChanges() = runComposeUiTest {
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val fake = FakeFfiEditor(rootAttrsProvider = { PlainRootNode(layoutMode = A4Layout) })
    val editor = Editor(fake, scope, Dispatchers.Unconfined)
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    var layout by mutableStateOf<LayoutMode>(A4Layout)
    editor.sync {}

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
    editor.sync {}

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
  fun previewUpdatesEditorLayoutBeforeRenderInvalidation() = runComposeUiTest {
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    var reportedLayout: LayoutMode = A4Layout
    val fake =
      FakeFfiEditor(
        onTick = { listOf(EditorEvent.RenderInvalidated) },
        rootAttrsProvider = { PlainRootNode(layoutMode = reportedLayout) },
      )
    val editor = Editor(fake, scope, Dispatchers.Unconfined)
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    var layout by mutableStateOf<LayoutMode>(A4Layout)
    editor.sync {}

    try {
      setPreviewContent(runtime = runtime, layout = { layout })
      waitForIdle()

      val surface =
        editor.attachSurface(
          page = 0,
          handle = 1L,
          width = A4Layout.pageWidth.toDouble(),
          height = A4Layout.pageHeight.toDouble(),
          scaleFactor = 1.0,
        )
      var observeNextRender = false
      var layoutAtRender: LayoutMode? = null
      val off =
        editor.on<EditorEvent.RenderInvalidated> { _, _ ->
          if (observeNextRender) {
            layoutAtRender = editor.rootAttrs?.layoutMode
            editor.onPageSettled(page = 0, version = Long.MAX_VALUE)
          }
        }

      runOnIdle {
        observeNextRender = true
        reportedLayout = B6Layout
        layout = B6Layout
      }
      waitUntil { layoutAtRender != null }

      assertEquals(B6Layout, layoutAtRender)
      off()
      surface.detach()
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
          graph = graph,
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
        pageWidth = 484,
        pageHeight = 686,
        pageMarginTop = 57,
        pageMarginBottom = 57,
        pageMarginLeft = 57,
        pageMarginRight = 57,
      )
  }
}
