package co.typie.ui.component.popover

import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.LayoutDirection
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class PopoverPositionProviderTest {

  private val windowSize = IntSize(390, 844)
  private val screenPadding = 48

  private val topAnchor = IntRect(16, 100, 116, 144)
  private val bottomAnchor = IntRect(16, 700, 116, 744)
  private val centerAnchor = IntRect(145, 400, 245, 444)

  private val popupSize = IntSize(200, 300)

  private fun calculate(
    anchorBounds: IntRect,
    position: PopoverPosition,
    popupContentSize: IntSize = popupSize,
  ): IntOffset {
    val provider = PopoverPositionProvider(position, screenPadding)
    return provider.calculatePosition(
      anchorBounds,
      windowSize,
      LayoutDirection.Ltr,
      popupContentSize
    )
  }

  // --- shouldShowBelow ---

  @Test
  fun shouldShowBelow_prefersBottom_enoughSpace() {
    assertTrue(shouldShowBelow(PopoverPosition.BottomRight, 200, 844, topAnchor, screenPadding))
  }

  @Test
  fun shouldShowBelow_prefersBottom_notEnoughBottom_flipsToTop() {
    assertFalse(shouldShowBelow(PopoverPosition.BottomRight, 300, 844, bottomAnchor, screenPadding))
  }

  @Test
  fun shouldShowBelow_prefersTop_enoughSpace() {
    assertFalse(shouldShowBelow(PopoverPosition.TopRight, 300, 844, centerAnchor, screenPadding))
  }

  @Test
  fun shouldShowBelow_prefersTop_notEnoughTop_flipsToBottom() {
    assertTrue(shouldShowBelow(PopoverPosition.TopRight, 300, 844, topAnchor, screenPadding))
  }

  // --- calculatePosition ---

  @Test
  fun bottomRight_positionsBelow() {
    val offset = calculate(topAnchor, PopoverPosition.BottomRight)
    assertEquals(100, offset.y)
  }

  @Test
  fun bottomLeft_anchorLeft() {
    val offset = calculate(topAnchor, PopoverPosition.BottomLeft)
    assertEquals(IntOffset(16, 100), offset)
  }

  @Test
  fun bottomCenter_centeredOnAnchor() {
    val offset = calculate(centerAnchor, PopoverPosition.BottomCenter)
    assertEquals(IntOffset(95, 400), offset)
  }

  @Test
  fun topRight_aboveAnchor() {
    val offset = calculate(centerAnchor, PopoverPosition.TopRight)
    assertEquals(144, offset.y)
  }

  @Test
  fun clampsRight_whenPopupExceedsScreen() {
    val rightAnchor = IntRect(300, 100, 370, 144)
    val offset = calculate(rightAnchor, PopoverPosition.BottomLeft)
    assertEquals(142, offset.x)
  }
}
