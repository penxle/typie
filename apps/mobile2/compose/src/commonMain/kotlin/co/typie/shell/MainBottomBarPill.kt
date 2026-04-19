package co.typie.shell

import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.wrapContentWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.ext.navigationBarsPadding
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.component.bottombar.ACTION_BUTTON_TOTAL_WIDTH
import co.typie.ui.component.bottombar.BottomBarDefaults
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.roundToInt
import kotlin.math.sign

@Composable
fun MainBottomBarPill() {
  val tabState = LocalTabState.current
  val density = LocalDensity.current
  val colors = AppTheme.colors
  val state = rememberMainBottomBarPillState(initialActiveTab = tabState.currentTab)
  val isPillPressed by state.interactionSource.collectIsPressedAsState()

  // Single source for the label's TextStyle — both the TextMeasurer (which sizes
  // the active-state tab box) and the rendered Text must use the same style or the
  // measured width and the actual layout width will diverge.
  val labelStyle = AppTheme.typography.label
  val tabWidthsConfig = rememberTabWidthsConfig(labelStyle)

  MainBottomBarPillEffects(
    state = state,
    currentTab = tabState.currentTab,
    isPillPressed = isPillPressed,
  )

  // Per-tab widths and cumulative centers derived from progress.
  // Single source: state.tabProgress → tab box width → indicator bounds.
  val currentTabWidths = tabWidthsConfig.currentWidths(state)
  val tabCenters = cumulativeCenters(currentTabWidths)
  val totalWidthPx = currentTabWidths.values.sum()

  val activeIndicatorInsetPx = with(density) { MainBottomBarPillIndicatorActiveInset.toPx() }
  val restingIndicatorInsetPx = with(density) { MainBottomBarPillIndicatorRestingInset.toPx() }
  val deformationIntensity by
    animateFloatAsState(
      targetValue = state.deformationTarget,
      animationSpec = tween(90, easing = EaseOutCubic),
    )
  val visualIndicatorInsetPx by
    animateFloatAsState(
      targetValue = if (state.isGestureActive) activeIndicatorInsetPx else restingIndicatorInsetPx,
      animationSpec =
        tween(MainBottomBarPillIndicatorInsetAnimationDurationMillis, easing = EaseOutCubic),
    )

  // Natural indicator bounds = active tab's box contracted by the resting inset.
  val activeTab = tabState.currentTab
  val activeBoxCenter = tabCenters.getValue(activeTab)
  val activeBoxWidth = currentTabWidths.getValue(activeTab)
  val naturalIndicatorLeft = activeBoxCenter - activeBoxWidth / 2f + restingIndicatorInsetPx
  val naturalIndicatorRight = activeBoxCenter + activeBoxWidth / 2f - restingIndicatorInsetPx

  val indicatorShape =
    state.indicatorShape(
      naturalLeft = naturalIndicatorLeft,
      naturalRight = naturalIndicatorRight,
      tabCenters = tabCenters,
      tabWidths = currentTabWidths,
      indicatorInsetPx = restingIndicatorInsetPx,
      totalWidth = totalWidthPx,
      deformationIntensity = deformationIntensity,
    )

  val totalWidthDp = with(density) { totalWidthPx.toDp() }

  val trackModifier =
    Modifier.width(totalWidthDp)
      .height(BottomBarDefaults.PillHeight)
      .mainBottomBarPillGestures(
        state = state,
        tabCenters = tabCenters,
        tabWidths = currentTabWidths,
        indicatorInsetPx = restingIndicatorInsetPx,
        totalWidth = totalWidthPx,
        tabState = tabState,
      )
      .mainBottomBarPillSurfaceDecoration(
        shadowColor = colors.shadowSpot,
        shadowOffsetY = 12.dp,
        shadowBlur = 28.dp,
        shadowSpread = (-10).dp,
        backgroundColor = colors.surfaceDefault,
        borderColor = colors.borderDefault.copy(alpha = 0.5f),
      )

  MainBottomBarPillLayout(
    pillScale = state.pillScale.value,
    trackModifier = trackModifier,
    indicatorShape = indicatorShape,
    indicatorColor = colors.surfaceInset,
    indicatorInsetPx = visualIndicatorInsetPx,
    state = state,
    currentTabWidths = currentTabWidths,
    activeWidthsPx = tabWidthsConfig.activeWidthsPx,
    labelStyle = labelStyle,
  )
}

