package co.typie.screen.editor.editor.toolbar

import androidx.compose.foundation.ScrollState
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.dp
import co.typie.ui.icon.IconData

internal val ToolbarHorizontalPadding = 16.dp
internal val ToolbarBottomPadding = 12.dp
internal val ToolbarHeight = 44.dp
internal val ToolbarIndicatorHeight = 32.dp
internal val ToolbarIndicatorGap = 8.dp
internal val EditorToolbarFloatingOverhang = ToolbarIndicatorHeight + ToolbarIndicatorGap
internal val ToolbarStackHeight = ToolbarIndicatorHeight + ToolbarIndicatorGap + ToolbarHeight
internal val ToolbarBottomPanelGap = 8.dp
internal val ToolbarBottomPanelHeight = 220.dp
internal val ToolbarButtonSize = 30.dp
internal val ToolbarLabelMinWidth = 52.dp
internal val ToolbarLabelHorizontalPadding = 10.dp
internal val ToolbarPageVerticalPadding = 4.dp
internal val ToolbarPageStartPadding = 8.dp
internal val ToolbarPageEndPadding = 4.dp
internal val ToolbarItemGap = 4.dp
internal val ToolbarIconSize = 20.dp
internal val ToolbarFixedActionWidth = 44.dp
internal val ToolbarPageIndicatorSlotWidth = ToolbarFixedActionWidth
internal val ToolbarLastPageReservedEndPadding = ToolbarPageEndPadding + ToolbarFixedActionWidth
internal val ToolbarFixedActionPadding = 4.dp
internal val ToolbarDividerHeight = 20.dp
internal val ToolbarBorderWidth = 1.dp
internal val ToolbarIndicatorItemSize = 28.dp
internal val ToolbarIndicatorIconSize = 16.dp
internal val ToolbarIndicatorPadding = 2.dp
internal val ToolbarIndicatorItemGap = 2.dp
internal val ToolbarHardStopOverscrollLimit = ToolbarPageIndicatorSlotWidth

internal const val ToolbarCapsulePressedScale = 1.015f
internal const val ToolbarFixedActionPressedScale = 1.1f
internal const val ToolbarHardStopOverscrollResistance = 0.45f
internal const val ToolbarSwipeVelocityThreshold = 600f
internal const val ToolbarVisibilityEnterMillis = 200
internal const val ToolbarVisibilityExitMillis = 160
internal const val ToolbarIndicatorBackgroundMillis = 120
internal const val ToolbarIndicatorFadeMillis = 220
internal const val ToolbarIndicatorVisibleMillis = 1200L
internal const val ToolbarIndicatorFollowMovementThresholdPx = 1f
internal const val ToolbarSurfaceOpacity = 0.86f

internal enum class EditorToolbarPageKey {
  Main,
  Text,
  Image,
}

internal enum class EditorToolbarBottomPanelKey {
  Insert,
  More,
}

internal class EditorToolbarPage(
  val key: EditorToolbarPageKey,
  val icon: IconData,
  val contentDescription: String,
  val scrollState: ScrollState? = null,
  val content: @Composable (EditorToolbarPageScope) -> Unit,
)

internal class EditorToolbarPageScope(
  val activeBottomPanel: EditorToolbarBottomPanelKey?,
  val hasNextPage: Boolean,
  val navigateToPage: (EditorToolbarPageKey) -> Unit,
  val toggleBottomPanel: (EditorToolbarBottomPanelKey) -> Unit,
)
