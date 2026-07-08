package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect as ComposeRect
import androidx.compose.ui.geometry.Size as ComposeSize
import androidx.compose.ui.input.pointer.changedToUp
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.Alignment
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionEndpoints
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.TableBorderStyle
import co.typie.editor.ffi.TableOverlay
import co.typie.editor.ffi.TableOverlayColumn
import co.typie.editor.ffi.TableOverlayRow
import co.typie.editor.interaction.EditorInteractionScope
import co.typie.editor.interaction.LocalEditorInteractionScope
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.viewport.EditorViewportState
import co.typie.ext.ScrollGestureLockState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class EditorSelectionHandleOverlayDesktopTest {
  @Test
  fun handleOverlayRoutesDragBeforeLowerPointerOverlay() = runComposeUiTest {
    val selection =
      Selection(
        anchor = Position("text", 0, Affinity.Downstream),
        head = Position("text", 5, Affinity.Downstream),
      )
    val endpoints =
      SelectionEndpoints(
        from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
        to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
        fromPosition = Position("text", 0, Affinity.Downstream),
        toPosition = Position("text", 5, Affinity.Downstream),
      )
    val fake =
      FakeFfiEditor(
        selectionProvider = { selection },
        selectionEndpointsProvider = { endpoints },
        pageSizesProvider = { listOf(Size(width = 120f, height = 120f)) },
      )
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val uiState =
      EditorUiState().apply {
        updateFocus(true)
        updatePageOffset(page = 0, offset = Offset.Zero)
      }
    editor.sync {}
    try {
      setContent {
        val interactionScope = remember { EditorInteractionScope(coroutineScope = scope) }
        val viewportState = remember {
          EditorViewportState().apply {
            updateMeasuredBounds(
              viewportSize = ComposeSize(width = 120f, height = 120f),
              contentSize = ComposeSize(width = 120f, height = 120f),
            )
          }
        }
        val visibleArea = remember { EditorVisibleArea(viewport = ComposeSize(120f, 120f)) }
        val bringIntoViewRequests = remember { EditorBringIntoViewRequests() }
        val scrollGestureLockState = remember { ScrollGestureLockState() }

        SideEffect {
          interactionScope.update(
            editor = editor,
            bringIntoViewRequests = bringIntoViewRequests,
            uiState = uiState,
            visibleArea = visibleArea,
            viewportState = viewportState,
            density = 1f,
            scrollGestureLockState = scrollGestureLockState,
            viewportZoomConfig = null,
            onSelectionHaptic = {},
          )
          interactionScope.onEditorStateChanged(editor.state)
        }

        CompositionLocalProvider(LocalEditorInteractionScope provides interactionScope) {
          Box(Modifier.testTag(RootTag).size(120.dp)) {
            Box(
              Modifier.fillMaxSize().pointerInput(Unit) {
                awaitEachGesture {
                  awaitFirstDown(requireUnconsumed = false)
                  while (true) {
                    val event = awaitPointerEvent()
                    val change = event.changes.first()
                    if (change.changedToUp()) {
                      change.consume()
                      break
                    }
                    change.consume()
                  }
                }
              }
            )
            EditorSelectionHandleOverlay(
              editor = editor,
              uiState = uiState,
              editorRectInOverlay = ComposeRect(Offset.Zero, ComposeSize(120f, 120f)),
              density = 1f,
            )
          }
        }
      }
      waitForIdle()

      onNodeWithTag(RootTag).performTouchInput {
        down(Offset(x = 42f, y = 30f))
        moveTo(Offset(x = 52f, y = 50f))
        up()
      }
      waitForIdle()

      val extend =
        fake.enqueued.filterIsInstance<Message.Selection>().single().op as SelectionOp.ExtendTo
      assertEquals(endpoints.fromPosition, extend.anchor)
      assertEquals(50f, extend.headX)
      assertEquals(44f, extend.headY)
      assertFalse(extend.allowCollapse)
      assertNull(extend.baseSelection)
    } finally {
      editor.dispose()
      scope.cancel()
    }
  }

  @Test
  fun tableHandleOverlayRoutesDragBeforeLowerPointerOverlay() = runComposeUiTest {
    val selection =
      Selection(
        anchor = Position("cell-text", 0, Affinity.Downstream),
        head = Position("cell-text", 0, Affinity.Downstream),
      )
    val fake =
      FakeFfiEditor(
        selectionProvider = { selection },
        pageSizesProvider = { listOf(Size(width = 120f, height = 120f)) },
        tableOverlaysProvider = {
          listOf(tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0))
        },
      )
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val uiState =
      EditorUiState().apply {
        updateFocus(true)
        updatePageOffset(page = 0, offset = Offset.Zero)
      }
    editor.sync {}
    try {
      setContent {
        val interactionScope = remember { EditorInteractionScope(coroutineScope = scope) }
        val viewportState = remember {
          EditorViewportState().apply {
            updateMeasuredBounds(
              viewportSize = ComposeSize(width = 120f, height = 120f),
              contentSize = ComposeSize(width = 120f, height = 120f),
            )
          }
        }
        val visibleArea = remember { EditorVisibleArea(viewport = ComposeSize(120f, 120f)) }
        val bringIntoViewRequests = remember { EditorBringIntoViewRequests() }
        val scrollGestureLockState = remember { ScrollGestureLockState() }

        SideEffect {
          interactionScope.update(
            editor = editor,
            bringIntoViewRequests = bringIntoViewRequests,
            uiState = uiState,
            visibleArea = visibleArea,
            viewportState = viewportState,
            density = 1f,
            scrollGestureLockState = scrollGestureLockState,
            viewportZoomConfig = null,
            onSelectionHaptic = {},
          )
          interactionScope.onEditorStateChanged(editor.state)
        }

        CompositionLocalProvider(LocalEditorInteractionScope provides interactionScope) {
          Box(Modifier.testTag(RootTag).size(120.dp)) {
            Box(
              Modifier.fillMaxSize().pointerInput(Unit) {
                awaitEachGesture {
                  awaitFirstDown(requireUnconsumed = false)
                  while (true) {
                    val event = awaitPointerEvent()
                    val change = event.changes.first()
                    if (change.changedToUp()) {
                      change.consume()
                      break
                    }
                    change.consume()
                  }
                }
              }
            )
            EditorTableCellSelectionOverlay(
              editor = editor,
              uiState = uiState,
              editorRectInOverlay = ComposeRect(Offset.Zero, ComposeSize(120f, 120f)),
              density = 1f,
            )
          }
        }
      }
      waitForIdle()

      onNodeWithTag(RootTag).performTouchInput {
        down(Offset(x = 60f, y = 60f))
        moveTo(Offset(x = 100f, y = 90f))
        up()
      }
      waitForIdle()

      val extend =
        fake.enqueued.filterIsInstance<Message.Selection>().single().op as SelectionOp.ExtendTo
      assertEquals(selection.anchor, extend.anchor)
      assertEquals(100f, extend.headX)
      assertEquals(90f, extend.headY)
      assertEquals(selection, extend.baseSelection)
      assertFalse(extend.allowCollapse)
    } finally {
      editor.dispose()
      scope.cancel()
    }
  }

  private companion object {
    const val RootTag = "selection-handle-overlay-root"

    fun tableOverlay(
      isFocused: Boolean = false,
      focusedRowIndex: Int? = null,
      focusedColIndex: Int? = null,
    ): TableOverlay =
      TableOverlay(
        tableId = "table",
        pageIdx = 0,
        bounds = Rect(x = 10f, y = 20f, width = 100f, height = 80f),
        borderStyle = TableBorderStyle.Solid,
        align = Alignment.Left,
        proportion = 1f,
        contentWidth = 100f,
        rows =
          listOf(
            TableOverlayRow(index = 0, height = 40f, position = 40f, backgroundColor = null),
            TableOverlayRow(index = 1, height = 40f, position = 80f, backgroundColor = null),
          ),
        columns =
          listOf(
            TableOverlayColumn(index = 0, widthAsPx = 50f, position = 50f, backgroundColor = null),
            TableOverlayColumn(index = 1, widthAsPx = 50f, position = 100f, backgroundColor = null),
          ),
        rowCount = 2,
        isLastRowFragment = true,
        isFocused = isFocused,
        focusedRowIndex = focusedRowIndex,
        focusedColIndex = focusedColIndex,
        cellSelection = null,
      )
  }
}
