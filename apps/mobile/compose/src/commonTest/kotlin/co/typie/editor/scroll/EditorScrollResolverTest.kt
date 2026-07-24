package co.typie.editor.scroll

import androidx.compose.ui.geometry.Size
import co.typie.editor.EditorState
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Rect as FfiRect
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionEndpoints
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.runtime.EditorBoundsInContainer
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class EditorScrollResolverTest {
  @Test
  fun `resolver returns target scroll from collapsed selection head and policy`() {
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
        target = EditorBringIntoViewTarget.CurrentSelectionHead,
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
        target = EditorBringIntoViewTarget.CurrentSelectionHead,
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
        target = EditorBringIntoViewTarget.CurrentSelectionHead,
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
        target = EditorBringIntoViewTarget.CurrentSelectionHead,
        currentScroll = 0f,
      )

    assertScrollTo(intent, 70.5f)
  }

  @Test
  fun `selection head target resolves from selection endpoint instead of collapsed cursor line`() {
    val anchor = position(offset = 1)
    val head = position(offset = 8)
    val frame =
      frame(
        state =
          state(
            cursor =
              CursorMetrics(
                pageIdx = 0,
                caret = FfiRect(0f, 20f, 0f, 20f),
                line = FfiRect(0f, 20f, 0f, 20f),
              ),
            selection = Selection(anchor = anchor, head = head),
            selectionEndpoints =
              SelectionEndpoints(
                from = PageRect(pageIdx = 0, rect = FfiRect(0f, 20f, 0f, 20f)),
                to = PageRect(pageIdx = 0, rect = FfiRect(0f, 580f, 0f, 20f)),
                fromPosition = anchor,
                toPosition = head,
              ),
            pageSizes = listOf(PageSize(width = 300f, height = 620f)),
          )
      )

    val intent =
      resolveEditorScrollIntent(
        frame = frame,
        target = EditorBringIntoViewTarget.CurrentSelectionHead,
        currentScroll = 200f,
      )

    assertScrollTo(intent, 360f)
  }

  @Test
  fun `selection head target does not fall back to cursor line without selection`() {
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
            selection = null,
            selectionEndpoints = null,
            pageSizes = listOf(PageSize(width = 300f, height = 620f)),
          )
      )

    val intent =
      resolveEditorScrollIntent(
        frame = frame,
        target = EditorBringIntoViewTarget.CurrentSelectionHead,
        currentScroll = 200f,
      )

    assertEquals(EditorScrollIntentResult.ConsumedWithoutScroll, intent)
  }

  @Test
  fun `page rects target reveals the vertical union across pages`() {
    val frame =
      frame(
        state =
          state(
            cursor = null,
            selection = null,
            pageSizes =
              listOf(PageSize(width = 300f, height = 620f), PageSize(width = 300f, height = 620f)),
          ),
        layoutSpec = paginatedLayout(),
      )

    val intent =
      resolveEditorScrollIntent(
        frame = frame,
        target =
          EditorBringIntoViewTarget.PageRects(
            listOf(
              PageRect(pageIdx = 0, rect = FfiRect(x = 0f, y = 500f, width = 40f, height = 20f)),
              PageRect(pageIdx = 0, rect = FfiRect(x = 0f, y = 580f, width = 40f, height = 20f)),
              PageRect(pageIdx = 1, rect = FfiRect(x = 0f, y = 20f, width = 40f, height = 20f)),
            )
          ),
        currentScroll = 0f,
      )

    assertScrollTo(intent, 440f)
  }

  @Test
  fun `selection head target height resolves from endpoint when range selection has no cursor`() {
    val anchor = position(offset = 1)
    val head = position(offset = 8)
    val height =
      resolveBringIntoViewTargetHeight(
        state =
          state(
            cursor = null,
            selection = Selection(anchor = anchor, head = head),
            selectionEndpoints =
              SelectionEndpoints(
                from = PageRect(pageIdx = 0, rect = FfiRect(0f, 20f, 0f, 16f)),
                to = PageRect(pageIdx = 0, rect = FfiRect(0f, 580f, 0f, 20f)),
                fromPosition = anchor,
                toPosition = head,
              ),
            pageSizes = listOf(PageSize(width = 300f, height = 620f)),
          ),
        layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 300f),
        target = EditorBringIntoViewTarget.CurrentSelectionHead,
        displayZoom = 1.5f,
      )

    assertEquals(30f, requireNotNull(height), 0.0001f)
  }

  @Test
  fun `page rects target height resolves from the vertical union across pages`() {
    val height =
      resolveBringIntoViewTargetHeight(
        state =
          state(
            cursor = null,
            selection = null,
            pageSizes =
              listOf(PageSize(width = 300f, height = 620f), PageSize(width = 300f, height = 620f)),
          ),
        layoutSpec = paginatedLayout(),
        target =
          EditorBringIntoViewTarget.PageRects(
            listOf(
              PageRect(pageIdx = 0, rect = FfiRect(x = 0f, y = 500f, width = 40f, height = 20f)),
              PageRect(pageIdx = 0, rect = FfiRect(x = 0f, y = 580f, width = 40f, height = 20f)),
              PageRect(pageIdx = 1, rect = FfiRect(x = 0f, y = 20f, width = 40f, height = 20f)),
            )
          ),
        displayZoom = 1f,
      )

    assertEquals(184f, requireNotNull(height), 0.0001f)
  }

  private fun assertScrollTo(intent: EditorScrollIntentResult, y: Float) {
    val scrollTo = intent as? EditorScrollIntentResult.ScrollTo
    assertNotNull(scrollTo)
    assertEquals(y, scrollTo.y, 0.0001f)
  }

  private fun state(
    cursor: CursorMetrics?,
    pageSizes: List<PageSize>,
    selection: Selection? = cursor?.let { collapsedSelection() },
    selectionEndpoints: SelectionEndpoints? = null,
  ): EditorState =
    EditorState(
      version = 1L,
      cursor = cursor,
      selection = selection,
      selectionEndpoints = selectionEndpoints,
      pageSizes = pageSizes,
      externalElements = emptyList(),
      rootAttrs = null,
      rootModifiers = null,
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

  private fun position(offset: Int): Position =
    Position(node = "paragraph", offset = offset, affinity = Affinity.Downstream)

  private fun collapsedSelection(): Selection {
    val position = position(offset = 0)
    return Selection(anchor = position, head = position)
  }
}
