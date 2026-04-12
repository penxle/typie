package co.typie.ui.component.sheet

import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals

class SheetOverlayHostTest {
  @Test
  fun boundaryHandoffIsAttemptedOnlyAtDismissBoundary() {
    assertEquals(false, shouldAttemptSheetBoundaryHandoff(2400f, isAtDismissBoundary = false))
    assertEquals(false, shouldAttemptSheetBoundaryHandoff(0f, isAtDismissBoundary = true))
    assertEquals(true, shouldAttemptSheetBoundaryHandoff(2400f, isAtDismissBoundary = true))
  }

  @Test
  fun nestedChildFlingHandoffRequiresStrongDownwardVelocity() {
    assertEquals(false, shouldHandOffSheetNestedChildFlingToSheet(1800f))
    assertEquals(false, shouldHandOffSheetNestedChildFlingToSheet(-2400f))
    assertEquals(true, shouldHandOffSheetNestedChildFlingToSheet(2400f))
  }

  @Test
  fun nestedSheetPreFlingIsHandledImmediatelyWhenDraggedAwayFromCurrentDetent() {
    assertEquals(
      true,
      shouldHandleSheetNestedPreFling(
        currentDetentHeightPx = 672f,
        sheetHeightPx = 500f,
        dragOffsetPx = 0f,
      ),
    )
    assertEquals(
      true,
      shouldHandleSheetNestedPreFling(
        currentDetentHeightPx = 672f,
        sheetHeightPx = 672f,
        dragOffsetPx = 72f,
      ),
    )
  }

  @Test
  fun nestedSheetPreFlingIsIgnoredWhenAlreadySettledAtDetent() {
    assertEquals(
      false,
      shouldHandleSheetNestedPreFling(
        currentDetentHeightPx = 672f,
        sheetHeightPx = 672.25f,
        dragOffsetPx = 0.25f,
      ),
    )
  }

