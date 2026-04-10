package co.typie.ui.component.sheet

import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals

class SheetInsetPolicyTest {

  @Test
  fun containerInsetConsumesLargestBottomInsetAtContainer() {
    val resolved = resolveSheetBottomInset(
      policy = SheetInsetPolicy.Container,
      imeBottom = 320.dp,
      safeBottom = 34.dp,
    )

    assertEquals(320.dp, resolved.containerBottom)
    assertEquals(0.dp, resolved.contentTailBottom)
  }

  @Test
  fun contentTailInsetPushesInsetIntoScrollableTail() {
    val resolved = resolveSheetBottomInset(
      policy = SheetInsetPolicy.ContentTail,
      imeBottom = 280.dp,
      safeBottom = 16.dp,
    )

    assertEquals(0.dp, resolved.containerBottom)
    assertEquals(280.dp, resolved.contentTailBottom)
  }

  @Test
  fun noneInsetLeavesBottomUntouched() {
    val resolved = resolveSheetBottomInset(
      policy = SheetInsetPolicy.None,
      imeBottom = 280.dp,
      safeBottom = 16.dp,
    )

    assertEquals(0.dp, resolved.containerBottom)
    assertEquals(0.dp, resolved.contentTailBottom)
  }
}
