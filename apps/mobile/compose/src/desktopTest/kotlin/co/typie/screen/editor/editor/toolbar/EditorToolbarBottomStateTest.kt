package co.typie.screen.editor.editor.toolbar

import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorToolbarBottomStateTest {
  @Test
  fun rapidReopenKeepsRememberedKeyboardInsetFromShrinking() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 20.dp,
    )
    state.closePanel()
    state.openPanel(
      panel = EditorToolbarBottomPanelKey.More,
      imeBottom = 80.dp,
      safeBottomInset = 20.dp,
    )

    assertEquals(320.dp, state.rememberedKeyboardInset)
  }

  @Test
  fun openPanelKeepsVisibleImeInsetEquivalentToRememberedKeyboardSpace() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 20.dp,
    )

    assertEquals(320.dp, state.visibleImeInset(imeBottom = 0.dp, safeBottomInset = 20.dp))
  }
}
