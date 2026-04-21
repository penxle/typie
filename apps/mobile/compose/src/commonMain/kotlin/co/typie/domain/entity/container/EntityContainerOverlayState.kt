package co.typie.domain.entity

import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.Dp

class EntityContainerOverlayState
internal constructor(
  val animatedPasteBarVisible: Boolean,
  val reservedBottomSpacerHeight: Dp,
  private val onMetricsChangedInternal: (EntityContainerBottomOverlayMetrics) -> Unit,
) {
  fun onMetricsChanged(metrics: EntityContainerBottomOverlayMetrics) {
    onMetricsChangedInternal(metrics)
  }
}

@Composable
fun rememberEntityContainerOverlayState(
  baseBottomInset: Dp,
  pasteBarVisible: Boolean,
  resetKey: Any? = Unit,
): EntityContainerOverlayState {
  var animatedPasteBarVisible by remember(resetKey) { mutableStateOf(false) }
  var overlayMetrics by
    remember(resetKey) {
      mutableStateOf(initialEntityContainerBottomOverlayMetrics(baseBottomInset))
    }
  var lastReservedBottomSpacerTarget by
    remember(resetKey) { mutableStateOf(overlayMetrics.reservedSpacerHeight) }
  val reservedBottomSpacerAnimationDuration =
    if (overlayMetrics.reservedSpacerHeight < lastReservedBottomSpacerTarget) {
      EntityBottomOverlayDefaults.ExitDurationMillis
    } else {
      EntityBottomOverlayDefaults.EnterDurationMillis
    }
  val reservedBottomSpacerHeight by
    animateDpAsState(
      targetValue = overlayMetrics.reservedSpacerHeight,
      animationSpec = tween(reservedBottomSpacerAnimationDuration),
      label = "entity-container-bottom-spacer-height",
    )

  SideEffect { lastReservedBottomSpacerTarget = overlayMetrics.reservedSpacerHeight }
  LaunchedEffect(pasteBarVisible) { animatedPasteBarVisible = pasteBarVisible }

  return remember(resetKey, animatedPasteBarVisible, reservedBottomSpacerHeight) {
    EntityContainerOverlayState(
      animatedPasteBarVisible = animatedPasteBarVisible,
      reservedBottomSpacerHeight = reservedBottomSpacerHeight,
      onMetricsChangedInternal = { overlayMetrics = it },
    )
  }
}
