package co.typie.screen.editor.editor.toolbar

import co.typie.screen.editor.editor.toolbar.contextual.TextOptionMode
import kotlin.test.Test
import kotlin.test.assertEquals

class ToolbarSecondaryStateTest {
  @Test
  fun image_resize_secondary_toggles_for_same_node() {
    val state = EditorToolbarSessionState()

    state.toggleSecondaryToolbar(EditorToolbarSecondary.ImageResize(nodeId = "image-1"))
    assertEquals(
      EditorToolbarSecondary.ImageResize(nodeId = "image-1"),
      state.activeSecondaryToolbar,
    )

    state.toggleSecondaryToolbar(EditorToolbarSecondary.ImageResize(nodeId = "image-1"))
    assertEquals(null, state.activeSecondaryToolbar)
  }

  @Test
  fun text_and_image_secondary_states_are_mutually_exclusive() {
    val state = EditorToolbarSessionState()

    state.activeTextOptionMode = TextOptionMode.FontSize
    state.toggleSecondaryToolbar(EditorToolbarSecondary.ImageResize(nodeId = "image-1"))

    assertEquals(
      EditorToolbarSecondary.ImageResize(nodeId = "image-1"),
      state.activeSecondaryToolbar,
    )
    assertEquals(null, state.activeTextOptionMode)
  }

  @Test
  fun image_resize_secondary_closes_when_page_or_node_no_longer_matches() {
    val state = EditorToolbarSessionState()

    state.toggleSecondaryToolbar(EditorToolbarSecondary.ImageResize(nodeId = "image-1"))
    state.clearSecondaryToolbarIfInvalid(
      currentPageKey = EditorToolbarPageKey.Image,
      selectedNodeId = "image-2",
    )
    assertEquals(null, state.activeSecondaryToolbar)

    state.toggleSecondaryToolbar(EditorToolbarSecondary.ImageResize(nodeId = "image-1"))
    state.clearSecondaryToolbarIfInvalid(
      currentPageKey = EditorToolbarPageKey.Text,
      selectedNodeId = "image-1",
    )
    assertEquals(null, state.activeSecondaryToolbar)
  }

  @Test
  fun secondary_toolbar_stays_open_when_page_and_owner_match() {
    val state = EditorToolbarSessionState()

    state.toggleSecondaryToolbar(EditorToolbarSecondary.ImageResize(nodeId = "image-1"))
    state.clearSecondaryToolbarIfInvalid(
      currentPageKey = EditorToolbarPageKey.Image,
      selectedNodeId = "image-1",
    )
    assertEquals(
      EditorToolbarSecondary.ImageResize(nodeId = "image-1"),
      state.activeSecondaryToolbar,
    )

    state.activeTextOptionMode = TextOptionMode.FontSize
    state.clearSecondaryToolbarIfInvalid(
      currentPageKey = EditorToolbarPageKey.Text,
      selectedNodeId = null,
    )
    assertEquals(
      EditorToolbarSecondary.TextOption(TextOptionMode.FontSize),
      state.activeSecondaryToolbar,
    )
  }

  @Test
  fun text_secondary_closes_when_page_is_not_text() {
    val state = EditorToolbarSessionState()

    state.activeTextOptionMode = TextOptionMode.FontSize
    state.clearSecondaryToolbarIfInvalid(
      currentPageKey = EditorToolbarPageKey.Image,
      selectedNodeId = "image-1",
    )

    assertEquals(null, state.activeSecondaryToolbar)
  }
}
