package co.typie.ui.component

import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.ScrollableState
import androidx.compose.foundation.gestures.rememberScrollableState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.scrollableArea
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.LocalScrollGestureLockState
import co.typie.ext.desktopDragScroll

object ResponsiveContainerDefaults {
  val Breakpoint = 600.dp
  val MaxWidth = 600.dp
}

data class ResponsiveContainerMetrics(
  val enabled: Boolean,
  val contentWidth: Float,
  val gutterWidth: Float,
)

fun resolveResponsiveContainerMetrics(
  screenWidth: Float,
  maxWidth: Float = ResponsiveContainerDefaults.MaxWidth.value,
  breakpoint: Float = ResponsiveContainerDefaults.Breakpoint.value,
): ResponsiveContainerMetrics {
  if (screenWidth < breakpoint) {
    return ResponsiveContainerMetrics(enabled = false, contentWidth = screenWidth, gutterWidth = 0f)
  }

  val contentWidth = minOf(screenWidth, maxWidth)
  val gutterWidth = ((screenWidth - contentWidth) / 2f).coerceAtLeast(0f)

  return ResponsiveContainerMetrics(
    enabled = true,
    contentWidth = contentWidth,
    gutterWidth = gutterWidth,
  )
}

@Composable
fun ResponsiveContainer(
  modifier: Modifier = Modifier,
  contentMaxWidth: Dp = ResponsiveContainerDefaults.MaxWidth,
  breakpoint: Dp = ResponsiveContainerDefaults.Breakpoint,
  alignment: Alignment = Alignment.TopCenter,
  primaryScrollableState: ScrollableState? = null,
  content: @Composable () -> Unit,
) {
  BoxWithConstraints(modifier = modifier) {
    val metrics =
      resolveResponsiveContainerMetrics(
        screenWidth = this.maxWidth.value,
        maxWidth = contentMaxWidth.value,
        breakpoint = breakpoint.value,
      )
    val contentWidth = metrics.contentWidth.dp
    val gutterWidth = metrics.gutterWidth.dp

    Box(Modifier.fillMaxSize()) {
      Box(modifier = Modifier.align(alignment).width(contentWidth)) { content() }

      if (metrics.enabled && primaryScrollableState != null && gutterWidth > 0.dp) {
        Row(Modifier.fillMaxSize()) {
          ResponsiveContainerGutter(
            primaryScrollableState = primaryScrollableState,
            width = gutterWidth,
          )
          Spacer(Modifier.weight(1f))
          ResponsiveContainerGutter(
            primaryScrollableState = primaryScrollableState,
            width = gutterWidth,
          )
        }
      }
    }
  }
}

@Composable
private fun ResponsiveContainerGutter(primaryScrollableState: ScrollableState, width: Dp) {
  val isLocked = LocalScrollGestureLockState.current.isLocked
  val gutterScrollableState = rememberScrollableState { delta ->
    primaryScrollableState.dispatchRawDelta(delta)
  }

  Box(
    modifier =
      Modifier.width(width)
        .fillMaxHeight()
        .desktopDragScroll(
          state = gutterScrollableState,
          orientation = Orientation.Vertical,
          enabled = !isLocked,
        )
        .scrollableArea(
          state = gutterScrollableState,
          orientation = Orientation.Vertical,
          enabled = !isLocked,
        )
  )
}
