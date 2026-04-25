package co.typie.editor.input

import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class EditorInputKeyHandlingTest {
  @Test
  fun `iOS printable text is owned by platform text input`() {
    assertFalse(requiresRawKeyTextFallback(Platform.iOS))
  }

  @Test
  fun `non iOS platforms keep raw key text fallback`() {
    assertTrue(requiresRawKeyTextFallback(Platform.Android))
    assertTrue(requiresRawKeyTextFallback(Platform.Desktop))
  }
}
