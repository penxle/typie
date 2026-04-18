package co.typie.ui.component.popover

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import co.typie.ext.toDp
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
internal fun PopoverPaneSelectionIndicator(activeBoundsInPane: Rect?, itemRadius: Dp) {
  val density = LocalDensity.current
  val indicatorX = remember { Animatable(0f) }
  val indicatorY = remember { Animatable(0f) }
  val indicatorWidth = remember { Animatable(0f) }
  val indicatorHeight = remember { Animatable(0f) }
  var indicatorVisible by remember { mutableStateOf(false) }
  val animSpec = tween<Float>(PopoverDefaults.IndicatorDuration, easing = EaseOutCubic)

  LaunchedEffect(activeBoundsInPane) {
    val bounds = activeBoundsInPane
    if (bounds == null) {
      indicatorVisible = false
      return@LaunchedEffect
    }

    if (!indicatorVisible) {
      indicatorX.snapTo(bounds.left)
      indicatorY.snapTo(bounds.top)
      indicatorWidth.snapTo(bounds.width)
      indicatorHeight.snapTo(bounds.height)
      indicatorVisible = true
    } else {
      launch { indicatorX.animateTo(bounds.left, animSpec) }
      launch { indicatorY.animateTo(bounds.top, animSpec) }
      launch { indicatorWidth.animateTo(bounds.width, animSpec) }
      launch { indicatorHeight.animateTo(bounds.height, animSpec) }
    }
  }

  if (indicatorVisible) {
    Box(
      modifier =
        Modifier.offset { IntOffset(indicatorX.value.toInt(), indicatorY.value.toInt()) }
          .width(indicatorWidth.value.toDp(density))
          .height(indicatorHeight.value.toDp(density))
          .background(AppTheme.colors.surfaceInset, AppShapes.squircle(itemRadius))
    )
  }
}
