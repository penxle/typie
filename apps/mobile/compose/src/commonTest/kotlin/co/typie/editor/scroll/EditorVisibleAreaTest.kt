package co.typie.editor.scroll

import androidx.compose.ui.geometry.Size
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorVisibleAreaTest {
  @Test
  fun `visible viewport bottom follows bottom safe area when keyboard is closed`() {
    val visibleArea =
      EditorVisibleArea(
        viewport = Size(width = 720f, height = 900f),
        topInset = 120f,
        safeBottomInset = 34f,
      )

    assertEquals(866f, visibleArea.visibleViewportBottom)
    assertEquals(34f, visibleArea.bottomOcclusion)
    assertEquals(Size(width = 720f, height = 746f), visibleArea.visibleBodySize)
  }

  @Test
  fun `visible viewport bottom follows explicit bottom occlusion`() {
    val visibleArea =
      EditorVisibleArea(
        viewport = Size(width = 720f, height = 900f),
        topInset = 120f,
        bottomOcclusionInset = 80f,
      )

    assertEquals(120f, visibleArea.visibleViewportTop)
    assertEquals(820f, visibleArea.visibleViewportBottom)
    assertEquals(80f, visibleArea.bottomOcclusion)
    assertEquals(Size(width = 720f, height = 700f), visibleArea.visibleBodySize)
  }

  @Test
  fun `visible viewport bottom follows keyboard inset`() {
    val visibleArea =
      EditorVisibleArea(
        viewport = Size(width = 720f, height = 900f),
        topInset = 120f,
        imeInset = 240f,
      )

    assertEquals(660f, visibleArea.visibleViewportBottom)
    assertEquals(240f, visibleArea.bottomOcclusion)
    assertEquals(Size(width = 720f, height = 540f), visibleArea.visibleBodySize)
  }
}
