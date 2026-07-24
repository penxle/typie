package co.typie.screen.editor.editor

import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.screen.editor.editor.header.resolveEditorHeaderGeometry
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class EditorHeaderGeometryTest {
  @Test
  fun `header geometry preserves continuous alignment and follows zoomed content bounds`() {
    val continuous =
      checkNotNull(
        resolveEditorHeaderGeometry(
          layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
          viewportWidth = 720f,
          bodyTrackWidth = 640f,
          displayZoom = 1f,
        )
      )
    assertEquals(600f, continuous.fieldWidth)
    assertEquals(60f, continuous.resolveFieldScreenLeft(scrollOffsetX = 0f))

    val paginated =
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = 720f,
        pageHeight = 960f,
        pageMarginTop = 72f,
        pageMarginBottom = 72f,
        pageMarginLeft = 64f,
        pageMarginRight = 64f,
      )

    val lowZoom =
      checkNotNull(
        resolveEditorHeaderGeometry(
          layoutSpec = paginated,
          viewportWidth = 320f,
          bodyTrackWidth = 180f,
          displayZoom = 0.25f,
        )
      )
    assertEquals(240f, lowZoom.fieldWidth)
    assertEquals(40f, lowZoom.resolveFieldScreenLeft(scrollOffsetX = 0f))

    val highZoom =
      checkNotNull(
        resolveEditorHeaderGeometry(
          layoutSpec = paginated,
          viewportWidth = 320f,
          bodyTrackWidth = 1440f,
          displayZoom = 2f,
        )
      )
    assertEquals(280f, highZoom.fieldWidth)
    assertEquals(128f, highZoom.resolveFieldScreenLeft(scrollOffsetX = 0f))
    assertEquals(20f, highZoom.resolveFieldScreenLeft(scrollOffsetX = 300f))
    assertEquals(-88f, highZoom.resolveFieldScreenLeft(scrollOffsetX = 1120f))

    val asymmetric =
      checkNotNull(
        resolveEditorHeaderGeometry(
          layoutSpec = paginated.copy(pageMarginRight = 96f),
          viewportWidth = 320f,
          bodyTrackWidth = 1440f,
          displayZoom = 2f,
        )
      )
    assertEquals(-152f, asymmetric.resolveFieldScreenLeft(scrollOffsetX = 1120f))

    assertNull(
      resolveEditorHeaderGeometry(
        layoutSpec = paginated,
        viewportWidth = 0f,
        bodyTrackWidth = 720f,
        displayZoom = 1f,
      )
    )
  }
}
