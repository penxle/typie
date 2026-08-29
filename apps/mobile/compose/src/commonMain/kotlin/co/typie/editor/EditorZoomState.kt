package co.typie.editor

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.documentZoomWidth
import kotlin.math.abs
import kotlin.math.ceil
import kotlin.math.floor
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private const val MinDocumentDisplayWidth = 100f
private const val MaxDocumentZoom = 2f
private const val FitWidthZoomSnapThreshold = 0.02f
private const val UnitZoomSnapThreshold = 0.02f
private const val ZoomEpsilon = 0.0001f
private const val RenderZoomDebounceMs = 120L
private const val RenderZoomMinCommitIntervalMs = 160L
private const val RenderZoomMaxCommitDelayMs = 300L
private const val RenderZoomScaleRatioThreshold = 1.18f
private const val DiscreteZoomStep = 0.1f

@Stable
internal class EditorZoomController(
  private val scope: CoroutineScope? = null,
  private val renderZoomDebounceMs: Long = RenderZoomDebounceMs,
) {
  var displayZoom by mutableFloatStateOf(1f)
    private set

  var renderZoom by mutableFloatStateOf(1f)
    private set

  val isZoomEnabled: Boolean
    get() = initializedLayoutKey != null

  val indicatorZoom: Float
    get() =
      if (isZoomEnabled) {
        clampDocumentZoom(displayZoom, computeDocumentZoomBounds(currentLayoutSpec))
      } else {
        1f
      }

  private var initializedLayoutKey: String? = null
  private var currentLayoutSpec: EditorDocumentLayoutSpec = EditorDocumentLayoutSpec.Continuous(0f)
  private var currentViewportWidth: Float = 0f
  private var renderZoomQuietJob: Job? = null
  private var renderZoomMaxDelayJob: Job? = null
  private var renderZoomCooldownJob: Job? = null
  private var renderZoomCommitPending = false

  fun syncLayout(layoutSpec: EditorDocumentLayoutSpec, viewportWidth: Float) {
    currentLayoutSpec = layoutSpec
    currentViewportWidth = viewportWidth

    if (viewportWidth <= 0f) return
    val width = layoutSpec.documentZoomWidth()
    val key =
      when (layoutSpec) {
        is EditorDocumentLayoutSpec.Continuous -> "continuous:$width"
        is EditorDocumentLayoutSpec.Paginated -> "paginated:$width"
      }
    if (initializedLayoutKey != key) {
      initializedLayoutKey = key
      setZoomInternal(
        zoom = computeInitialDocumentZoom(layoutSpec, viewportWidth),
        layoutSpec = layoutSpec,
        viewportWidth = viewportWidth,
        commitRender = true,
      )
      return
    }

    setResolvedDisplayZoom(
      displayZoom = clampDocumentZoom(displayZoom, computeDocumentZoomBounds(layoutSpec)),
      commitRender = true,
    )
  }

  fun setDisplayZoom(
    zoom: Float,
    layoutSpec: EditorDocumentLayoutSpec,
    viewportWidth: Float,
    snapToLandmarks: Boolean = true,
  ): Boolean {
    return setZoomInternal(
      zoom = zoom,
      layoutSpec = layoutSpec,
      viewportWidth = viewportWidth,
      commitRender = false,
      snapToLandmarks = snapToLandmarks,
    )
  }

  fun setGestureDisplayZoom(
    rawZoom: Float,
    snapZoom: Float? = null,
    layoutSpec: EditorDocumentLayoutSpec,
    viewportWidth: Float,
  ): Boolean {
    currentLayoutSpec = layoutSpec
    currentViewportWidth = viewportWidth
    val displayZoom =
      snapZoom?.takeIf { it.isFinite() && it > 0f }
        ?: elasticEditorDisplayZoom(rawZoom, computeDocumentZoomBounds(layoutSpec))
        ?: return false
    return setResolvedDisplayZoom(displayZoom = displayZoom, commitRender = false)
  }

  fun setMotionDisplayZoom(
    displayZoom: Float,
    layoutSpec: EditorDocumentLayoutSpec,
    viewportWidth: Float,
  ): Boolean {
    if (!displayZoom.isFinite() || displayZoom <= 0f) return false
    currentLayoutSpec = layoutSpec
    currentViewportWidth = viewportWidth
    return setResolvedDisplayZoom(displayZoom = displayZoom, commitRender = false)
  }

  fun commitRenderZoom() {
    requestRenderZoomCommit()
  }

  fun resolveSnapKey(zoom: Float = displayZoom): EditorZoomSnapKey? {
    if (!isZoomEnabled) return null
    val layout = currentLayoutSpec
    val viewportWidth = resolveViewportWidthFallback(layout.documentZoomWidth())
    val fitWidthZoom =
      computeDocumentFitWidthZoom(layoutSpec = layout, viewportWidth = viewportWidth)
    val unitZoom = clampDocumentZoom(zoom = 1f, bounds = computeDocumentZoomBounds(layout))

    return when {
      zoomEquals(zoom, fitWidthZoom) -> EditorZoomSnapKey.FitWidth
      zoomEquals(zoom, unitZoom) -> EditorZoomSnapKey.Unit
      else -> null
    }
  }

  fun resolveLandmark(zoom: Float = indicatorZoom): EditorZoomLandmark? {
    if (!isZoomEnabled) return null
    return resolveEditorZoomLandmark(
      zoom = zoom,
      layoutSpec = currentLayoutSpec,
      viewportWidth = currentViewportWidth,
    )
  }

  fun resolveIndicatorToggleTarget(): Float? {
    if (!isZoomEnabled) return null
    val bounds = computeDocumentZoomBounds(currentLayoutSpec)
    val unitZoom = clampDocumentZoom(zoom = 1f, bounds = bounds)
    val fitWidthZoom =
      computeDocumentFitWidthZoom(
        layoutSpec = currentLayoutSpec,
        viewportWidth = currentViewportWidth,
      )
    val targetZoom = if (resolveLandmark() == EditorZoomLandmark.Unit) fitWidthZoom else unitZoom
    return targetZoom.takeUnless { zoomEquals(it, displayZoom) }
  }

  fun resolveZoomInTarget(): Float? = resolveStepTarget(1)

  fun resolveZoomOutTarget(): Float? = resolveStepTarget(-1)

  private fun resolveStepTarget(direction: Int): Float? {
    if (!isZoomEnabled) return null
    return resolveEditorZoomStepTarget(
      zoom = indicatorZoom,
      direction = direction,
      layoutSpec = currentLayoutSpec,
      viewportWidth = currentViewportWidth,
    )
  }

  private fun resolveDisplayZoom(
    zoom: Float,
    layoutSpec: EditorDocumentLayoutSpec,
    viewportWidth: Float,
    snapToLandmarks: Boolean,
  ): Float =
    if (snapToLandmarks) {
      clampDocumentLayoutZoom(zoom, layoutSpec, viewportWidth)
    } else {
      clampDocumentZoom(zoom, computeDocumentZoomBounds(layoutSpec))
    }

  private fun setZoomInternal(
    zoom: Float,
    layoutSpec: EditorDocumentLayoutSpec,
    viewportWidth: Float,
    commitRender: Boolean,
    snapToLandmarks: Boolean = true,
  ): Boolean {
    currentLayoutSpec = layoutSpec
    currentViewportWidth = viewportWidth

    val resolvedZoom =
      resolveDisplayZoom(
        zoom = zoom,
        layoutSpec = layoutSpec,
        viewportWidth = viewportWidth,
        snapToLandmarks = snapToLandmarks,
      )
    return setResolvedDisplayZoom(displayZoom = resolvedZoom, commitRender = commitRender)
  }

  private fun setResolvedDisplayZoom(displayZoom: Float, commitRender: Boolean): Boolean {
    val changed = zoomDiffers(this.displayZoom, displayZoom)
    if (changed) {
      this.displayZoom = displayZoom
    }

    if (commitRender) {
      clearRenderZoomSchedule()
      syncRenderZoomNow()
      return changed
    }

    scheduleRenderZoom()
    return changed
  }

  private fun scheduleRenderZoom() {
    renderZoomQuietJob?.cancel()
    renderZoomQuietJob = null
    renderZoomCommitPending = false

    val nextRenderZoom = renderZoomForDisplay(displayZoom)
    if (!zoomDiffers(renderZoom, nextRenderZoom)) {
      clearRenderZoomSchedule()
      return
    }

    if (renderZoomMaxDelayJob?.isActive != true) {
      renderZoomMaxDelayJob = scope?.launch {
        delay(RenderZoomMaxCommitDelayMs)
        renderZoomMaxDelayJob = null
        requestRenderZoomCommit()
      }
    }

    val scaleRatio = maxOf(displayZoom / renderZoom, renderZoom / displayZoom)
    if (scaleRatio >= RenderZoomScaleRatioThreshold) {
      requestRenderZoomCommit()
      return
    }

    renderZoomQuietJob = scope?.launch {
      delay(renderZoomDebounceMs)
      renderZoomQuietJob = null
      requestRenderZoomCommit()
    }
  }

  private fun requestRenderZoomCommit() {
    renderZoomQuietJob?.cancel()
    renderZoomQuietJob = null

    val nextRenderZoom = renderZoomForDisplay(displayZoom)
    if (!zoomDiffers(renderZoom, nextRenderZoom)) {
      clearRenderZoomSchedule()
      return
    }

    if (renderZoomCooldownJob?.isActive == true) {
      renderZoomCommitPending = true
      return
    }

    clearRenderZoomSchedule()
    syncRenderZoomNow()
  }

  private fun clearRenderZoomSchedule() {
    renderZoomQuietJob?.cancel()
    renderZoomQuietJob = null
    renderZoomMaxDelayJob?.cancel()
    renderZoomMaxDelayJob = null
    renderZoomCommitPending = false
  }

  private fun syncRenderZoomNow() {
    val nextRenderZoom = renderZoomForDisplay(displayZoom)
    if (!zoomEquals(renderZoom, nextRenderZoom)) {
      renderZoom = nextRenderZoom
      renderZoomCooldownJob?.cancel()
      renderZoomCooldownJob = scope?.launch {
        delay(RenderZoomMinCommitIntervalMs)
        renderZoomCooldownJob = null
        if (renderZoomCommitPending) {
          renderZoomCommitPending = false
          requestRenderZoomCommit()
        }
      }
    }
  }

  private fun resolveViewportWidthFallback(pageWidth: Float): Float {
    return if (currentViewportWidth.isFinite() && currentViewportWidth > 0f) {
      currentViewportWidth
    } else if (pageWidth.isFinite() && pageWidth > 0f) {
      pageWidth
    } else {
      1f
    }
  }
}

