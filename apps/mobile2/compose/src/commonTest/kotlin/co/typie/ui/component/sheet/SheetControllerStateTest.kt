package co.typie.ui.component.sheet

import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals

class SheetControllerStateTest {

  @Test
  fun expandAndCollapseFollowResolvedDetents() {
    val controller = SheetControllerState<Unit>(
      dismissPolicy = SheetDismissPolicy(),
      mode = SheetMode.Modal,
    )

    controller.updateResolvedDetents(
      listOf(
        ResolvedSheetDetent(SheetDetentId.Fixed(280.dp), 280.dp),
        ResolvedSheetDetent(SheetDetentId.TopGap(64.dp), 736.dp),
      ),
      initialDetentId = SheetDetentId.Fixed(280.dp),
      stackDepth = 0,
      isTopOfStack = true,
    )

    assertEquals(SheetDetentId.Fixed(280.dp), controller.currentDetentId)

    controller.expand()
    assertEquals(SheetDetentId.TopGap(64.dp), controller.targetDetentId)

    controller.collapse()
    assertEquals(SheetDetentId.Fixed(280.dp), controller.targetDetentId)
  }
}
