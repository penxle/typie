package co.typie.editor.interaction.semantics

import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.unit.IntSize
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.Alignment
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.FrameKey
import co.typie.editor.ffi.Rect as FfiRect
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.TableBorderStyle
import co.typie.editor.ffi.TableOverlay
import co.typie.editor.ffi.TableOverlayColumn
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineExceptionHandler
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest

class EditorTableColumnResizeSemanticTest {
  @Test
  fun `terminal commit failure clears the resize interaction`() = runTest {
    val failure = IllegalStateException("commit failed")
    val reported = mutableListOf<Throwable>()
    val dispatcher = StandardTestDispatcher(testScheduler)
    val editor =
      Editor(
        FakeFfiEditor(onTick = { throw failure }),
        this,
        dispatcher,
        onError = { _, error -> reported += error },
      )
    val semantic = EditorTableColumnResizeSemantic()

    semantic.press(editor, columnResizePlacement())
    assertTrue(semantic.start())
    semantic.update(5f)
    semantic.end()

    assertTrue(editor.terminal)
    assertSame(failure, reported.single())
    assertNull(semantic.presentation.draft)
  }

  @Test
  fun `sub-epsilon column resize does not enqueue a commit`() = runTest {
    val dispatcher = StandardTestDispatcher(testScheduler)
    val fake = FakeFfiEditor()
    val editor = Editor(fake, this, dispatcher)
    val semantic = EditorTableColumnResizeSemantic()

    semantic.press(editor, columnResizePlacement())
    assertTrue(semantic.start())
    semantic.update(0.005f)
    semantic.end()

    assertTrue(fake.enqueued.isEmpty())
  }

  @Test
  fun `unavailable surface clears the committed draft without escaping the interaction`() =
    runTest {
      val dispatcher = StandardTestDispatcher(testScheduler)
      val uncaught = mutableListOf<Throwable>()
      val editorScope =
        CoroutineScope(
          SupervisorJob() + dispatcher + CoroutineExceptionHandler { _, error -> uncaught += error }
        )
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) },
        )
      val editor = Editor(fake, editorScope, dispatcher)
      val semantic = EditorTableColumnResizeSemantic()
      var pendingFrameKey: FrameKey? = null

      try {
        editor.activateVisualHost(Any())
        val session =
          editor.attachSurface(0, 10L, 100.0, 100.0, 1.0) { frameKey -> pendingFrameKey = frameKey }
        editor.requestSurfacePages(setOf(0))
        advanceUntilIdle()
        editor.deliverFrame(
          session = session,
          bitmap = ImageBitmap(width = 100, height = 100),
          pixelSize = IntSize(width = 100, height = 100),
          editorRevision = 0L,
          frameKey = requireNotNull(pendingFrameKey).value,
        )
        advanceUntilIdle()
        pendingFrameKey = null

        semantic.press(editor, columnResizePlacement())
        assertTrue(semantic.start())
        semantic.update(5f)
        semantic.end()
        advanceUntilIdle()

        editor.surfaceTargetUnavailable(session, requireNotNull(pendingFrameKey))
        advanceUntilIdle()

        assertTrue(uncaught.isEmpty())
        assertFalse(editor.terminal)
        assertNull(semantic.presentation.draft)
      } finally {
        editorScope.cancel()
      }
    }

  private fun columnResizePlacement(): EditorTableColumnResizePlacement {
    val overlay =
      TableOverlay(
        tableId = "table",
        pageIdx = 0,
        bounds = FfiRect(x = 0f, y = 0f, width = 100f, height = 80f),
        borderStyle = TableBorderStyle.Solid,
        align = Alignment.Left,
        proportion = 100f,
        contentWidth = 100f,
        minProportionWidth = 80f,
        maxProportionWidth = 100f,
        rows = emptyList(),
        columns =
          listOf(
            TableOverlayColumn(index = 0, widthAsPx = 50f, position = 50f),
            TableOverlayColumn(index = 1, widthAsPx = 50f, position = 100f),
          ),
        rowCount = 0,
        isLastRowFragment = true,
        isFocused = true,
        focusedRowIndex = null,
        focusedColIndex = null,
        cellSelection = null,
      )
    return EditorTableColumnResizePlacement(
      target =
        EditorTableColumnResizeTarget(
          overlay = overlay,
          colIndex = 0,
          localColIndex = 0,
          isTableResize = false,
          pageX = 50f,
        ),
      centerX = 50f,
      top = 0f,
      bottom = 80f,
      handleRects = listOf(Rect(left = 40f, top = 0f, right = 60f, bottom = 80f)),
      pxPerPageUnit = 1f,
    )
  }
}
