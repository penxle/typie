package co.typie.editor.viewport

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import co.typie.editor.EditorState
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.ResolvedViewportAnchor
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.ffi.ViewportAnchorPoint
import co.typie.editor.runtime.EditorBoundsInContainer
import co.typie.editor.scroll.EditorScrollFrame
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.resolveEditorAutoScrollPolicy
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorViewportAnchorGeometryTest {
  @Test
  fun `viewport center follows horizontal pan in page-local coordinates`() {
    val frame = frame()
    val contentOriginY = resolveViewportAnchorContentOriginY(frame)

    val point =
      viewportCenterAnchorPoint(
        frame = frame,
        scrollOffset = Offset(x = 100f, y = 100f),
        contentOriginY = contentOriginY,
      )

    assertEquals(250f, point?.x)
    assertEquals(210f, point?.y)
  }

  @Test
  fun `viewport center clamps to the last page bottom`() {
    val frame = frame(visibleArea = EditorVisibleArea(viewport = Size(width = 300f, height = 20f)))

    val point =
      viewportCenterAnchorPoint(
        frame = frame,
        scrollOffset = Offset(x = 0f, y = 1000f),
        contentOriginY = 0f,
      )

    assertEquals(900f, point?.y)
  }

  @Test
  fun `paginated viewport center chooses the nearest page across a gap`() {
    val frame =
      frame(
        pageSizes =
          listOf(PageSize(width = 600f, height = 100f), PageSize(width = 600f, height = 100f)),
        layoutSpec =
          EditorDocumentLayoutSpec.Paginated(
            pageWidth = 600f,
            pageHeight = 100f,
            pageMarginTop = 0f,
            pageMarginBottom = 0f,
            pageMarginLeft = 0f,
            pageMarginRight = 0f,
          ),
        visibleArea = EditorVisibleArea(viewport = Size(width = 300f, height = 20f)),
      )

    val beforeMidpoint =
      viewportCenterAnchorPoint(frame, Offset(x = 0f, y = 100f), contentOriginY = 0f)
    val afterMidpoint =
      viewportCenterAnchorPoint(frame, Offset(x = 0f, y = 104f), contentOriginY = 0f)

    assertEquals(0, beforeMidpoint?.pageIdx)
    assertEquals(100f, beforeMidpoint?.y)
    assertEquals(1, afterMidpoint?.pageIdx)
    assertEquals(0f, afterMidpoint?.y)
  }

  @Test
  fun `continuous viewport anchor origin includes the body top spacer`() {
    assertEquals(40f, resolveViewportAnchorContentOriginY(frame()))
  }

  @Test
  fun `resolved anchor geometry includes displayed horizontal page position`() {
    val geometry =
      ResolvedViewportAnchor(
          point = ViewportAnchorPoint(pageIdx = 0, x = 40f, y = 50f),
          rect = null,
        )
        .toEditorViewportAnchorGeometry(frame = frame(), contentOriginY = 40f)

    assertEquals(40f, geometry?.pointX)
    assertEquals(90f, geometry?.pointY)
  }

  private fun frame(
    pageSizes: List<PageSize> = listOf(PageSize(width = 600f, height = 900f)),
    layoutSpec: EditorDocumentLayoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
    visibleArea: EditorVisibleArea = EditorVisibleArea(viewport = Size(width = 300f, height = 300f)),
  ): EditorScrollFrame {
    return EditorScrollFrame(
      state =
        EditorState(
          version = 1L,
          cursor = null,
          selection = null,
          pageSizes = pageSizes,
          externalElements = emptyList(),
          rootAttrs = null,
          rootModifiers = null,
          ime = null,
        ),
      layoutSpec = layoutSpec,
      displayZoom = 1f,
      visibleArea = visibleArea,
      autoScrollPolicy = resolveEditorAutoScrollPolicy(visibleArea = visibleArea),
      headerHeight = 0f,
      density = 1f,
      editorBounds = EditorBoundsInContainer(x = 0f, y = 0f, width = 600f, height = 900f),
    )
  }
}
