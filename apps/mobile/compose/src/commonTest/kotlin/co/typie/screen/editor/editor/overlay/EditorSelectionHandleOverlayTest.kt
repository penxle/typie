package co.typie.screen.editor.editor.overlay

import androidx.compose.ui.geometry.Offset
import co.typie.editor.interaction.gestures.EditorSelectionHandleType
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorSelectionHandleOverlayTest {
  @Test
  fun `from handle hit target and paint offsets match legacy selection handle`() {
    val geometry =
      resolveSelectionHandleGeometry(
        type = EditorSelectionHandleType.From,
        endpointTopLeftInOverlay = Offset(100f, 200f),
        stemHeightPx = 8f,
        radiusPx = 8f,
        stemWidthPx = 2f,
        touchTargetPx = 44f,
      )

    assertEquals(Offset(77f, 174f), geometry.touchTargetTopLeft)
    assertEquals(44f, geometry.touchTargetSize.width)
    assertEquals(44f, geometry.touchTargetSize.height)
    assertEquals(Offset(14f, 10f), geometry.paintTopLeftInTouchTarget)
  }

  @Test
  fun `to handle hit target and paint offsets match legacy selection handle`() {
    val geometry =
      resolveSelectionHandleGeometry(
        type = EditorSelectionHandleType.To,
        endpointTopLeftInOverlay = Offset(100f, 200f),
        stemHeightPx = 8f,
        radiusPx = 8f,
        stemWidthPx = 2f,
        touchTargetPx = 44f,
      )

    assertEquals(Offset(79f, 190f), geometry.touchTargetTopLeft)
    assertEquals(44f, geometry.touchTargetSize.width)
    assertEquals(44f, geometry.touchTargetSize.height)
    assertEquals(Offset(14f, 10f), geometry.paintTopLeftInTouchTarget)
  }
}
