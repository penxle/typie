package co.typie.ui.component.sheet

import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals

class SheetControllerTest {
  @Test
  fun updateResolvedDetentsPreservesDetentsWhenGivenCurrentSnapshot() {
    val controller = SheetControllerState<Unit>(
      mode = SheetMode.Modal,
      dismissPolicy = SheetDismissPolicy(),
    )
    val detents = listOf(
      ResolvedSheetDetent(SheetDetentId.Fixed(360.dp), 360.dp),
      ResolvedSheetDetent(SheetDetentId.TopGap(128.dp), 672.dp),
    )

    controller.updateResolvedDetents(
      detents = detents,
      initialDetentId = SheetDetentId.Fixed(360.dp),
      stackDepth = 0,
      isTopOfStack = true,
    )
    controller.updateResolvedDetents(
      detents = controller.resolvedDetents,
      initialDetentId = SheetDetentId.Fixed(360.dp),
      stackDepth = 0,
      isTopOfStack = true,
    )

    assertEquals(detents, controller.resolvedDetents)
  }
}
