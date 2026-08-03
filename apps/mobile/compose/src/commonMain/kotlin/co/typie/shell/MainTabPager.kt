package co.typie.shell

import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.PagerState
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.unit.Velocity
import kotlin.math.abs

@Composable
internal fun MainTabPager(
  state: PagerState,
  userScrollEnabled: Boolean,
  modifier: Modifier = Modifier,
  content: @Composable (Tab) -> Unit,
) {
  HorizontalPager(
    state = state,
    modifier = modifier,
    userScrollEnabled = userScrollEnabled,
    overscrollEffect = null,
  ) { page ->
    content(Tab.entries[page])
  }
}

internal fun mainTabActivationWeights(position: Float): Map<Tab, Float> =
  Tab.entries.associateWith { tab -> (1f - abs(position - tab.ordinal)).coerceIn(0f, 1f) }

internal fun resolveMainTabChrome(settledTab: Tab, targetTab: Tab, isDirectDrag: Boolean): Tab =
  if (isDirectDrag) settledTab else targetTab

internal fun canAdmitMainTabPagerGesture(
  navigatorCanPop: Boolean,
  navigatorIsTransitioning: Boolean,
  pagerAtRest: Boolean,
  drawerAtRest: Boolean,
): Boolean = !navigatorCanPop && !navigatorIsTransitioning && pagerAtRest && drawerAtRest

internal fun mainTabPillHandoffProgress(
  position: Float,
  originPosition: Float,
  targetPosition: Float,
): Float {
  val distance = targetPosition - originPosition
  if (distance == 0f) return 1f
  return ((position - originPosition) / distance).coerceIn(0f, 1f)
}

internal fun Modifier.mainTabPagerChildHorizontalGesture(): Modifier =
  nestedScroll(MainTabPagerChildHorizontalHandoffBoundary)

private data object MainTabPagerChildHorizontalHandoffBoundary : NestedScrollConnection {
  override fun onPostScroll(
    consumed: Offset,
    available: Offset,
    source: NestedScrollSource,
  ): Offset =
    if (source == NestedScrollSource.UserInput) Offset(x = available.x, y = 0f) else Offset.Zero

  override suspend fun onPostFling(consumed: Velocity, available: Velocity): Velocity =
    Velocity(x = available.x, y = 0f)
}
