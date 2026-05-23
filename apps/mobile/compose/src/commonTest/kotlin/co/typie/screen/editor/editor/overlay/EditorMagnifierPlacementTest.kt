package co.typie.screen.editor.editor.overlay

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import co.typie.editor.scroll.EditorVisibleArea
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorMagnifierPlacementTest {
  @Test
  fun `magnifier stays above focal point when there is room`() {
    val placement =
      resolveEditorMagnifierPlacement(
        focalPosition = Offset(260f, 300f),
        overlaySize = Size(width = 400f, height = 700f),
        visibleArea = EditorVisibleArea(viewport = Size(width = 400f, height = 700f)),
        density = 1f,
      )

    assertEquals(
      EditorMagnifierPlacement(
        sourceCenter = Offset(260f, 300f),
        magnifierCenter = Offset(260f, 200f),
        topLeft = Offset(188f, 160f),
      ),
      placement,
    )
  }

  @Test
  fun `magnifier clamps horizontally and moves below near top edge`() {
    val placement =
      resolveEditorMagnifierPlacement(
        focalPosition = Offset(390f, 50f),
        overlaySize = Size(width = 400f, height = 700f),
        visibleArea = EditorVisibleArea(viewport = Size(width = 400f, height = 700f)),
        density = 1f,
      )

    assertEquals(
      EditorMagnifierPlacement(
        sourceCenter = Offset(390f, 50f),
        magnifierCenter = Offset(328f, 150f),
        topLeft = Offset(256f, 110f),
      ),
      placement,
    )
  }
}
