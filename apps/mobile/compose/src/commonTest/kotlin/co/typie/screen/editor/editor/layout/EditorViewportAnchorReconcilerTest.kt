package co.typie.screen.editor.editor.layout

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.FakeFfiEditor
import co.typie.editor.PublishedBundle
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.ResolvedViewportAnchor
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.ffi.ViewportAnchor
import co.typie.editor.ffi.ViewportAnchorPoint
import co.typie.editor.ffi.ViewportAnchorResolution
import co.typie.editor.runtime.EditorBoundsInContainer
import co.typie.editor.scroll.EditorScrollFrame
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.resolveEditorAutoScrollPolicy
import co.typie.editor.viewport.EditorViewportAnchorState
import co.typie.editor.viewport.EditorViewportState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runTest

class EditorViewportAnchorReconcilerTest {
  private val selectionAnchor = ViewportAnchor.Node(node = "1:1", offsetX = 0f, offsetY = 0f)
  private val viewportAnchor = ViewportAnchor.Node(node = "2:1", offsetX = 0f, offsetY = 0f)

  @Test
  fun `direct scroll restores preferred selection after it returns inside cursor guard`() =
    runTest {
      val visibleArea = visibleArea()
      val frame = frame(visibleArea)
      val anchorState = EditorViewportAnchorState()
      val viewportState = viewportState(scrollY = 100f)
      val editor = editor()

      reconcile(editor, anchorState, frame, viewportState, visibleArea)
      assertEquals(selectionAnchor, anchorState.identity)

      viewportState.scrollToY(350f)
      reconcile(editor, anchorState, frame, viewportState, visibleArea)
      assertEquals(viewportAnchor, anchorState.identity)

      viewportState.scrollToY(120f)
      reconcile(editor, anchorState, frame, viewportState, visibleArea)
      assertEquals(selectionAnchor, anchorState.identity)
    }

  @Test
  fun `smooth reveal tracks the viewport center without replacing the running scroll`() = runTest {
    val visibleArea = visibleArea()
    val frame = frame(visibleArea)
    val anchorState = EditorViewportAnchorState()
    val viewportState = viewportState(scrollY = 100f)
    val editor = editor()

    reconcile(editor, anchorState, frame, viewportState, visibleArea)
    viewportState.scrollToY(targetY = 250f, isAutoScroll = true)
    reconcile(
      editor = editor,
      anchorState = anchorState,
      frame = frame,
      viewportState = viewportState,
      visibleArea = visibleArea,
      smoothRevealActive = true,
    )

    assertEquals(viewportAnchor, anchorState.identity)
    assertEquals(Offset(x = 0f, y = 250f), viewportState.scrollOffset)
  }

  @Test
  fun `visible area shrink moves the active rect only enough to keep it guarded`() = runTest {
    val visibleArea = visibleArea()
    val occluded = visibleArea(bottomOcclusionInset = 100f)
    val frame = frame(visibleArea)
    val anchorState = EditorViewportAnchorState()
    val viewportState = viewportState(scrollY = 100f)
    val editor = editor(selectionY = 260f)

    reconcile(editor, anchorState, frame, viewportState, visibleArea)
    reconcile(editor, anchorState, frame.copy(visibleArea = occluded), viewportState, occluded)

    assertEquals(Offset(x = 0f, y = 130f), viewportState.scrollOffset)
    assertTrue(viewportState.lastScrollWasAuto)
  }

  private fun reconcile(
    editor: Editor,
    anchorState: EditorViewportAnchorState,
    frame: EditorScrollFrame,
    viewportState: EditorViewportState,
    visibleArea: EditorVisibleArea,
    smoothRevealActive: Boolean = false,
  ) {
    reconcileViewportAnchorObservation(
      editor = editor,
      anchorState = anchorState,
      bundle = PublishedBundle(snapshot = frame.state, frames = emptyMap()),
      frame = frame,
      viewportState = viewportState,
      visibleArea = visibleArea,
      mode = EditorViewportScrollReconcileMode.Enabled,
      smoothRevealActive = smoothRevealActive,
      handoffToSelection = false,
      selectionRevealOrigin = null,
    )
  }

  private fun TestScope.editor(selectionY: Float = 200f): Editor {
    val selectionRect = PageRect(pageIdx = 0, rect = Rect(0f, selectionY - 10f, 1f, 20f))
    return Editor(
      FakeFfiEditor(
        captureSelectionViewportAnchorProvider = { selectionAnchor },
        captureViewportAnchorAtProvider = { _, _ -> viewportAnchor },
        resolveViewportAnchorProvider = { _, anchor ->
          val resolved =
            when (anchor) {
              selectionAnchor ->
                ResolvedViewportAnchor(
                  point = ViewportAnchorPoint(pageIdx = 0, x = 0f, y = selectionY),
                  rect = selectionRect,
                )
              viewportAnchor ->
                ResolvedViewportAnchor(
                  point = ViewportAnchorPoint(pageIdx = 0, x = 0f, y = 500f),
                  rect = null,
                )
              else -> null
            }
          if (resolved == null) {
            ViewportAnchorResolution.Deleted
          } else {
            ViewportAnchorResolution.Resolved(resolved)
          }
        },
      ),
      this,
      StandardTestDispatcher(testScheduler),
    )
  }

  private fun viewportState(scrollY: Float): EditorViewportState =
    EditorViewportState().apply {
      updateMeasuredBounds(
        viewportSize = Size(width = 300f, height = 300f),
        contentSize = Size(width = 300f, height = 900f),
      )
      scrollToY(scrollY)
    }

  private fun visibleArea(bottomOcclusionInset: Float = 0f): EditorVisibleArea =
    EditorVisibleArea(
      viewport = Size(width = 300f, height = 300f),
      bottomOcclusionInset = bottomOcclusionInset,
    )

  private fun frame(visibleArea: EditorVisibleArea): EditorScrollFrame =
    EditorScrollFrame(
      state =
        EditorState(
          version = 1L,
          cursor = null,
          selection = null,
          pageSizes = listOf(PageSize(width = 300f, height = 900f)),
          externalElements = emptyList(),
          rootAttrs = null,
          rootModifiers = null,
          ime = null,
        ),
      layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 300f),
      displayZoom = 1f,
      visibleArea = visibleArea,
      autoScrollPolicy = resolveEditorAutoScrollPolicy(visibleArea = visibleArea),
      headerHeight = 0f,
      density = 1f,
      editorBounds = EditorBoundsInContainer(x = 0f, y = 0f, width = 300f, height = 900f),
    )
}