val MainBottomBarPillKey = Any()
val MainBottomBarPillEntry: @Composable () -> Unit = { MainBottomBarPill() }

@Composable
private fun MainBottomBarPillLayout(
  pillScale: Float,
  trackModifier: Modifier,
  indicatorShape: BottomBarIndicatorShape?,
  indicatorColor: Color,
  indicatorInsetPx: Float,
  state: MainBottomBarPillState,
  currentTabWidths: Map<Tab, Float>,
  activeWidthsPx: Map<Tab, Float>,
  labelStyle: TextStyle,
) {
  Box(Modifier.fillMaxSize(), contentAlignment = Alignment.BottomCenter) {
    Box(
      Modifier.fillMaxWidth()
        .navigationBarsPadding()
        .padding(horizontal = 16.dp)
        .padding(bottom = BottomBarDefaults.BottomPadding),
      contentAlignment = Alignment.Center,
    ) {
      Row(
        Modifier.widthIn(max = 488.dp).fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
      ) {
        Box(
          Modifier.wrapContentWidth().graphicsLayer {
            scaleX = pillScale
            scaleY = pillScale
          }
        ) {
          Box(trackModifier) {
            if (indicatorShape != null) {
              MainBottomBarPillIndicator(
                shape = indicatorShape,
                color = indicatorColor,
                insetPx = indicatorInsetPx,
              )
            }

            MainBottomBarPillTrack(
              state = state,
              currentTabWidths = currentTabWidths,
              activeWidthsPx = activeWidthsPx,
              labelStyle = labelStyle,
            )
          }
        }

        Spacer(Modifier.weight(1f))
        Spacer(Modifier.width(ACTION_BUTTON_TOTAL_WIDTH.dp))
      }
    }
  }
}

private fun Modifier.mainBottomBarPillSurfaceDecoration(
  shadowColor: Color,
  shadowOffsetY: Dp,
  shadowBlur: Dp,
  shadowSpread: Dp,
  backgroundColor: Color,
  borderColor: Color,
): Modifier =
  dropShadow(AppShapes.circle) {
      color = shadowColor
      offset = Offset(0f, shadowOffsetY.toPx())
      radius = shadowBlur.toPx()
      spread = shadowSpread.toPx()
    }
    .border(1.dp, borderColor, AppShapes.circle)
    .background(backgroundColor, AppShapes.circle)

@Composable
private fun MainBottomBarPillTrack(
  state: MainBottomBarPillState,
  currentTabWidths: Map<Tab, Float>,
  activeWidthsPx: Map<Tab, Float>,
  labelStyle: TextStyle,
) {
  val density = LocalDensity.current
  val insetPx = with(density) { MainBottomBarPillIndicatorRestingInset.toPx() }
  Row(Modifier.fillMaxHeight()) {
    Tab.entries.forEach { tab ->
      val widthDp = with(density) { currentTabWidths.getValue(tab).toDp() }
      val activeWidthDp = with(density) { activeWidthsPx.getValue(tab).toDp() }
      val progress = state.tabProgress.getValue(tab).value
      val presentation = tab.presentation
      // The active indicator sits restingInset inward from its box, leaving a
      // visual gap on each inactive tab's active-adjacent edge. Shift the tab's
      // row toward the active side by progress-weighted inset/2 so the icon
      // lands at the perceived center of that visible gap. Weighting sums to ±1
      // in resting state and interpolates smoothly during transitions; the
      // (other.ordinal - tab.ordinal) sign generalizes the direction to N tabs.
      val visualOffsetPx =
        (Tab.entries.sumOf { other ->
            if (other == tab) 0.0
            else
              state.tabProgress.getValue(other).value.toDouble() *
                (other.ordinal - tab.ordinal).sign
          } * insetPx / 2.0)
          .toFloat()
      Box(
        modifier = Modifier.width(widthDp).fillMaxHeight().clipToBounds(),
        contentAlignment = Alignment.CenterStart,
      ) {
        // Inner row always laid out at the active width so content position stays stable
        // across the progress transition. The outer Box clips the trailing (label) portion
        // while the tab is collapsed; the label itself fades via alpha = progress.
        Row(
          modifier =
            Modifier.width(activeWidthDp)
              .fillMaxHeight()
              .offset { IntOffset(visualOffsetPx.roundToInt(), 0) }
              .padding(horizontal = MainBottomBarPillTabHorizontalPadding),
          verticalAlignment = Alignment.CenterVertically,
        ) {
          Icon(
            icon = presentation.icon,
            modifier = Modifier.size(MainBottomBarPillIconSize),
            tint = lerp(AppTheme.colors.textHint, AppTheme.colors.textDefault, progress),
            strokeWidth = 2.5f,
          )
          Spacer(Modifier.width(MainBottomBarPillTrackInlineLabelGap))
          Text(
            text = presentation.label,
            modifier = Modifier.alpha(progress),
            style = labelStyle,
            color = AppTheme.colors.textDefault,
            maxLines = 1,
          )
        }
      }
    }
  }
}

