package co.typie.screen.editor.editor.toolbar

import co.typie.screen.editor.editor.toolbar.contextual.TextOptionMode
import kotlin.test.Test
import kotlin.test.assertEquals

class ToolbarSecondaryStateTest {
  @Test
  fun image_resize_secondary_toggles_for_same_node() {
    val state = EditorToolbarSessionState()
    val scope = EditorToolbarScope(EditorToolbarPageKey.Image, "image-1")

    state.toggleSecondaryToolbar(EditorToolbarSecondary.ImageResize(nodeId = "image-1"), scope)
    assertEquals(
      EditorToolbarSecondary.ImageResize(nodeId = "image-1"),
      state.activeSecondaryToolbar,
    )

    state.toggleSecondaryToolbar(EditorToolbarSecondary.ImageResize(nodeId = "image-1"), scope)
    assertEquals(null, state.activeSecondaryToolbar)
  }

  @Test
  fun text_and_image_secondary_states_are_mutually_exclusive() {
    val state = EditorToolbarSessionState()

    state.toggleSecondaryToolbar(
      EditorToolbarSecondary.TextOption(TextOptionMode.FontSize),
      EditorToolbarScope(EditorToolbarPageKey.Text),
    )
    state.toggleSecondaryToolbar(
      EditorToolbarSecondary.ImageResize(nodeId = "image-1"),
      EditorToolbarScope(EditorToolbarPageKey.Image, "image-1"),
    )

    assertEquals(
      EditorToolbarSecondary.ImageResize(nodeId = "image-1"),
      state.activeSecondaryToolbar,
    )
    assertEquals(null, state.activeTextOptionMode)
  }

  @Test
  fun image_resize_secondary_closes_when_page_or_node_no_longer_matches() {
    val state = EditorToolbarSessionState()

    state.toggleSecondaryToolbar(
      EditorToolbarSecondary.ImageResize(nodeId = "image-1"),
      EditorToolbarScope(EditorToolbarPageKey.Image, "image-1"),
    )
    state.clearSecondaryToolbarIfInvalid(EditorToolbarScope(EditorToolbarPageKey.Image, "image-2"))
    assertEquals(null, state.activeSecondaryToolbar)

    state.toggleSecondaryToolbar(
      EditorToolbarSecondary.ImageResize(nodeId = "image-1"),
      EditorToolbarScope(EditorToolbarPageKey.Image, "image-1"),
    )
    state.clearSecondaryToolbarIfInvalid(EditorToolbarScope(EditorToolbarPageKey.Text))
    assertEquals(null, state.activeSecondaryToolbar)
  }

  @Test
  fun secondary_toolbar_stays_open_when_page_and_owner_match() {
    val state = EditorToolbarSessionState()

    state.toggleSecondaryToolbar(
      EditorToolbarSecondary.ImageResize(nodeId = "image-1"),
      EditorToolbarScope(EditorToolbarPageKey.Image, "image-1"),
    )
    state.clearSecondaryToolbarIfInvalid(EditorToolbarScope(EditorToolbarPageKey.Image, "image-1"))
    assertEquals(
      EditorToolbarSecondary.ImageResize(nodeId = "image-1"),
      state.activeSecondaryToolbar,
    )

    state.toggleSecondaryToolbar(
      EditorToolbarSecondary.TextOption(TextOptionMode.FontSize),
      EditorToolbarScope(EditorToolbarPageKey.Text),
    )
    state.clearSecondaryToolbarIfInvalid(EditorToolbarScope(EditorToolbarPageKey.Text))
    assertEquals(
      EditorToolbarSecondary.TextOption(TextOptionMode.FontSize),
      state.activeSecondaryToolbar,
    )
  }

  @Test
  fun text_secondary_closes_when_page_is_not_text() {
    val state = EditorToolbarSessionState()

    state.toggleSecondaryToolbar(
      EditorToolbarSecondary.TextOption(TextOptionMode.FontSize),
      EditorToolbarScope(EditorToolbarPageKey.Text),
    )
    state.clearSecondaryToolbarIfInvalid(EditorToolbarScope(EditorToolbarPageKey.Image, "image-1"))

    assertEquals(null, state.activeSecondaryToolbar)
  }

  @Test
  fun table_alignment_secondary_uses_table_owner() {
    val state = EditorToolbarSessionState()

    state.toggleSecondaryToolbar(
      EditorToolbarSecondary.TableAlignment(tableId = "table-1"),
      EditorToolbarScope(EditorToolbarPageKey.Table, "table-1"),
    )
    state.clearSecondaryToolbarIfInvalid(EditorToolbarScope(EditorToolbarPageKey.Table, "table-1"))
    assertEquals(
      EditorToolbarSecondary.TableAlignment(tableId = "table-1"),
      state.activeSecondaryToolbar,
    )

    state.clearSecondaryToolbarIfInvalid(EditorToolbarScope(EditorToolbarPageKey.Table, "table-2"))
    assertEquals(null, state.activeSecondaryToolbar)
  }

  @Test
  fun table_cell_background_secondary_uses_table_owner() {
    val state = EditorToolbarSessionState()

    state.toggleSecondaryToolbar(
      EditorToolbarSecondary.TableCellBackground(tableId = "table-1"),
      EditorToolbarScope(EditorToolbarPageKey.Table, "table-1"),
    )
    state.clearSecondaryToolbarIfInvalid(EditorToolbarScope(EditorToolbarPageKey.Table, "table-1"))
    assertEquals(
      EditorToolbarSecondary.TableCellBackground(tableId = "table-1"),
      state.activeSecondaryToolbar,
    )

    state.clearSecondaryToolbarIfInvalid(EditorToolbarScope(EditorToolbarPageKey.Table, "table-2"))
    assertEquals(null, state.activeSecondaryToolbar)
  }
}
