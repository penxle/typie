package co.typie.ui.component.popover

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.State
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

  val acceptsInput: Boolean
    get() = owner != null && entry != null && interactive

  var anchorBounds: IntRect by mutableStateOf(IntRect.Zero)
    private set

  private var detachedProgress by mutableStateOf(0f)
  internal val progress: Float
    get() = if (isDetached) detachedProgress else entry?.progress?.value ?: 0f

  var interactive: Boolean by mutableStateOf(true)
    private set

  var paneBoundsInWindow: Rect? by mutableStateOf(null)
    private set

  internal val outsideDismissPaneBoundsInWindow: Rect?
    get() = if (onOutsideDismiss != null) paneBoundsInWindow else null

  internal var isDetached: Boolean by mutableStateOf(false)
    private set

  private var outsideDismissPointerId by mutableStateOf<Long?>(null)
  internal val isOutsideDismissGestureActive: Boolean
    get() = outsideDismissPointerId != null

  internal fun suppressesTap(pointerId: Long): Boolean = outsideDismissPointerId == pointerId

  internal fun show(owner: Any, entry: PopoverOverlayEntry, anchorBounds: IntRect) {
    this.owner = owner
    isDetached = false
    onOutsideDismiss = null
    this.entry = entry
    this.anchorBounds = anchorBounds
    detachedProgress = 0f
    interactive = true
    paneBoundsInWindow = null
  }

  internal fun update(
    owner: Any,
    entry: PopoverOverlayEntry,
    anchorBounds: IntRect,
    interactive: Boolean,
  ) {
    if (this.owner !== owner) {
      return
    }

    this.entry = entry
    this.anchorBounds = anchorBounds
    this.interactive = interactive
  }

  internal fun updatePaneBounds(owner: Any, paneBoundsInWindow: Rect?) {
    if (this.owner !== owner) {
      return
    }

    this.paneBoundsInWindow = paneBoundsInWindow
  }

  internal fun stopAcceptingInput(owner: Any) {
    if (this.owner !== owner) {
      return
    }

    onOutsideDismiss = null
    interactive = false
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

  internal fun beginOutsideDismissGesture(pointerId: Long): Int {
    outsideDismissGestureIdState += 1
    outsideDismissPointerId = pointerId
    return outsideDismissGestureIdState
  }

  internal fun endOutsideDismissGesture(gestureId: Int) {
    if (outsideDismissGestureIdState == gestureId) {
      outsideDismissPointerId = null
    }
  }

  internal fun detach(owner: Any) {
    if (this.owner !== owner) {
      return
    }

    detachedProgress = progress
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
    detachedProgress = progress
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
    detachedProgress = 0f
    interactive = true
    paneBoundsInWindow = null
  }
}

internal class PopoverOverlayEntry(
  val owner: Any,
  val progress: State<Float>,
  val placement: PopoverPlacement,
  val screenPadding: PopoverScreenPadding,
  val maxWidth: Dp?,
  val minWidth: Dp,
  val pane: @Composable () -> Unit,
  val anchor: @Composable () -> Unit,
  val anchorSurface: PopoverAnchorSurface? = null,
)
