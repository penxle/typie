package co.typie.ui.component.sheet

import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals

class SheetOverlayHostTest {
  @Test
  fun downwardDragCollapsesToMinHeightBeforeAddingOffset() {
    val nextState = consumeSheetDragDelta(
      currentHeightPx = 720f,
      currentOffsetPx = 0f,
      delta = 420f,
      minHeightPx = 480f,
      maxHeightPx = 820f,
    )

    assertEquals(480f, nextState.heightPx)
    assertEquals(180f, nextState.offsetPx)
  }

  @Test
  fun upwardDragRecoversOffsetBeforeExpandingHeight() {
    val nextState = consumeSheetDragDelta(
      currentHeightPx = 480f,
      currentOffsetPx = 180f,
      delta = -260f,
      minHeightPx = 480f,
      maxHeightPx = 820f,
    )

    assertEquals(560f, nextState.heightPx)
    assertEquals(0f, nextState.offsetPx)
  }

  @Test
  fun upwardScrollConsumptionAccountsForOffsetRecoveryAndHeightExpansion() {
    val nextState = consumeSheetDragDelta(
      currentHeightPx = 480f,
      currentOffsetPx = 100f,
      delta = -200f,
      minHeightPx = 480f,
      maxHeightPx = 720f,
    )

    assertEquals(580f, nextState.heightPx)
    assertEquals(0f, nextState.offsetPx)
    assertEquals(
      -200f,
      resolveConsumedSheetScrollDeltaY(
        currentHeightPx = 480f,
        currentOffsetPx = 100f,
        nextState = nextState,
      ),
    )
  }

  @Test
  fun sheetOffsetCombinesVisibilityAndDragOffset() {
    assertEquals(
      300,
      resolveSheetOffsetY(
        progress = 0.5f,
        renderedSheetHeightPx = 480f,
        dragOffsetPx = 60f,
      ),
    )
  }

  @Test
  fun sheetVisibleFractionAccountsForDragOffset() {
    assertEquals(
      0.375f,
      resolveSheetVisibleFraction(
        progress = 0.5f,
        renderedSheetHeightPx = 480f,
        dragOffsetPx = 60f,
      ),
    )
  }

  @Test
  fun sheetVisibleFractionDropsToZeroWhenDraggedFullyOffscreen() {
    assertEquals(
      0f,
      resolveSheetVisibleFraction(
        progress = 1f,
        renderedSheetHeightPx = 480f,
        dragOffsetPx = 480f,
      ),
    )
  }

  @Test
  fun intrinsicDetentsStayUnresolvedUntilMeasuredContentIsAvailable() {
    val detents = resolveDetentsForSheetMeasurement(
      policy = SheetSizePolicy.Intrinsic(),
      viewportHeight = 640.dp,
      measuredSheetHeightPx = 0f,
      density = Density(density = 2f, fontScale = 1f),
    )

    assertEquals(emptyList(), detents)
  }

  @Test
  fun fixedInitialDetentRemainsResolvableBeforeMeasuredContentArrives() {
    val detents = resolveDetentsForSheetMeasurement(
      policy = SheetSizePolicy.Detents(
        initial = SheetDetent.Fixed(360.dp),
        available = listOf(
          SheetDetent.Fixed(360.dp),
          SheetDetent.Content(maxTopGap = 64.dp),
        ),
      ),
      viewportHeight = 800.dp,
      measuredSheetHeightPx = 0f,
      density = Density(density = 2f, fontScale = 1f),
    )

    assertEquals(
      listOf(
        ResolvedSheetDetent(
          id = SheetDetentId.Fixed(360.dp),
          height = 360.dp,
        ),
      ),
      detents,
    )
  }

  @Test
  fun constrainedMeasuredSheetDetectionRequiresVisibleHeightToBeSmallerThanNaturalHeight() {
    assertEquals(
      true,
      shouldTreatMeasuredSheetAsConstrained(
        requiresContentMeasurement = true,
        measuredSheetHeightPx = 480f,
        renderedSheetHeightPx = 320f,
      ),
    )

    assertEquals(
      false,
      shouldTreatMeasuredSheetAsConstrained(
        requiresContentMeasurement = true,
        measuredSheetHeightPx = 480f,
        renderedSheetHeightPx = 480f,
      ),
    )
  }

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