  @Test
  fun downwardDragCollapsesToMinHeightBeforeAddingOffset() {
    val nextState =
      consumeSheetDragDelta(
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
    val nextState =
      consumeSheetDragDelta(
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
    val nextState =
      consumeSheetDragDelta(
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
  fun reversingAfterOvershootingMaxHeightRecoversOverflowBeforeCollapsing() {
    val overshotState =
      consumeSheetDragDelta(
        currentHeightPx = 820f,
        currentOffsetPx = 0f,
        delta = -120f,
        minHeightPx = 480f,
        maxHeightPx = 820f,
        trackUpperBoundaryOverflow = true,
      )

    assertEquals(820f, overshotState.heightPx)
    assertEquals(-120f, overshotState.offsetPx)

    val recoveredState =
      consumeSheetDragDelta(
        currentHeightPx = overshotState.heightPx,
        currentOffsetPx = overshotState.offsetPx,
        delta = 80f,
        minHeightPx = 480f,
        maxHeightPx = 820f,
        trackUpperBoundaryOverflow = true,
      )

    assertEquals(820f, recoveredState.heightPx)
    assertEquals(-40f, recoveredState.offsetPx)
  }

  @Test
  fun downwardDragRecoversExistingNegativeOverflowBeforeCollapsing() {
    val nextState =
      consumeSheetDragDelta(
        currentHeightPx = 820f,
        currentOffsetPx = -120f,
        delta = 80f,
        minHeightPx = 480f,
        maxHeightPx = 820f,
      )

    assertEquals(820f, nextState.heightPx)
    assertEquals(-40f, nextState.offsetPx)
  }

  @Test
  fun nestedScrollUpwardOverflowIsLeftForChildElasticOverscroll() {
    val preScrollState =
      consumeSheetDragDelta(
        currentHeightPx = 760f,
        currentOffsetPx = 0f,
        delta = -100f,
        minHeightPx = 480f,
        maxHeightPx = 820f,
      )
    val postScrollOverflowState =
      consumeSheetDragDelta(
        currentHeightPx = preScrollState.heightPx,
        currentOffsetPx = preScrollState.offsetPx,
        delta = -40f,
        minHeightPx = 480f,
        maxHeightPx = 820f,
      )
    val consumedY =
      resolveConsumedSheetScrollDeltaY(
        currentHeightPx = preScrollState.heightPx,
        currentOffsetPx = preScrollState.offsetPx,
        nextState = postScrollOverflowState,
      )

    assertEquals(820f, preScrollState.heightPx)
    assertEquals(0f, preScrollState.offsetPx)
    assertEquals(820f, postScrollOverflowState.heightPx)
    assertEquals(0f, postScrollOverflowState.offsetPx)
    assertEquals(0f, consumedY)
  }

  @Test
  fun negativeDragOffsetDoesNotAffectVisibleSheetOffsetOrFraction() {
    assertEquals(
      240,
      resolveSheetOffsetY(progress = 0.5f, renderedSheetHeightPx = 480f, dragOffsetPx = -60f),
    )
    assertEquals(
      0.5f,
      resolveSheetVisibleFraction(
        progress = 0.5f,
        renderedSheetHeightPx = 480f,
        dragOffsetPx = -60f,
      ),
    )
  }

  @Test
  fun sheetOffsetCombinesVisibilityAndDragOffset() {
    assertEquals(
      300,
      resolveSheetOffsetY(progress = 0.5f, renderedSheetHeightPx = 480f, dragOffsetPx = 60f),
    )
  }

  @Test
  fun sheetVisibleFractionAccountsForDragOffset() {
    assertEquals(
      0.375f,
      resolveSheetVisibleFraction(progress = 0.5f, renderedSheetHeightPx = 480f, dragOffsetPx = 60f),
    )
  }

  @Test
  fun sheetVisibleFractionDropsToZeroWhenDraggedFullyOffscreen() {
    assertEquals(
      0f,
      resolveSheetVisibleFraction(progress = 1f, renderedSheetHeightPx = 480f, dragOffsetPx = 480f),
    )
  }

  @Test
  fun intrinsicDetentsStayUnresolvedUntilMeasuredContentIsAvailable() {
    val detents =
      resolveDetentsForSheetMeasurement(
        policy = SheetSizePolicy.Intrinsic(),
        viewportHeight = 640.dp,
        measuredSheetHeightPx = 0f,
        density = Density(density = 2f, fontScale = 1f),
      )

    assertEquals(emptyList(), detents)
  }

  @Test
  fun fixedInitialDetentRemainsResolvableBeforeMeasuredContentArrives() {
    val detents =
      resolveDetentsForSheetMeasurement(
        policy =
          SheetSizePolicy.Detents(
            initial = SheetDetent.Fixed(360.dp),
            available = listOf(SheetDetent.Fixed(360.dp), SheetDetent.Content(maxTopGap = 64.dp)),
          ),
        viewportHeight = 800.dp,
        measuredSheetHeightPx = 0f,
        density = Density(density = 2f, fontScale = 1f),
      )

    assertEquals(
      listOf(ResolvedSheetDetent(id = SheetDetentId.Fixed(360.dp), height = 360.dp)),
      detents,
    )
  }

  @Test
  fun modalSheetFocusRequestSkipsWhenFocusIsAlreadyInsideSheet() {
    assertEquals(
      false,
      shouldRequestModalSheetFocus(
        isTopOfStack = true,
        mode = SheetMode.Modal,
        progress = 0.4f,
        sheetHasFocus = true,
      ),
    )
  }

  @Test
  fun modalSheetFocusRequestNeedsVisibleTopModalWithoutExistingFocus() {
    assertEquals(
      true,
      shouldRequestModalSheetFocus(
        isTopOfStack = true,
        mode = SheetMode.Modal,
        progress = 0.4f,
        sheetHasFocus = false,
      ),
    )
  }

  @Test
  fun modalSheetFocusRequestIgnoresHiddenOrNonModalSheets() {
    assertEquals(
      false,
      shouldRequestModalSheetFocus(
        isTopOfStack = true,
        mode = SheetMode.Modal,
        progress = 0f,
        sheetHasFocus = false,
      ),
    )
    assertEquals(
      false,
      shouldRequestModalSheetFocus(
        isTopOfStack = true,
        mode = SheetMode.NonModalOverlay,
        progress = 1f,
        sheetHasFocus = false,
      ),
    )
    assertEquals(
      false,
      shouldRequestModalSheetFocus(
        isTopOfStack = false,
        mode = SheetMode.Modal,
        progress = 1f,
        sheetHasFocus = false,
      ),
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
  fun intrinsicSheetDoesNotLockRenderedHeightAtNaturalContentSize() {
    assertEquals(
      false,
      shouldApplyRenderedSheetHeight(
        policy = SheetSizePolicy.Intrinsic(),
        targetDetentId = SheetDetentId.Intrinsic,
        requiresContentMeasurement = true,
        measuredSheetHeightPx = 480f,
        renderedSheetHeightPx = 480f,
      ),
    )
  }

  @Test
  fun contentDrivenDetentLocksRenderedHeightOnlyWhileConstrained() {
    val policy =
      SheetSizePolicy.Detents(
        initial = SheetDetent.Content(maxTopGap = 64.dp),
        available = listOf(SheetDetent.Content(maxTopGap = 64.dp), SheetDetent.Fixed(560.dp)),
      )

    assertEquals(
      false,
      shouldApplyRenderedSheetHeight(
        policy = policy,
        targetDetentId = SheetDetentId.Content(64.dp),
        requiresContentMeasurement = true,
        measuredSheetHeightPx = 480f,
        renderedSheetHeightPx = 480f,
      ),
    )

    assertEquals(
      true,
      shouldApplyRenderedSheetHeight(
        policy = policy,
        targetDetentId = SheetDetentId.Content(64.dp),
        requiresContentMeasurement = true,
        measuredSheetHeightPx = 560f,
        renderedSheetHeightPx = 480f,
      ),
    )
  }

  @Test
  fun intrinsicResizeDoesNotAnimateWhileDetentTransitionsStillDo() {
    assertEquals(
      false,
      shouldAnimateSheetHeightChange(
        policy = SheetSizePolicy.Intrinsic(),
        targetDetentId = SheetDetentId.Intrinsic,
        lastSettledDetentId = SheetDetentId.Intrinsic,
      ),
    )

    assertEquals(
      true,
      shouldAnimateSheetHeightChange(
        policy =
          SheetSizePolicy.Detents(
            initial = SheetDetent.Fixed(360.dp),
            available = listOf(SheetDetent.Fixed(360.dp), SheetDetent.Intrinsic),
          ),
        targetDetentId = SheetDetentId.Intrinsic,
        lastSettledDetentId = SheetDetentId.Fixed(360.dp),
      ),
    )
  }

  @Test
  fun intrinsicResizeUsesLatestTargetHeightInCurrentFrame() {
    assertEquals(
      560f,
      resolveRenderedSheetHeightPx(
        currentSheetHeightPx = 480f,
        requiresContentMeasurement = true,
        measuredSheetHeightPx = 560f,
        initialSheetHeightPx = 0f,
        targetSheetHeightPx = 560f,
        shouldAnimateTargetHeightChange = false,
      ),
    )
  }

  @Test
  fun initialSheetHeightUsesPhysicalPixelsForInitialDetent() {
    val initialDetent = ResolvedSheetDetent(id = SheetDetentId.Fixed(360.dp), height = 360.dp)

    val initialHeight =
      resolveInitialSheetHeightPx(
        density = Density(density = 3f, fontScale = 1f),
        requiresContentMeasurement = false,
        initialResolvedDetent = initialDetent,
      )

    assertEquals(1080f, initialHeight)
  }

  @Test
  fun initialSheetHeightStaysHiddenUntilContentMeasuresWhenRequired() {
    val initialDetent = ResolvedSheetDetent(id = SheetDetentId.Intrinsic, height = 0.dp)

    val initialHeight =
      resolveInitialSheetHeightPx(
        density = Density(density = 3f, fontScale = 1f),
        requiresContentMeasurement = true,
        initialResolvedDetent = initialDetent,
      )

    assertEquals(0f, initialHeight)
  }
}
