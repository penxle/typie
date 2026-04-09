package co.typie.ext

import androidx.compose.ui.geometry.Rect
import kotlin.test.Test
import kotlin.test.assertEquals

class EdgeAutoScrollTest {
  @Test
  fun `insetViewportRect applies top and bottom viewport insets`() {
    assertEquals(
      Rect(left = 24f, top = 331f, right = 846f, bottom = 3260f),
      insetViewportRect(
        viewportRect = Rect(left = 24f, top = 283f, right = 846f, bottom = 3404f),
        topInsetPx = 48f,
        bottomInsetPx = 144f,
      ),
    )
  }
}
