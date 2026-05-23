package co.typie.screen.editor.editor.overlay

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import co.typie.editor.scroll.EditorVisibleArea
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorContextMenuPlacementTest {
  @Test
  fun `context menu stays above anchor when there is room`() {
    val placement =
      resolveEditorContextMenuPlacement(
        anchor = EditorContextMenuAnchor(centerX = 200f, above = 220f, below = 320f),
        menuSize = Size(width = 120f, height = 40f),
        overlaySize = Size(width = 400f, height = 700f),
        visibleArea = EditorVisibleArea(viewport = Size(width = 400f, height = 700f)),
        density = 1f,
      )

    assertEquals(EditorContextMenuPlacement(topLeft = Offset(140f, 180f)), placement)
  }

  @Test
  fun `context menu moves below anchor when top side has no room`() {
    val placement =
      resolveEditorContextMenuPlacement(
        anchor = EditorContextMenuAnchor(centerX = 200f, above = 24f, below = 80f),
        menuSize = Size(width = 120f, height = 40f),
        overlaySize = Size(width = 400f, height = 700f),
        visibleArea = EditorVisibleArea(viewport = Size(width = 400f, height = 700f)),
        density = 1f,
      )

    assertEquals(EditorContextMenuPlacement(topLeft = Offset(140f, 80f)), placement)
  }

  @Test
  fun `context menu centers in visible area when neither side has room`() {
    val placement =
      resolveEditorContextMenuPlacement(
        anchor = EditorContextMenuAnchor(centerX = 48f, above = 168f, below = 232f),
        menuSize = Size(width = 120f, height = 80f),
        overlaySize = Size(width = 400f, height = 400f),
        visibleArea =
          EditorVisibleArea(
            viewport = Size(width = 400f, height = 400f),
            topInset = 140f,
            safeBottomInset = 140f,
          ),
        density = 1f,
      )

    assertEquals(EditorContextMenuPlacement(topLeft = Offset(140f, 160f)), placement)
  }
}
