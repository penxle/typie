package co.typie.screen.editor.editor.toolbar.contextual

import kotlin.test.Test
import kotlin.test.assertEquals

class TextOptionModeTest {
  @Test
  fun `toolbarFontWeightLabel uses numeric fallback for available unnamed weight`() {
    assertEquals("950", toolbarFontWeightLabel(950))
  }

  @Test
  fun `toolbarFontWeightLabel uses unknown fallback for unavailable weight`() {
    assertEquals("(알 수 없는 굵기)", toolbarFontWeightLabel(weight = 950, available = false))
  }

  @Test
  fun `toolbarFontWeightLabel uses unknown fallback for unavailable standard weight`() {
    assertEquals("(알 수 없는 굵기)", toolbarFontWeightLabel(weight = 900, available = false))
  }
}
