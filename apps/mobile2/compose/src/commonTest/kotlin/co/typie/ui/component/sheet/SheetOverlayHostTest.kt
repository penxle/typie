package co.typie.ui.component.sheet

import androidx.compose.ui.unit.Density
import kotlin.test.Test
import kotlin.test.assertEquals
import androidx.compose.ui.unit.dp

class SheetOverlayHostTest {

  @Test
  fun initialSheetHeightUsesPhysicalPixelsForInitialDetent() {
    val initialDetent = ResolvedSheetDetent(
      id = SheetDetentId.Fixed(360.dp),
      height = 360.dp,
    )

    val initialHeight = resolveInitialSheetHeightPx(
      density = Density(density = 3f, fontScale = 1f),
      requiresContentMeasurement = false,
      initialResolvedDetent = initialDetent,
    )

    assertEquals(1080f, initialHeight)
  }

  @Test
  fun initialSheetHeightStaysHiddenUntilContentMeasuresWhenRequired() {
    val initialDetent = ResolvedSheetDetent(
      id = SheetDetentId.Intrinsic,
      height = 0.dp,
    )

    val initialHeight = resolveInitialSheetHeightPx(
      density = Density(density = 3f, fontScale = 1f),
      requiresContentMeasurement = true,
      initialResolvedDetent = initialDetent,
    )

    assertEquals(0f, initialHeight)
  }
}
