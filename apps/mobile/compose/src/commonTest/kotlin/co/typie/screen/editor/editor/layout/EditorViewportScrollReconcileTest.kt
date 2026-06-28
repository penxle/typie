package co.typie.screen.editor.editor.layout

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import co.typie.editor.EditorState
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.runtime.EditorBoundsInContainer
import co.typie.editor.scroll.EditorScrollFrame
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.resolveEditorAutoScrollPolicy
import co.typie.editor.viewport.EditorViewportState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class EditorViewportScrollReconcileTest {
  @Test
  fun `visible area shrink preserves the center document anchor`() {
    val reconcileState = EditorViewportScrollReconcileState()
    val viewportState = viewportState(scrollY = 200f)
    val visibleArea = EditorVisibleArea(viewport = Size(width = 300f, height = 300f))
    val occludedVisibleArea =
      EditorVisibleArea(viewport = Size(width = 300f, height = 300f), bottomOcclusionInset = 100f)

    val firstReconcile =
      reconcileState.reconcile(
        mode = EditorViewportScrollReconcileMode.KeepVisibleAnchor,
        viewportState = viewportState,
        scrollFrame = frame(visibleArea = visibleArea),
        visibleArea = visibleArea,
      )

    assertFalse(firstReconcile)
    assertEquals(Offset(x = 0f, y = 200f), viewportState.scrollOffset)

    val secondReconcile =
      reconcileState.reconcile(
        mode = EditorViewportScrollReconcileMode.KeepVisibleAnchor,
        viewportState = viewportState,
        scrollFrame = frame(visibleArea = occludedVisibleArea),
        visibleArea = occludedVisibleArea,
      )

    assertTrue(secondReconcile)
    assertEquals(Offset(x = 0f, y = 250f), viewportState.scrollOffset)
    assertTrue(viewportState.lastScrollWasAuto)
  }

  @Test
  fun `viewport scroll reconcile waits while direct manipulation is active`() {
    val reconcileState = EditorViewportScrollReconcileState()
    val viewportState = viewportState(scrollY = 200f)
    val visibleArea = EditorVisibleArea(viewport = Size(width = 300f, height = 300f))
    val occludedVisibleArea =
      EditorVisibleArea(viewport = Size(width = 300f, height = 300f), bottomOcclusionInset = 100f)

    reconcileState.reconcile(
      mode = EditorViewportScrollReconcileMode.KeepVisibleAnchor,
      viewportState = viewportState,
      scrollFrame = frame(visibleArea = visibleArea),
      visibleArea = visibleArea,
    )
    viewportState.updateScrollableInteractionInProgress(true)

    val blockedReconcile =
      reconcileState.reconcile(
        mode = EditorViewportScrollReconcileMode.KeepVisibleAnchor,
        viewportState = viewportState,
        scrollFrame = frame(visibleArea = occludedVisibleArea),
        visibleArea = occludedVisibleArea,
      )

    assertFalse(blockedReconcile)
    assertEquals(Offset(x = 0f, y = 200f), viewportState.scrollOffset)

    viewportState.updateScrollableInteractionInProgress(false)
    val resumedReconcile =
      reconcileState.reconcile(
        mode = EditorViewportScrollReconcileMode.KeepVisibleAnchor,
        viewportState = viewportState,
        scrollFrame = frame(visibleArea = occludedVisibleArea),
        visibleArea = occludedVisibleArea,
      )

    assertTrue(resumedReconcile)
    assertEquals(Offset(x = 0f, y = 250f), viewportState.scrollOffset)
  }

  @Test
  fun `selection reveal mode scrolls only when visible area shrink covers cursor`() {
    val reconcileState = EditorViewportScrollReconcileState()
    val viewportState = viewportState(scrollY = 200f)
    val visibleArea = EditorVisibleArea(viewport = Size(width = 300f, height = 300f))
    val occludedVisibleArea =
      EditorVisibleArea(viewport = Size(width = 300f, height = 300f), bottomOcclusionInset = 100f)

    val firstReconcile =
      reconcileState.reconcile(
        mode = EditorViewportScrollReconcileMode.RevealSelectionHead,
        viewportState = viewportState,
        scrollFrame = frame(visibleArea = visibleArea, cursorY = 450f),
        visibleArea = visibleArea,
      )

    assertFalse(firstReconcile)
    assertEquals(Offset(x = 0f, y = 200f), viewportState.scrollOffset)

    val secondReconcile =
      reconcileState.reconcile(
        mode = EditorViewportScrollReconcileMode.RevealSelectionHead,
        viewportState = viewportState,
        scrollFrame = frame(visibleArea = occludedVisibleArea, cursorY = 450f),
        visibleArea = occludedVisibleArea,
      )

    assertTrue(secondReconcile)
    assertEquals(Offset(x = 0f, y = 330f), viewportState.scrollOffset)

    val repeatedReconcile =
      reconcileState.reconcile(
        mode = EditorViewportScrollReconcileMode.RevealSelectionHead,
        viewportState = viewportState,
        scrollFrame = frame(visibleArea = occludedVisibleArea, cursorY = 450f),
        visibleArea = occludedVisibleArea,
      )

    assertFalse(repeatedReconcile)
    assertEquals(Offset(x = 0f, y = 330f), viewportState.scrollOffset)
  }

  private fun viewportState(scrollY: Float): EditorViewportState =
    EditorViewportState().apply {
      updateMeasuredBounds(
        viewportSize = Size(width = 300f, height = 300f),
        contentSize = Size(width = 300f, height = 900f),
      )
      scrollToY(scrollY)
    }

  private fun frame(visibleArea: EditorVisibleArea, cursorY: Float = 0f): EditorScrollFrame =
    EditorScrollFrame(
      state =
        EditorState(
          version = 1L,
          cursor =
            CursorMetrics(
              pageIdx = 0,
              caret = Rect(x = 0f, y = cursorY, width = 0f, height = 20f),
              line = Rect(x = 0f, y = cursorY, width = 0f, height = 20f),
            ),
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