internal enum class EditorZoomSnapKey {
  FitWidth,
  Unit,
}

internal enum class EditorZoomLandmark {
  Minimum,
  FitWidth,
  Unit,
  Maximum,
}

internal fun resolveEditorZoomLandmark(
  zoom: Float,
  layoutSpec: EditorDocumentLayoutSpec,
  viewportWidth: Float,
): EditorZoomLandmark? {
  val layoutWidth =
    when (layoutSpec) {
      is EditorDocumentLayoutSpec.Continuous -> layoutSpec.maxWidth
      is EditorDocumentLayoutSpec.Paginated -> layoutSpec.pageWidth
    }
  if (!zoom.isFinite() || zoom <= 0f || !layoutWidth.isFinite() || layoutWidth <= 0f) return null
  if (!viewportWidth.isFinite() || viewportWidth <= 0f) return null

  val bounds = computeDocumentZoomBounds(layoutSpec)
  val unitZoom = clampDocumentZoom(zoom = 1f, bounds = bounds)
  if (zoomEquals(zoom, unitZoom)) return EditorZoomLandmark.Unit

  val naturalFitWidthZoom = viewportWidth / layoutSpec.documentZoomWidth()
  if (naturalFitWidthZoom in bounds && zoomEquals(zoom, naturalFitWidthZoom)) {
    return EditorZoomLandmark.FitWidth
  }
  if (zoomEquals(zoom, bounds.start)) return EditorZoomLandmark.Minimum
  if (zoomEquals(zoom, bounds.endInclusive)) return EditorZoomLandmark.Maximum
  return null
}

