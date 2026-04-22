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
  fun `visible viewport bottom uses the higher occlusion between keyboard and toolbar`() {
    val visibleArea =
      EditorVisibleArea(
        viewport = Size(width = 720f, height = 900f),
        topInset = 120f,
        imeInset = 80f,
        toolbarTop = 756f,
      )

    assertEquals(120f, visibleArea.visibleViewportTop)
    assertEquals(756f, visibleArea.visibleViewportBottom)
    assertEquals(144f, visibleArea.bottomOcclusion)
    assertEquals(Size(width = 720f, height = 636f), visibleArea.visibleBodySize)
  }

  @Test
  fun `visible viewport bottom follows keyboard when it occludes more than the toolbar`() {
    val visibleArea =
      EditorVisibleArea(
        viewport = Size(width = 720f, height = 900f),
        topInset = 120f,
        imeInset = 240f,
        toolbarTop = 756f,
      )

    assertEquals(660f, visibleArea.visibleViewportBottom)
    assertEquals(240f, visibleArea.bottomOcclusion)
    assertEquals(Size(width = 720f, height = 540f), visibleArea.visibleExtensionSize)
  }

  @Test
  fun `visible editor viewport top is clamped below the top inset`() {
    val visibleArea =
      EditorVisibleArea(viewport = Size(width = 720f, height = 900f), topInset = 120f)

    assertEquals(120f, visibleArea.resolveVisibleEditorViewportTop(editorTopInViewport = 40f))
    assertEquals(180f, visibleArea.resolveVisibleEditorViewportTop(editorTopInViewport = 180f))
  }
}
