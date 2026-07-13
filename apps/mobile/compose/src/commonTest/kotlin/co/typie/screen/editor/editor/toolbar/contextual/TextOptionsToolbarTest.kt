package co.typie.screen.editor.editor.toolbar.contextual

import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Modifier as EditorModifier
import co.typie.editor.ffi.ModifierOp
import co.typie.editor.ffi.ModifierType
import kotlin.test.Test
import kotlin.test.assertEquals

class TextOptionsToolbarTest {
  @Test
  fun text_background_color_uses_set_for_colors() {
    assertEquals(
      Message.Modifier(ModifierOp.Set(EditorModifier.BackgroundColor("yellow"))),
      textBackgroundColorMessage("yellow"),
    )
  }

  @Test
  fun text_background_none_uses_edit_removal() {
    assertEquals(
      Message.Modifier(
        ModifierOp.Edit(modifierType = ModifierType.BackgroundColor, modifier = null)
      ),
      textBackgroundColorMessage("none"),
    )
  }
}