internal fun computeDocumentZoomBounds(
  layoutSpec: EditorDocumentLayoutSpec
): ClosedFloatingPointRange<Float> {
  val minZoom = (MinDocumentDisplayWidth / layoutSpec.documentZoomWidth()).coerceAtLeast(0.01f)
  val maxZoom = MaxDocumentZoom.coerceAtLeast(minZoom)
  return minZoom..maxZoom
}

internal fun clampDocumentZoom(zoom: Float, bounds: ClosedFloatingPointRange<Float>): Float {
  if (!zoom.isFinite()) {
    return bounds.start
  }

  return zoom.coerceIn(bounds.start, bounds.endInclusive)
}

internal fun computeDocumentFitWidthZoom(
  layoutSpec: EditorDocumentLayoutSpec,
  viewportWidth: Float,
): Float {
  val bounds = computeDocumentZoomBounds(layoutSpec)
  val width = layoutSpec.documentZoomWidth()
  val safeViewportWidth =
    if (viewportWidth.isFinite() && viewportWidth > 0f) {
      viewportWidth
    } else {
      width
    }
  return (safeViewportWidth / width).coerceIn(bounds.start, bounds.endInclusive)
}

internal fun computeInitialDocumentZoom(
  layoutSpec: EditorDocumentLayoutSpec,
  viewportWidth: Float,
): Float =
  when (layoutSpec) {
    is EditorDocumentLayoutSpec.Continuous ->
      clampDocumentZoom(1f, computeDocumentZoomBounds(layoutSpec))
    is EditorDocumentLayoutSpec.Paginated ->
      computeDocumentFitWidthZoom(layoutSpec, viewportWidth).coerceAtMost(1f)
  }

