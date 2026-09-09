package co.typie.editor

import co.typie.editor.ffi.ThemeVariant
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class EditorThemeTest {
  @Test
  fun default_theme_keys_remain_stable() {
    assertEquals("light-white", ThemeVariant.LightWhite.key)
    assertEquals("dark-black", ThemeVariant.DarkBlack.key)
  }

  @Test
  fun every_generated_variant_resolves_bundled_editor_colors() {
    for (variant in ThemeVariant.entries) {
      val theme = EditorTheme.resolve(variant)
      assertNotNull(theme["text.red"], variant.key)
      assertNotNull(theme["ui.text.default"], variant.key)
      assertNotNull(theme["selection"], variant.key)
    }
  }
}
