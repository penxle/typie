package co.typie.screen.editor.editor.layout

import kotlin.test.Test
import kotlin.test.assertEquals

class EditorScreenLayoutTest {
  @Test
  fun `toolbar viewport inset excludes floating indicator but includes toolbar bottom padding`() {
    assertEquals(
      56,
      resolveEditorToolbarViewportInsetHeight(
        toolbarHeightPx = 96,
        floatingOverhangPx = 40,
        maxToolbarViewportInsetPx = 56,
      ),
    )
  }

  @Test
  fun `toolbar viewport inset ignores bottom panel height because visible area applies that inset`() {
    assertEquals(
      56,
      resolveEditorToolbarViewportInsetHeight(
        toolbarHeightPx = 396,
        floatingOverhangPx = 40,
        maxToolbarViewportInsetPx = 56,
      ),
    )
  }
}
