package co.typie.ui.component.popover

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntRect

val LocalPopoverOverlayState =
  staticCompositionLocalOf<PopoverOverlayState> { error("No PopoverOverlayState provided") }

@Stable
class PopoverOverlayState {
  internal var entry: PopoverOverlayEntry? by mutableStateOf(null)
  var anchorBounds: IntRect by mutableStateOf(IntRect.Zero)
  var progress: Float by mutableStateOf(0f)
  var interactive: Boolean by mutableStateOf(true)
  var paneBoundsInWindow: Rect? by mutableStateOf(null)
}

internal class PopoverOverlayEntry(
  val placement: PopoverPlacement,
  val screenPadding: PopoverScreenPadding,
  val collapsedCornerRadius: Dp,
  val maxWidth: Dp?,
  val minWidth: Dp,
  val expandToMaxWidth: Boolean,
  val pane: @Composable () -> Unit,
  val anchor: @Composable () -> Unit,
)
