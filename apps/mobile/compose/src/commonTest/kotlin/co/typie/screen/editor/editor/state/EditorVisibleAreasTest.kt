package co.typie.screen.editor.editor.state

import androidx.compose.ui.geometry.Size
import co.typie.editor.viewport.EditorViewportState
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorVisibleAreasTest {
  @Test
  fun `overlay occlusion affects editor and spacer areas without affecting base area`() {
    val viewportState = EditorViewportState()
    viewportState.updateMeasuredBounds(
      viewportSize = Size(width = 320f, height = 800f),
      contentSize = Size(width = 320f, height = 1200f),
    )
    val state = EditorScreenState(viewportState)

    val areas =
      state.resolveEditorVisibleAreas(
        topInset = 20f,
        rawBottomSafeInset = 10f,
        rawEditorInputBottomInset = 100f,
        rawSubPaneBottomInset = 0f,
        overlayOcclusion =
          EditorOverlayOcclusion(top = 30f, bottom = 80f, bottomScrollReserve = 160f),
      )

    assertEquals(20f, areas.base.visibleViewportTop)
    assertEquals(100f, areas.base.bottomOcclusion)

    assertEquals(50f, areas.editor.visibleViewportTop)
    assertEquals(180f, areas.editor.bottomOcclusion)

    assertEquals(50f, areas.bottomSpacer.visibleViewportTop)
    assertEquals(260f, areas.bottomSpacer.bottomOcclusion)
  }
}
