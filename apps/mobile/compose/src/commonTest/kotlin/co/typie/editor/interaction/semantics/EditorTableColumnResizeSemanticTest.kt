package co.typie.editor.interaction.semantics

import androidx.compose.ui.geometry.Rect
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.Alignment
import co.typie.editor.ffi.Rect as FfiRect
import co.typie.editor.ffi.TableBorderStyle
import co.typie.editor.ffi.TableOverlay
import co.typie.editor.ffi.TableOverlayColumn
import kotlin.test.Test
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.test.StandardTestDispatcher
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
