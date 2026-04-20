package co.typie.ui.component.popover

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.layout.LayoutCoordinates

private data class PopoverPaneRegisteredItem(
  val layoutCoordinates: LayoutCoordinates?,
  val enabled: Boolean,
  val onSelected: () -> Unit,
)

private enum class PopoverPaneTrackingSource {
  Local,
  SharedPending,
  SharedCommitted,
}

internal val LocalPopoverPaneSelectionState =
  compositionLocalOf<PopoverPaneSelectionState?> { null }

@Stable
internal class PopoverPaneSelectionState {
  private val itemOrder = mutableListOf<Any>()
  private val items = linkedMapOf<Any, PopoverPaneRegisteredItem>()
  private var paneCoordinates by mutableStateOf<LayoutCoordinates?>(null)
  private var trackedPointerInWindow by mutableStateOf<Offset?>(null)
  private var suppressedClickKey by mutableStateOf<Any?>(null)

  private var trackingSource by mutableStateOf<PopoverPaneTrackingSource?>(null)

  var activeItemKey by mutableStateOf<Any?>(null)
    private set

  val activeItemBoundsInPane: Rect?
    get() = activeItemKey?.let { key ->
      items[key]?.takeIf(PopoverPaneRegisteredItem::enabled)?.let(::currentBoundsInPane)
    }

  val sharedTrackedPointerInWindow: Offset?
    get() = trackedPointerInWindow?.takeIf {
      trackingSource == PopoverPaneTrackingSource.SharedCommitted
    }

  fun updatePaneCoordinates(layoutCoordinates: LayoutCoordinates?) {
    paneCoordinates = layoutCoordinates
    recomputeActiveItem()
  }

  fun registerItem(key: Any, enabled: Boolean, onSelected: () -> Unit) {
    if (itemOrder.none { it == key }) {
      itemOrder += key
    }

    val current = items[key]
    items[key] =
      if (current == null) {
        PopoverPaneRegisteredItem(
          layoutCoordinates = null,
          enabled = enabled,
          onSelected = onSelected,
        )
      } else {
        current.copy(enabled = enabled, onSelected = onSelected)
      }

    recomputeActiveItem()
  }

  fun unregisterItem(key: Any) {
    itemOrder.removeAll { it == key }
    items.remove(key)
    if (suppressedClickKey == key) {
      suppressedClickKey = null
    }
    recomputeActiveItem()
  }

  fun updateItemLayoutCoordinates(key: Any, layoutCoordinates: LayoutCoordinates?) {
    val current = items[key] ?: return
    items[key] = current.copy(layoutCoordinates = layoutCoordinates)
    recomputeActiveItem()
  }

  fun canHandleLocalGesture(positionInWindow: Offset): Boolean {
    suppressedClickKey = null
    return hitTest(positionInWindow) != null
  }

  fun consumeSuppressedClick(key: Any): Boolean {
    if (suppressedClickKey != key) {
      return false
    }

    suppressedClickKey = null
    return true
  }

  fun updatePointer(positionInWindow: Offset) {
    trackingSource = PopoverPaneTrackingSource.Local
    trackedPointerInWindow = positionInWindow
    recomputeActiveItem()
  }

  fun release(positionInWindow: Offset) {
    trackingSource = PopoverPaneTrackingSource.Local
    trackedPointerInWindow = positionInWindow
    releaseSelection(positionInWindow)
  }

  fun syncSharedSession(session: PressGestureSession?, commitDistance: Float) {
    if (trackingSource == PopoverPaneTrackingSource.Local) {
      return
    }

    if (session == null) {
      if (
        trackingSource == PopoverPaneTrackingSource.SharedPending ||
          trackingSource == PopoverPaneTrackingSource.SharedCommitted
      ) {
        clear()
      }
      return
    }

    if (!session.isArmed) {
      trackingSource = null
      trackedPointerInWindow = null
      activeItemKey = null
      if (session.isReleased) {
        clear()
      }
      return
    }

    if (
      trackingSource != PopoverPaneTrackingSource.SharedCommitted &&
        (session.positionInWindow - session.initialPositionInWindow).getDistance() <= commitDistance
    ) {
      trackingSource = PopoverPaneTrackingSource.SharedPending
      trackedPointerInWindow = null
      activeItemKey = null
      if (session.isReleased) {
        clear()
      }
      return
    }

    trackingSource = PopoverPaneTrackingSource.SharedCommitted
    trackedPointerInWindow = session.positionInWindow
    recomputeActiveItem()
    if (session.isReleased) {
      releaseSelection(session.positionInWindow)
    }
  }

  fun clear() {
    trackingSource = null
    trackedPointerInWindow = null
    activeItemKey = null
  }

  fun reset() {
    clear()
    suppressedClickKey = null
  }

  private fun releaseSelection(positionInWindow: Offset) {
    val releasedItemKey = hitTest(positionInWindow)
    val selectedItemKey =
      activeItemKey?.takeIf { it == releasedItemKey }
        ?: run {
          clear()
          return
        }
    val selectedCallback =
      items[selectedItemKey]?.takeIf(PopoverPaneRegisteredItem::enabled)?.onSelected

    if (selectedCallback != null) {
      suppressedClickKey = selectedItemKey
    }

    clear()
    selectedCallback?.invoke()
  }

  private fun hitTest(positionInWindow: Offset): Any? {
    val paneCoordinates = paneCoordinates?.takeIf(LayoutCoordinates::isAttached) ?: return null
    val positionInPane = paneCoordinates.windowToLocal(positionInWindow)

    for (key in itemOrder) {
      val item = items[key] ?: continue
      if (item.enabled && currentBoundsInPane(item)?.contains(positionInPane) == true) {
        return key
      }
    }
    return null
  }

  private fun recomputeActiveItem() {
    activeItemKey = trackedPointerInWindow?.let(::hitTest)
  }

  private fun currentBoundsInPane(item: PopoverPaneRegisteredItem): Rect? {
    val paneCoordinates = paneCoordinates?.takeIf(LayoutCoordinates::isAttached) ?: return null
    val itemCoordinates =
      item.layoutCoordinates?.takeIf(LayoutCoordinates::isAttached) ?: return null
    return paneCoordinates.localBoundingBoxOf(itemCoordinates, clipBounds = false)
  }
}

@Composable
internal fun rememberPopoverPaneSelectionState(): PopoverPaneSelectionState {
  return remember { PopoverPaneSelectionState() }
}
