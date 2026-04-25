package co.typie.editor.scroll

import androidx.compose.ui.geometry.Size
import co.typie.editor.EditorState
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Rect as FfiRect
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.runtime.EditorBoundsInContainer
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class EditorScrollResolverTest {
  @Test
  fun `resolver returns target scroll from cursor line and policy`() {
    val frame =
      frame(
        state =
          state(
            cursor =
              CursorMetrics(
                pageIdx = 0,
                caret = FfiRect(0f, 580f, 0f, 20f),
                line = FfiRect(0f, 580f, 0f, 20f),
              ),
            pageSizes = listOf(PageSize(width = 300f, height = 620f)),
          )
      )

    val intent =
      resolveEditorScrollIntent(
        frame = frame,
        target = EditorBringIntoViewTarget.CurrentCursorLine,
        currentScroll = 200f,
      )

    assertScrollTo(intent, 360f)
  }

  @Test
  fun `resolver resolves target on newly added page before its measured offset is available`() {
    val frame =
      frame(
        state =
          state(
            cursor =
              CursorMetrics(
                pageIdx = 1,
                caret = FfiRect(0f, 100f, 0f, 20f),
                line = FfiRect(0f, 100f, 0f, 20f),
              ),
            pageSizes =
              listOf(PageSize(width = 300f, height = 500f), PageSize(width = 300f, height = 500f)),
          ),
        layoutSpec = paginatedLayout(),
      )

    val intent =
      resolveEditorScrollIntent(
        frame = frame,
        target = EditorBringIntoViewTarget.CurrentCursorLine,
        currentScroll = 200f,
      )

    assertScrollTo(intent, 404f)
  }

  @Test
  fun `resolver ignores stale measured page offsets and uses layout-spec page geometry`() {
    val frame =
      frame(
        state =
          state(
            cursor =
              CursorMetrics(
                pageIdx = 1,
                caret = FfiRect(0f, 100f, 0f, 20f),
                line = FfiRect(0f, 100f, 0f, 20f),
              ),
            pageSizes =
              listOf(PageSize(width = 300f, height = 500f), PageSize(width = 300f, height = 500f)),
          ),
        layoutSpec = paginatedLayout(),
      )

    val intent =
      resolveEditorScrollIntent(
        frame = frame,
        target = EditorBringIntoViewTarget.CurrentCursorLine,
        currentScroll = 200f,
      )

    assertScrollTo(intent, 404f)
  }

  @Test
  fun `resolver uses the same rounded page heights as layout`() {
    val frame =
      frame(
        state =
          state(
            cursor =
              CursorMetrics(
                pageIdx = 1,
                caret = FfiRect(0f, 280f, 0f, 20f),
                line = FfiRect(0f, 280f, 0f, 20f),
              ),
            pageSizes =
              listOf(PageSize(width = 300f, height = 10.26f), PageSize(width = 300f, height = 500f)),
          ),
        density = 2f,
      )

    val intent =
      resolveEditorScrollIntent(
        frame = frame,
        target = EditorBringIntoViewTarget.CurrentCursorLine,
        currentScroll = 0f,
      )

    assertScrollTo(intent, 70.5f)
  }

  private fun assertScrollTo(intent: EditorScrollIntentResult, y: Float) {
    val scrollTo = intent as? EditorScrollIntentResult.ScrollTo
    assertNotNull(scrollTo)
    assertEquals(y, scrollTo.y, 0.0001f)
  }

  private fun state(cursor: CursorMetrics, pageSizes: List<PageSize>): EditorState =
    EditorState(
      version = 1L,
      cursor = cursor,
      selection = null,
      pageSizes = pageSizes,
      documentAttrs = null,
      ime = null,
    )

  private fun frame(
    state: EditorState,
    layoutSpec: EditorDocumentLayoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 300f),
    density: Float = 1f,
  ): EditorScrollFrame {
    val visibleArea = EditorVisibleArea(viewport = Size(width = 300f, height = 300f))
    return EditorScrollFrame(
      state = state,
      layoutSpec = layoutSpec,
      displayZoom = 1f,
      visibleArea = visibleArea,
      autoScrollPolicy = resolveEditorAutoScrollPolicy(visibleArea = visibleArea),
      headerHeight = 0f,
      density = density,
      editorBounds = EditorBoundsInContainer(x = 0f, y = 0f, width = 300f, height = 1000f),
    )
  }

  private fun paginatedLayout(): EditorDocumentLayoutSpec.Paginated =
    EditorDocumentLayoutSpec.Paginated(
      pageWidth = 300f,
      pageHeight = 500f,
      pageMarginTop = 0f,
      pageMarginBottom = 0f,
      pageMarginLeft = 0f,
      pageMarginRight = 0f,
    )
}
