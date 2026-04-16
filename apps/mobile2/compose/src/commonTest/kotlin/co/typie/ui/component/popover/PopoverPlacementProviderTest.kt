package co.typie.ui.component.popover

import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.IntSize
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class PopoverPlacementProviderTest {

  private val windowSize = IntSize(390, 844)
  private val screenPadding = PopoverScreenPadding(left = 48, top = 48, right = 48, bottom = 48)

  private val topAnchor = IntRect(16, 100, 116, 144)
  private val bottomAnchor = IntRect(16, 700, 116, 744)
  private val centerAnchor = IntRect(145, 400, 245, 444)

  private val popupSize = IntSize(200, 300)

  private fun calculate(
    anchorBounds: IntRect,
    placement: PopoverPlacement,
    popupContentSize: IntSize = popupSize,
  ): IntOffset {
    return resolvePopoverGeometry(
        anchorBounds = anchorBounds,
        windowSize = windowSize,
        placement = placement,
        popupContentSize = popupContentSize,
        screenPadding = screenPadding,
      )
      .popupOffset
  }

  private fun resolveGeometry(
    anchorBounds: IntRect,
    placement: PopoverPlacement,
    popupContentSize: IntSize = popupSize,
  ): ResolvedPopoverGeometry {
    return resolvePopoverGeometry(
      anchorBounds = anchorBounds,
      windowSize = windowSize,
      placement = placement,
      popupContentSize = popupContentSize,
      screenPadding = screenPadding,
    )
  }

  @Test
  fun shouldShowBelow_prefersBottom_enoughSpace() {
    assertTrue(shouldShowBelow(PopoverPlacement.BelowEnd, 200, 844, topAnchor, screenPadding))
  }

  @Test
  fun shouldShowBelow_prefersBottom_notEnoughBottom_flipsToTop() {
    assertFalse(shouldShowBelow(PopoverPlacement.BelowEnd, 300, 844, bottomAnchor, screenPadding))
  }

  @Test
  fun shouldShowBelow_prefersTop_enoughSpace() {
    assertFalse(shouldShowBelow(PopoverPlacement.AboveEnd, 300, 844, centerAnchor, screenPadding))
  }

  @Test
  fun shouldShowBelow_prefersTop_notEnoughTop_flipsToBottom() {
    assertTrue(shouldShowBelow(PopoverPlacement.AboveEnd, 300, 844, topAnchor, screenPadding))
  }

  @Test
  fun belowEnd_positionsBelow() {
    val offset = calculate(topAnchor, PopoverPlacement.BelowEnd)
    assertEquals(100, offset.y)
  }

  @Test
  fun belowStart_anchorLeft() {
    val offset = calculate(topAnchor, PopoverPlacement.BelowStart)
    assertEquals(IntOffset(16, 100), offset)
  }

  @Test
  fun belowCenter_centeredOnAnchor() {
    val offset = calculate(centerAnchor, PopoverPlacement.BelowCenter)
    assertEquals(IntOffset(95, 400), offset)
  }

  @Test
  fun aboveEnd_positionsAbove() {
    val offset = calculate(centerAnchor, PopoverPlacement.AboveEnd)
    assertEquals(144, offset.y)
  }

  @Test
  fun belowStart_keepsAnchorLeft_whenPopupExceedsScreen() {
    val rightAnchor = IntRect(300, 100, 370, 144)
    val offset = calculate(rightAnchor, PopoverPlacement.BelowStart)
    assertEquals(300, offset.x)
  }

  @Test
  fun resolvePopoverGeometry_tracksActualAnchorRect_whenCenteredPopupClampsToRightEdge() {
    val rightAnchor = IntRect(320, 100, 360, 144)

    val geometry = resolveGeometry(rightAnchor, PopoverPlacement.BelowCenter)

    assertEquals(IntOffset(142, 100), geometry.popupOffset)
    assertEquals(PopoverPlacement.BelowCenter, geometry.placement)
    assertEquals(IntRect(178, 0, 218, 44), geometry.anchorBoundsInPopup)
  }
}
