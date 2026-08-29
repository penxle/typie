package co.typie.screen.editor.editor.overlay

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.editor.EditorZoomLandmark
import co.typie.editor.zoomDiffers

@Stable
internal class EditorZoomIndicatorState {
  var visibilityRequest by mutableIntStateOf(0)
    private set

  var autoHideDelayMillis by mutableLongStateOf(ZOOM_INDICATOR_DEFAULT_DWELL_MS)
    private set

  var landmarkRequest by mutableIntStateOf(0)
    private set

  var announcedLandmark by mutableStateOf<EditorZoomLandmark?>(null)
    private set

  var landmarkHeld by mutableStateOf(false)
    private set

  var snapFeedbackRequest by mutableIntStateOf(0)
    private set

  var snapFeedbackLandmark by mutableStateOf<EditorZoomLandmark?>(null)
    private set

  private var enabled by mutableStateOf(false)
  private var transientVisible by mutableStateOf(false)
  private var indicatorHovered by mutableStateOf(false)
  private var focusWithin by mutableStateOf(false)
  private var valueHovered by mutableStateOf(false)
  private var valueFocused by mutableStateOf(false)
  private var lastZoom: Float? = null
  private var lastLandmark: EditorZoomLandmark? = null
  private var landmarkInitialized = false
  private var lastRangeState: ZoomRangeState? = null

  val visible: Boolean
    get() = enabled && (transientVisible || landmarkHeld || indicatorHovered || focusWithin)

  fun updateZoom(
    enabled: Boolean,
    displayZoom: Float,
    indicatorZoom: Float = displayZoom,
    landmark: EditorZoomLandmark?,
  ) {
    val entered = enabled && !this.enabled
    this.enabled = enabled
    if (!enabled) {
      transientVisible = false
      announcedLandmark = null
      landmarkHeld = false
      snapFeedbackLandmark = null
      lastZoom = null
      lastRangeState = null
      lastLandmark = null
      landmarkInitialized = false
      return
    }

    val previousZoom = lastZoom
    val initialObservation = previousZoom == null
    val zoomChanged = !initialObservation && zoomDiffers(previousZoom, displayZoom)
    val rangeState = resolveZoomRangeState(displayZoom, indicatorZoom)
    val previousRangeState = lastRangeState
    val previousLandmark = lastLandmark
    val hadPreviousLandmark = landmarkInitialized
    lastZoom = displayZoom
    lastRangeState = rangeState
    lastLandmark = landmark
    landmarkInitialized = true

    val shouldShowOnEntry = entered && landmark != EditorZoomLandmark.Unit
    if (zoomChanged || shouldShowOnEntry) requestVisibility()

    if (previousRangeState == null || !hadPreviousLandmark) return

    if (rangeState != previousRangeState) {
      onRangeStateChanged(previousRangeState, rangeState)
      return
    }

    if (rangeState == ZoomRangeState.InRange && landmark != previousLandmark) {
      onInRangeLandmarkChanged(landmark)
    }
  }

  fun onBoundaryAttempt(landmark: EditorZoomLandmark) {
    if (!enabled) return
    if (lastRangeState != ZoomRangeState.InRange || lastLandmark != landmark) return
    if (landmark != EditorZoomLandmark.Minimum && landmark != EditorZoomLandmark.Maximum) return
    announceLandmark(landmark)
    requestVisibility()
  }

  fun onPanePointerEnter(landmark: EditorZoomLandmark?) {
    if (enabled && landmark != EditorZoomLandmark.Unit) requestVisibility()
  }

  fun onIndicatorPointerEnter() {
    indicatorHovered = true
  }

  fun onIndicatorPointerExit() {
    if (enabled && indicatorHovered) requestVisibility()
    indicatorHovered = false
  }

  fun onFocusChanged(focused: Boolean) {
    if (enabled && focusWithin && !focused) requestVisibility()
    focusWithin = focused
  }

  fun onValuePointerEnter() {
    valueHovered = true
  }

  fun onValuePointerExit() {
    valueHovered = false
  }

  fun displayedLandmark(landmark: EditorZoomLandmark?): EditorZoomLandmark? =
    if (valueHovered || valueFocused) landmark else announcedLandmark

  fun displayText(landmark: EditorZoomLandmark?, zoomPercent: Int): String =
    displayedLandmark(landmark)?.label ?: "$zoomPercent%"

  fun onValueFocusChanged(focused: Boolean) {
    valueFocused = focused
  }

  fun expireVisibility(request: Int) {
    if (request == visibilityRequest) transientVisible = false
  }

  fun expireLandmark(request: Int) {
    if (request == landmarkRequest && !landmarkHeld) announcedLandmark = null
  }

  fun reset() {
    visibilityRequest += 1
    landmarkRequest += 1
    autoHideDelayMillis = ZOOM_INDICATOR_DEFAULT_DWELL_MS
    enabled = false
    transientVisible = false
    indicatorHovered = false
    focusWithin = false
    valueHovered = false
    valueFocused = false
    announcedLandmark = null
    landmarkHeld = false
    snapFeedbackRequest = 0
    snapFeedbackLandmark = null
    lastZoom = null
    lastLandmark = null
    landmarkInitialized = false
    lastRangeState = null
  }

  private fun requestVisibility() {
    autoHideDelayMillis =
      if (announcedLandmark == null) {
        ZOOM_INDICATOR_DEFAULT_DWELL_MS
      } else {
        ZOOM_INDICATOR_LANDMARK_DWELL_MS
      }
    visibilityRequest += 1
    transientVisible = true
  }

  private fun announceLandmark(landmark: EditorZoomLandmark?, held: Boolean = false) {
    if (landmark == null && announcedLandmark == null && !landmarkHeld) return
    landmarkRequest += 1
    announcedLandmark = landmark
    landmarkHeld = landmark != null && held
  }

  private fun onRangeStateChanged(previous: ZoomRangeState, current: ZoomRangeState) {
    val boundary = current.landmark
    if (boundary != null) {
      announceLandmark(boundary, held = true)
    } else {
      announceLandmark(previous.landmark)
      requestSnapFeedback(previous.landmark)
    }
    requestVisibility()
  }

  private fun onInRangeLandmarkChanged(landmark: EditorZoomLandmark?) {
    announceLandmark(landmark)
    requestSnapFeedback(landmark)
    requestVisibility()
  }

  private fun requestSnapFeedback(landmark: EditorZoomLandmark?) {
    if (landmark == null) return
    snapFeedbackLandmark = landmark
    snapFeedbackRequest += 1
  }
}

private const val ZOOM_INDICATOR_DEFAULT_DWELL_MS = 1000L
private const val ZOOM_INDICATOR_LANDMARK_DWELL_MS = 2000L

private enum class ZoomRangeState(val landmark: EditorZoomLandmark?) {
  InRange(null),
  BelowMinimum(EditorZoomLandmark.Minimum),
  AboveMaximum(EditorZoomLandmark.Maximum),
}

private fun resolveZoomRangeState(displayZoom: Float, indicatorZoom: Float): ZoomRangeState =
  when {
    !zoomDiffers(displayZoom, indicatorZoom) -> ZoomRangeState.InRange
    displayZoom < indicatorZoom -> ZoomRangeState.BelowMinimum
    else -> ZoomRangeState.AboveMaximum
  }

private val EditorZoomLandmark.label: String
  get() =
    when (this) {
      EditorZoomLandmark.Minimum -> "최소"
      EditorZoomLandmark.FitWidth -> "맞춤"
      EditorZoomLandmark.Unit -> "원본"
      EditorZoomLandmark.Maximum -> "최대"
    }
