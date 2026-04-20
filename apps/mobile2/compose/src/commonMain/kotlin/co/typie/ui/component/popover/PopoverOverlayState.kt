package co.typie.ui.component.popover

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
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
  private var owner: Any? by mutableStateOf(null)
  private var detachedCloseRequestIdState: Int by mutableIntStateOf(0)
  private var outsideDismissGestureIdState: Int by mutableIntStateOf(0)
  private var onOutsideDismiss: (() -> Unit)? = null
  internal var entry: PopoverOverlayEntry? by mutableStateOf(null)
    private set

  var anchorBounds: IntRect by mutableStateOf(IntRect.Zero)
    private set

  internal var progress: Float by mutableStateOf(0f)
    private set

  val easedProgress: Float
    get() = PopoverDefaults.PopoverEasing.transform(progress).coerceIn(0f, 1f)

  var interactive: Boolean by mutableStateOf(true)
    private set

  var paneBoundsInWindow: Rect? by mutableStateOf(null)
    private set

  internal val outsideDismissPaneBoundsInWindow: Rect?
    get() = if (onOutsideDismiss != null) paneBoundsInWindow else null

  internal var isDetached: Boolean by mutableStateOf(false)
    private set

  internal var isOutsideDismissGestureActive: Boolean by mutableStateOf(false)
    private set

  internal fun show(owner: Any, entry: PopoverOverlayEntry, anchorBounds: IntRect) {
    this.owner = owner
    isDetached = false
    onOutsideDismiss = null
    this.entry = entry
    this.anchorBounds = anchorBounds
    progress = 0f
    interactive = true
    paneBoundsInWindow = null
  }

  internal fun update(
    owner: Any,
    entry: PopoverOverlayEntry,
    anchorBounds: IntRect,
    progress: Float,
    interactive: Boolean,
  ) {
    if (this.owner !== owner) {
      return
    }

    this.entry = entry
    this.anchorBounds = anchorBounds
    this.progress = progress
    this.interactive = interactive
  }

  internal fun updatePaneBounds(owner: Any, paneBoundsInWindow: Rect?) {
    if (this.owner !== owner) {
      return
    }

    this.paneBoundsInWindow = paneBoundsInWindow
  }

  internal fun isOwnedBy(owner: Any): Boolean = this.owner === owner

  internal fun updateOutsideDismiss(owner: Any, onOutsideDismiss: () -> Unit) {
    if (this.owner !== owner) {
      return
    }

    this.onOutsideDismiss = onOutsideDismiss
  }

  internal fun clearOutsideDismiss(owner: Any) {
    if (this.owner !== owner) {
      return
    }

    onOutsideDismiss = null
  }

  internal fun dismissFromOutsideGesture() {
    onOutsideDismiss?.invoke()
  }

  internal fun beginOutsideDismissGesture(): Int {
    outsideDismissGestureIdState += 1
    isOutsideDismissGestureActive = true
    return outsideDismissGestureIdState
  }

  internal fun endOutsideDismissGesture(gestureId: Int) {
    if (outsideDismissGestureIdState == gestureId) {
      isOutsideDismissGestureActive = false
    }
  }

  internal fun detach(owner: Any) {
    if (this.owner !== owner) {
      return
    }

    this.owner = null
    onOutsideDismiss = null
    interactive = false
    paneBoundsInWindow = null
    isDetached = true
    detachedCloseRequestIdState += 1
  }

  internal fun updateDetachedProgress(closeRequestId: Int, progress: Float) {
    if (!isDetached || detachedCloseRequestIdState != closeRequestId) {
      return
    }

    this.progress = progress
  }

  internal fun clearDetached(closeRequestId: Int) {
    if (!isDetached || detachedCloseRequestIdState != closeRequestId) {
      return
    }

    reset()
  }

  internal fun detachedCloseRequestId(): Int = detachedCloseRequestIdState

  internal fun clear(owner: Any) {
    if (this.owner !== owner) {
      return
    }

    reset()
  }

  private fun reset() {
    owner = null
    onOutsideDismiss = null
    isDetached = false
    entry = null
    anchorBounds = IntRect.Zero
    progress = 0f
    interactive = true
    paneBoundsInWindow = null
  }
}

internal class PopoverOverlayEntry(
  val owner: Any,
  val placement: PopoverPlacement,
  val screenPadding: PopoverScreenPadding,
  val collapsedCornerRadius: Dp,
  val maxWidth: Dp?,
  val minWidth: Dp,
  val expandToMaxWidth: Boolean,
  val pane: @Composable () -> Unit,
  val anchor: @Composable () -> Unit,
)