internal fun clampDocumentLayoutZoom(
  zoom: Float,
  layoutSpec: EditorDocumentLayoutSpec,
  viewportWidth: Float,
): Float {
  val bounds = computeDocumentZoomBounds(layoutSpec)
  val clamped = clampDocumentZoom(zoom = zoom, bounds = bounds)
  val fitWidthZoom =
    computeDocumentFitWidthZoom(layoutSpec = layoutSpec, viewportWidth = viewportWidth)
  val unitZoom = clampDocumentZoom(zoom = 1f, bounds = bounds)

  var snapped: Float? = null
  var bestDistance = Float.POSITIVE_INFINITY

  val fitWidthDistance = abs(clamped - fitWidthZoom)
  if (fitWidthDistance <= FitWidthZoomSnapThreshold) {
    snapped = fitWidthZoom
    bestDistance = fitWidthDistance
  }

  val unitDistance = abs(clamped - unitZoom)
  if (unitDistance <= UnitZoomSnapThreshold && unitDistance < bestDistance) {
    snapped = unitZoom
  }

  return snapped ?: clamped
}

private fun resolveEditorZoomStepTarget(
  zoom: Float,
  direction: Int,
  layoutSpec: EditorDocumentLayoutSpec,
  viewportWidth: Float,
): Float? {
  val bounds = computeDocumentZoomBounds(layoutSpec)
  val current = clampDocumentZoom(zoom, bounds)
  val candidates =
    mutableListOf(
      bounds.start,
      computeDocumentFitWidthZoom(layoutSpec, viewportWidth),
      clampDocumentZoom(1f, bounds),
      bounds.endInclusive,
    )
  val firstGridIndex = ceil((bounds.start - ZoomEpsilon) / DiscreteZoomStep).toInt()
  val lastGridIndex = floor((bounds.endInclusive + ZoomEpsilon) / DiscreteZoomStep).toInt()
  for (index in firstGridIndex..lastGridIndex) {
    candidates += clampDocumentZoom(index * DiscreteZoomStep, bounds)
  }

  return if (direction > 0) {
    candidates.filter { it > current + ZoomEpsilon }.minOrNull()
  } else {
    candidates.filter { it < current - ZoomEpsilon }.maxOrNull()
  }
}

internal fun renderZoomForDisplay(displayZoom: Float): Float {
  if (!displayZoom.isFinite()) {
    return 1f
  }

  return if (displayZoom <= 0f) 0.01f else displayZoom
}

internal fun zoomEquals(a: Float, b: Float): Boolean = abs(a - b) < ZoomEpsilon

internal fun zoomDiffers(a: Float, b: Float): Boolean = !zoomEquals(a, b)

@Composable
internal fun rememberEditorZoomController(key: Any): EditorZoomController {
  val scope = rememberCoroutineScope()
  return remember(key) { EditorZoomController(scope = scope) }
}

internal val LocalEditorZoomController =
  compositionLocalOf<EditorZoomController> { error("No EditorZoomController provided") }