@Composable
private fun MainBottomBarPillIndicator(
  shape: BottomBarIndicatorShape,
  color: Color,
  insetPx: Float,
) {
  val density = LocalDensity.current
  val height =
    with(density) {
      val pillHeightPx = BottomBarDefaults.PillHeight.toPx()
      (pillHeightPx - insetPx * 2f).coerceAtLeast(0f).toDp()
    }

  Box(
    Modifier.offset { IntOffset(x = shape.leftX.roundToInt(), y = insetPx.roundToInt()) }
      .width(with(density) { shape.width.toDp() })
      .height(height)
      .background(color, AppShapes.circle)
  )
}

/**
 * Displayable presentation for a [Tab]. Co-locating icon and label in a single exhaustive `when`
 * ensures a new tab can't be added without also providing both pieces.
 */
private data class TabPresentation(val icon: IconData, val label: String)

private val Tab.presentation: TabPresentation
  get() =
    when (this) {
      Tab.Home -> TabPresentation(icon = Lucide.House, label = "홈")
      Tab.Space -> TabPresentation(icon = Lucide.FolderOpen, label = "스페이스")
    }

/**
 * Pre-computed tab box widths keyed by activation state. `inactiveWidthPx` is shared (icon-only
 * layout = padding × 2 + icon size); `activeWidthsPx` is per-tab since each label's text width
 * differs. Measured once via [rememberTabWidthsConfig] so later transition/indicator math never
 * needs to wait for layout.
 */
private data class MainBottomBarPillTabWidthsConfig(
  val inactiveWidthPx: Float,
  val activeWidthsPx: Map<Tab, Float>,
) {
  fun widthFor(tab: Tab, progress: Float): Float =
    inactiveWidthPx + (activeWidthsPx.getValue(tab) - inactiveWidthPx) * progress

  fun currentWidths(state: MainBottomBarPillState): Map<Tab, Float> =
    Tab.entries.associateWith { tab -> widthFor(tab, state.tabProgress.getValue(tab).value) }
}

@Composable
private fun rememberTabWidthsConfig(labelStyle: TextStyle): MainBottomBarPillTabWidthsConfig {
  val density = LocalDensity.current
  val textMeasurer = rememberTextMeasurer()
  return remember(textMeasurer, labelStyle, density) {
    val paddingPx = with(density) { MainBottomBarPillTabHorizontalPadding.toPx() }
    val iconPx = with(density) { MainBottomBarPillIconSize.toPx() }
    val gapPx = with(density) { MainBottomBarPillTrackInlineLabelGap.toPx() }
    val inactiveWidthPx = paddingPx * 2f + iconPx
    val activeWidthsPx =
      Tab.entries.associateWith { tab ->
        val labelWidth =
          textMeasurer
            .measure(tab.presentation.label, labelStyle, maxLines = 1)
            .size
            .width
            .toFloat()
        paddingPx * 2f + iconPx + gapPx + labelWidth
      }
    MainBottomBarPillTabWidthsConfig(inactiveWidthPx, activeWidthsPx)
  }
}

internal fun cumulativeCenters(widths: Map<Tab, Float>): Map<Tab, Float> {
  var cumulativeLeft = 0f
  return Tab.entries.associateWith { tab ->
    val w = widths.getValue(tab)
    val center = cumulativeLeft + w / 2f
    cumulativeLeft += w
    center
  }
}

private val MainBottomBarPillIconSize = 18.dp
private val MainBottomBarPillTabHorizontalPadding = 24.dp
private val MainBottomBarPillTrackInlineLabelGap = 8.dp
