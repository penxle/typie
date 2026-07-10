package co.typie.screen.editor.editor.toolbar.contextual

import co.typie.editor.ffi.BackgroundColorValue
import co.typie.editor.ffi.TextColorValue
import co.typie.editor.ffi.Tri
import kotlin.test.Test
import kotlin.test.assertEquals

class TextOptionModeTest {
  @Test
  fun `textColorCurrentValue maps uniform to its value`() {
    assertEquals("red", Tri.Uniform(TextColorValue("red")).textColorCurrentValue())
  }

  @Test
  fun `textColorCurrentValue maps absent to black`() {
    assertEquals("black", Tri.Absent.textColorCurrentValue())
  }

  @Test
  fun `textColorCurrentValue maps mixed and null to null`() {
    assertEquals(null, Tri.Mixed.textColorCurrentValue())
    assertEquals(null, (null as Tri<TextColorValue>?).textColorCurrentValue())
  }

  @Test
  fun `backgroundColorCurrentValue maps uniform to its value`() {
    assertEquals(
      "yellow",
      Tri.Uniform(BackgroundColorValue("yellow")).backgroundColorCurrentValue(),
    )
  }

  @Test
  fun `backgroundColorCurrentValue maps absent to none`() {
    assertEquals("none", Tri.Absent.backgroundColorCurrentValue())
  }

  @Test
  fun `backgroundColorCurrentValue maps mixed and null to null`() {
    assertEquals(null, Tri.Mixed.backgroundColorCurrentValue())
    assertEquals(null, (null as Tri<BackgroundColorValue>?).backgroundColorCurrentValue())
  }

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
