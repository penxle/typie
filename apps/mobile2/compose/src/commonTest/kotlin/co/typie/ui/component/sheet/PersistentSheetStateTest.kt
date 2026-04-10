package co.typie.ui.component.sheet

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class PersistentSheetStateTest {

  @Test
  fun showAndHideToggleVisibility() {
    val state =
      PersistentSheetState(
        controller =
          SheetControllerState(mode = SheetMode.Persistent, dismissPolicy = SheetDismissPolicy()),
        spec = PersistentSheetSpec(),
      )

    assertTrue(state.visible)

    state.hide()
    assertFalse(state.visible)

    state.show()
    assertTrue(state.visible)
  }
}
