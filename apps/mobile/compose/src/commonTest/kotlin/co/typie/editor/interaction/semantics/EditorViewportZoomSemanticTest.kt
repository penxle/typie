package co.typie.editor.interaction.semantics

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import co.typie.editor.EditorZoomController
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.viewport.EditorViewportState
import kotlin.math.ln
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class EditorViewportZoomSemanticTest {
  @Test
  fun `pinch zoom keeps the anchor under the focal point`() {
    val fixture = Fixture()

    assertTrue(fixture.semantic.beginPinch(focalPx = Offset(80f, 150f), distancePx = 100f))
    assertTrue(fixture.viewportState.isTransforming)
    assertTrue(fixture.semantic.updatePinch(focalPx = Offset(80f, 150f), distancePx = 150f))

    assertEquals(1.5f, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(Offset(40f, 75f), fixture.viewportState.scrollOffset)

    fixture.semantic.end()
    assertEquals(false, fixture.viewportState.isTransforming)
    assertEquals(fixture.zoomController.displayZoom, fixture.zoomController.renderZoom, 0.0001f)
  }

  @Test
  fun `pointer signal zoom shares the viewport zoom semantic`() {
    val fixture = Fixture()
    val normalizedDeltaForOneAndHalfZoom = -240f * ln(1.5f)

    assertTrue(fixture.semantic.beginPointerSignal())
    assertTrue(fixture.viewportState.isTransforming)
    assertTrue(
      fixture.semantic.updatePointerSignal(
        focalPx = Offset(80f, 150f),
        normalizedDelta = normalizedDeltaForOneAndHalfZoom,
      )
    )

    assertEquals(1.5f, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(Offset(40f, 75f), fixture.viewportState.scrollOffset)

    fixture.semantic.end()
    assertEquals(false, fixture.viewportState.isTransforming)
    assertEquals(fixture.zoomController.displayZoom, fixture.zoomController.renderZoom, 0.0001f)
  }

  private class Fixture {
    val layoutSpec =
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = 720f,
        pageHeight = 960f,
        pageMarginTop = 0f,
        pageMarginBottom = 0f,
        pageMarginLeft = 0f,
        pageMarginRight = 0f,
      )
    val pageSizes = listOf(PageSize(width = 720f, height = 960f))
    val zoomController = EditorZoomController()
    val viewportState =
      EditorViewportState().apply {
        updateMeasuredBounds(
          viewportSize = Size(width = 100f, height = 120f),
          contentSize = Size(width = 2000f, height = 2000f),
        )
      }
    val uiState =
      EditorUiState().apply {
        updateDisplayZoom(1f)
        updatePageOffset(page = 0, offset = Offset.Zero)
      }
    val semantic =
      EditorViewportZoomSemantic().apply {
        configure(
          EditorViewportZoomSemanticConfig(
            layoutSpec = layoutSpec,
            zoomController = zoomController,
            viewportState = viewportState,
            uiState = uiState,
            pageSizes = pageSizes,
            viewportWidth = 720f,
            density = 1f,
            onZoomSnap = {},
          )
        )
      }

    init {
      zoomController.syncLayout(layoutSpec = layoutSpec, viewportWidth = 720f)
    }
  }
}
