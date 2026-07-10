package co.typie.screen.editor.editor.toolbar

import androidx.compose.foundation.ScrollState
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.dp
import co.typie.editor.ffi.BlockquoteVariant
import co.typie.editor.ffi.HorizontalRuleVariant
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.TableBorderStyle
import co.typie.ui.icon.IconData
import kotlinx.coroutines.CoroutineScope

internal val ToolbarHorizontalPadding = 16.dp
internal val ToolbarBottomPadding = 6.dp
internal val ToolbarHeight = 44.dp
internal val ToolbarSecondaryGap = 4.dp
internal val ToolbarSecondaryHeight = ToolbarHeight
internal val ToolbarSecondaryStackHeight = ToolbarSecondaryGap + ToolbarSecondaryHeight
internal val ToolbarIndicatorHeight = 32.dp
internal val ToolbarIndicatorGap = 8.dp
internal val EditorToolbarFloatingOverhang = ToolbarIndicatorHeight + ToolbarIndicatorGap
internal val ToolbarStackHeight = ToolbarIndicatorHeight + ToolbarIndicatorGap + ToolbarHeight
internal val ToolbarBottomPanelGap = 8.dp
internal val ToolbarBottomPanelHeight = 220.dp
internal val ToolbarBottomPanelMinHeight = 180.dp
internal val ToolbarButtonSize = 30.dp
internal val ToolbarLabelHorizontalPadding = 8.dp
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
internal val ToolbarBackdropBlurRadius = 4.dp
internal val ToolbarIndicatorItemSize = 28.dp
internal val ToolbarIndicatorIconSize = 16.dp
internal val ToolbarIndicatorPadding = 2.dp
internal val ToolbarIndicatorItemGap = 2.dp
internal val ToolbarHardStopOverscrollLimit = ToolbarPageIndicatorSlotWidth
internal val ToolbarHardStopActivationEpsilon = 10.dp

internal const val ToolbarCapsulePressedScale = 1.015f
internal const val ToolbarFixedActionPressedScale = 1.1f
internal const val ToolbarHardStopOverscrollResistance = 0.45f
internal const val ToolbarSwipeVelocityThreshold = 600f
internal const val ToolbarVisibilityEnterMillis = 200
internal const val ToolbarVisibilityExitMillis = 160
internal const val ToolbarFixedActionIconCrossfadeMillis = 150
internal const val ToolbarSecondaryVisibilityMillis = 150
internal const val ToolbarTextOptionsSwitchMillis = 150
internal const val ToolbarBottomPanelVisibilityEnterMillis = 180
internal const val ToolbarBottomPanelVisibilityExitMillis = 140
internal const val ToolbarBottomPanelHiddenScale = 0.96f
internal const val ToolbarIndicatorBackgroundMillis = 120
internal const val ToolbarIndicatorWidthMillis = 180
internal const val ToolbarIndicatorIconsMillis = 140
internal const val ToolbarIndicatorFadeMillis = 220
internal const val ToolbarIndicatorVisibleMillis = 1200L
internal const val ToolbarScrollGestureIdleResetMillis = 180L
internal const val ToolbarIndicatorFollowMovementThresholdPx = 1f
internal const val ToolbarSurfaceOpacity = 0.86f
internal const val ToolbarDisabledOpacity = 0.5f

internal enum class EditorToolbarPageKey {
  Main,
  Text,
  Image,
  File,
  Embed,
  Archived,
  HorizontalRule,
  List,
  Blockquote,
  Callout,
  Fold,
  Table,
}

internal sealed interface EditorToolbarBottomPanel {
  data object Insert : EditorToolbarBottomPanel

  data object Tools : EditorToolbarBottomPanel

  data object TableSizeSelector : EditorToolbarBottomPanel

  data class HorizontalRuleVariants(val target: HorizontalRuleVariantPanelTarget) :
    EditorToolbarBottomPanel

  data class BlockquoteVariants(val target: BlockquoteVariantPanelTarget) : EditorToolbarBottomPanel

  data class TableBorderStyles(val target: TableBorderStylePanelTarget) : EditorToolbarBottomPanel
}

internal sealed interface HorizontalRuleVariantPanelTarget {
  val currentVariant: HorizontalRuleVariant?

  data object Insertion : HorizontalRuleVariantPanelTarget {
    override val currentVariant: HorizontalRuleVariant? = null
  }

  data class Existing(val nodeId: String, override val currentVariant: HorizontalRuleVariant) :
    HorizontalRuleVariantPanelTarget
}

internal sealed interface BlockquoteVariantPanelTarget {
  val currentVariant: BlockquoteVariant?

  data object Selection : BlockquoteVariantPanelTarget {
    override val currentVariant: BlockquoteVariant? = null
  }

  data class Existing(val nodeId: String, override val currentVariant: BlockquoteVariant) :
    BlockquoteVariantPanelTarget
}

internal data class TableBorderStylePanelTarget(
  val tableId: String,
  val currentStyle: TableBorderStyle,
)

internal data class EditorToolbarScope(
  val pageKey: EditorToolbarPageKey,
  val ownerNodeId: String? = null,
) {
  companion object {
    val Main = EditorToolbarScope(EditorToolbarPageKey.Main)
  }
}

internal class EditorToolbarPage(
  val key: EditorToolbarPageKey,
  val icon: IconData,
  val contentDescription: String,
  val ownerNodeId: String? = null,
  val scrollState: ScrollState? = null,
  val content: @Composable (EditorToolbarPageScope) -> Unit,
) {
  val toolbarScope: EditorToolbarScope = EditorToolbarScope(key, ownerNodeId)
}

internal class EditorToolbarPageScope(
  val toolbarScope: EditorToolbarScope,
  val activeBottomPanel: EditorToolbarBottomPanel?,
  val activeSecondaryToolbar: EditorToolbarSecondary?,
  val commandScope: CoroutineScope,
  val hasNextPage: Boolean,
  val navigateToPage: (EditorToolbarPageKey) -> Unit,
  private val onSecondaryToolbarToggle: (EditorToolbarSecondary, EditorToolbarScope) -> Unit,
  val clearSecondaryToolbar: () -> Unit,
  private val onBottomPanelToggle: (EditorToolbarBottomPanel, EditorToolbarScope) -> Unit,
  val sendMessage: (Message) -> Unit,
  val performToolAction: (EditorToolbarToolAction) -> Unit,
) {
  fun toggleSecondaryToolbar(secondary: EditorToolbarSecondary) {
    onSecondaryToolbarToggle(secondary, toolbarScope)
  }

  fun toggleBottomPanel(panel: EditorToolbarBottomPanel) {
    onBottomPanelToggle(panel, toolbarScope)
  }
}
