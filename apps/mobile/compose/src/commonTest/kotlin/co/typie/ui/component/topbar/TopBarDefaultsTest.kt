package co.typie.ui.component.topbar

import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals

class TopBarDefaultsTest {
  @Test
  fun statusBarInsetIsTheTopPadding() {
    assertEquals(24.dp, resolveTopBarTopPadding(statusTop = 24.dp, hasHorizontalSafeArea = true))
  }

  @Test
  fun zeroInsetsHaveNoTopPadding() {
    assertEquals(0.dp, resolveTopBarTopPadding(statusTop = 0.dp, hasHorizontalSafeArea = false))
  }

  @Test
  fun horizontalSafeAreaUsesLandscapeTopPaddingWithoutStatusBar() {
    assertEquals(
      TopBarDefaults.LandscapeTopPadding,
      resolveTopBarTopPadding(statusTop = 0.dp, hasHorizontalSafeArea = true),
    )
  }
}
