package co.typie.shell

import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.ext.navigationBarsPadding
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.component.bottombar.ACTION_BUTTON_TOTAL_WIDTH
import co.typie.ui.component.bottombar.BottomBarActionButton
import co.typie.ui.component.bottombar.BottomBarDefaults
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.roundToInt

@Composable
fun MainBottomBarPill() {
  val tabState = LocalTabState.current
  val density = LocalDensity.current
  val colors = AppTheme.colors
  val state = rememberMainBottomBarPillState()
  val isPillPressed by state.interactionSource.collectIsPressedAsState()
  val activeIndicatorInsetPx = with(density) { MainBottomBarPillIndicatorActiveInset.toPx() }
  val restingIndicatorInsetPx = with(density) { MainBottomBarPillIndicatorRestingInset.toPx() }
  val trackLayout = rememberMainBottomBarTrackLayout(state.trackWidthPx, restingIndicatorInsetPx)

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

  MainBottomBarPillEffects(
    state = state,
    trackLayout = trackLayout,
    currentTab = tabState.currentTab,
    isPillPressed = isPillPressed,
  )

  val indicatorShape =
    state.indicatorShape(
      trackLayout = trackLayout,
      visualIndicatorInsetPx = visualIndicatorInsetPx,
      deformationIntensity = deformationIntensity,
    )
  val trackModifier =
    Modifier.fillMaxWidth()
      .height(BottomBarDefaults.PillHeight)
      .onSizeChanged { state.trackWidthPx = it.width.toFloat() }
      .mainBottomBarPillGestures(state = state, trackLayout = trackLayout, tabState = tabState)
      .mainBottomBarPillSurfaceDecoration(
        ambientShadowColor = colors.shadowAmbient,
        shadowColor = colors.shadowSpot,
        backgroundColor = colors.surfaceDefault,
        borderColor = colors.borderDefault.copy(alpha = 0.5f),
      )

  MainBottomBarPillLayout(
    pillScale = state.pillScale.value,
    trackModifier = trackModifier,
    indicatorShape = indicatorShape,
    indicatorColor = colors.surfaceInset,
    indicatorInsetPx = visualIndicatorInsetPx,
  )
}

@Composable
fun MainBottomBarActionButton(icon: IconData = Lucide.SquarePen, onClick: suspend () -> Unit = {}) {
  BottomBarActionButton(icon = icon, onClick = onClick)
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
) {
  Box(Modifier.fillMaxSize(), contentAlignment = Alignment.BottomCenter) {
    Box(
      Modifier.fillMaxWidth()
        .navigationBarsPadding()
        .padding(horizontal = 24.dp)
        .padding(bottom = BottomBarDefaults.BottomPadding),
      contentAlignment = Alignment.Center,
    ) {
      Row(
        Modifier.widthIn(max = 488.dp).fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
      ) {
        Box(
          Modifier.weight(1f).graphicsLayer {
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

            MainBottomBarPillTrack()
          }
        }

        Spacer(Modifier.width(ACTION_BUTTON_TOTAL_WIDTH.dp))
      }
    }
  }
}

private fun Modifier.mainBottomBarPillSurfaceDecoration(
  ambientShadowColor: Color,
  shadowColor: Color,
  backgroundColor: Color,
  borderColor: Color,
): Modifier =
  dropShadow(AppShapes.circle) {
      color = ambientShadowColor
      radius = 3f
    }
    .dropShadow(AppShapes.circle) {
      color = shadowColor
      offset = Offset(0f, 4f)
      radius = 16f
    }
    .background(backgroundColor, AppShapes.circle)
    .border(1.dp, borderColor, AppShapes.circle)

@Composable
private fun MainBottomBarPillTrack() {
  Row(Modifier.fillMaxSize()) {
    Tab.entries.forEach { tab ->
      Box(
        modifier =
          Modifier.weight(1f).fillMaxHeight().padding(MainBottomBarPillTrackSegmentPadding),
        contentAlignment = Alignment.Center,
      ) {
        Column(
          horizontalAlignment = Alignment.CenterHorizontally,
          verticalArrangement = Arrangement.Center,
        ) {
          Icon(
            icon =
              when (tab) {
                Tab.Home -> Lucide.House
                Tab.Space -> Lucide.FolderOpen
                Tab.Notes -> Lucide.StickyNote
                Tab.More -> Lucide.Ellipsis
              },
            tint = AppTheme.colors.textMuted,
          )
          Text(
            text =
              when (tab) {
                Tab.Home -> "홈"
                Tab.Space -> "스페이스"
                Tab.Notes -> "노트"
                Tab.More -> "더 보기"
              },
            style = AppTheme.typography.micro,
            color = AppTheme.colors.textMuted,
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

private val MainBottomBarPillTrackSegmentPadding = 4.dp
