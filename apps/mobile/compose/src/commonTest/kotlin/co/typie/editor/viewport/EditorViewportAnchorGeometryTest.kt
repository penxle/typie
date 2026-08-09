package co.typie.editor.viewport

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import co.typie.editor.EditorState
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Size as PageSize
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
  fun `continuous viewport anchor origin includes the body top spacer`() {
    assertEquals(40f, resolveViewportAnchorContentOriginY(frame()))
  }

  private fun frame(): EditorScrollFrame {
    val visibleArea = EditorVisibleArea(viewport = Size(width = 300f, height = 300f))
    return EditorScrollFrame(
      state =
        EditorState(
          version = 1L,
          cursor = null,
          selection = null,
          pageSizes = listOf(PageSize(width = 600f, height = 900f)),
          externalElements = emptyList(),
          rootAttrs = null,
          rootModifiers = null,
          ime = null,
        ),
      layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
      displayZoom = 1f,
      visibleArea = visibleArea,
      autoScrollPolicy = resolveEditorAutoScrollPolicy(visibleArea = visibleArea),
      headerHeight = 0f,
      density = 1f,
      editorBounds = EditorBoundsInContainer(x = 0f, y = 0f, width = 600f, height = 900f),
    )
  }
}
