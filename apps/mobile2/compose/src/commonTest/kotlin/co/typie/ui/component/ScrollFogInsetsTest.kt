package co.typie.ui.component

import androidx.compose.foundation.layout.calculateEndPadding
import androidx.compose.foundation.layout.calculateStartPadding
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals

class ScrollFogInsetsTest {

  @Test
  fun toPaddingValues_returnsZeroPaddingForZeroInsets() {
    val paddingValues = ScrollFogInsets().toPaddingValues()

    assertEquals(0.dp, paddingValues.calculateTopPadding())
    assertEquals(0.dp, paddingValues.calculateBottomPadding())
    assertEquals(0.dp, paddingValues.calculateStartPadding(LayoutDirection.Ltr))
    assertEquals(0.dp, paddingValues.calculateEndPadding(LayoutDirection.Ltr))
  }

  @Test
  fun toPaddingValues_preservesAllInsetValues() {
    val paddingValues =
      ScrollFogInsets(top = 4.dp, bottom = 8.dp, left = 12.dp, right = 16.dp).toPaddingValues()

    assertEquals(4.dp, paddingValues.calculateTopPadding())
    assertEquals(8.dp, paddingValues.calculateBottomPadding())
    assertEquals(12.dp, paddingValues.calculateStartPadding(LayoutDirection.Ltr))
    assertEquals(16.dp, paddingValues.calculateEndPadding(LayoutDirection.Ltr))
  }

  @Test
  fun toPaddingValues_respectsLayoutDirectionForHorizontalInsets() {
    val paddingValues = ScrollFogInsets(left = 12.dp, right = 16.dp).toPaddingValues()

    assertEquals(16.dp, paddingValues.calculateStartPadding(LayoutDirection.Rtl))
    assertEquals(12.dp, paddingValues.calculateEndPadding(LayoutDirection.Rtl))
  }
}
