package co.typie.ui.component.sheet

import kotlin.test.Test
import kotlin.test.assertEquals

class SheetStopTest {

  @Test
  fun keepAllPolicyRetainsAllAnchors() {
    val anchors =
      listOf(SheetAnchor(value = 0, offset = 440f), SheetAnchor(value = 1, offset = 128f))

    assertEquals(
      anchors,
      resolveEffectiveSheetAnchors(
        anchors = anchors,
        stopPolicy = SheetStop.Policy.KeepAll,
        hasReachedTopStop = true,
      ),
    )
  }

  @Test
  fun dismissFromTopStopKeepsAllAnchorsUntilTopStopWasReached() {
    val anchors =
      listOf(SheetAnchor(value = 0, offset = 440f), SheetAnchor(value = 1, offset = 128f))

    assertEquals(
      anchors,
      resolveEffectiveSheetAnchors(
        anchors = anchors,
        stopPolicy = SheetStop.Policy.DismissFromTopStop,
        hasReachedTopStop = false,
      ),
    )
  }

  @Test
  fun dismissFromTopStopDropsLowerAnchorsAfterTopStopWasReached() {
    assertEquals(
      listOf(SheetAnchor(value = 1, offset = 128f)),
      resolveEffectiveSheetAnchors(
        anchors =
          listOf(SheetAnchor(value = 0, offset = 440f), SheetAnchor(value = 1, offset = 128f)),
        stopPolicy = SheetStop.Policy.DismissFromTopStop,
        hasReachedTopStop = true,
      ),
    )
  }

  @Test
  fun anchoredSheetVisibleHeightUsesVisiblePortionInDp() {
    assertEquals(
      180f,
      resolveAnchoredSheetVisibleHeight(
        containerHeightPx = 1200f,
        sheetOffsetPx = 840f,
        density = 2f,
      ),
    )
    assertEquals(
      0f,
      resolveAnchoredSheetVisibleHeight(
        containerHeightPx = 1200f,
        sheetOffsetPx = 1400f,
        density = 2f,
      ),
    )
  }
}
